use crate::{
    errors::{FileOperation, IoError},
    preview::preview_as_tree,
    prompt::{apply_changes, get_answers, Answer, PromptError},
    source::Source,
    transactions::{Active, FinalTransactionState, RollbackOperation, Transaction},
    utils::normalize_path,
    vfs::{VirtualEntry, VirtualFS},
};
use colored::Colorize;
use indexmap::IndexMap;
use miette::Diagnostic;
use std::path::{Path, PathBuf};
use tera::{Context, Tera};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Error, Diagnostic)]
pub enum TemplateError {
    #[error("I/O error within template domain")]
    #[diagnostic(code(kopye::template::io))]
    Io(#[from] IoError),

    #[error("Project not found with name: {name}")]
    #[diagnostic(
        code(kopye::template::project_not_found),
        help("Make sure project is available -> point to documentation about creating projects")
    )]
    ProjectNotFound { name: String },

    #[error("Error occurred trying to prompt user")]
    #[diagnostic(code(kopye::template::prompt))]
    Prompt(#[from] PromptError),

    #[error("Error occurred trying to convert blueprint directory to string")]
    #[diagnostic(
        code(kopye::template::invalid_project_string_unicode),
        help("Please check the path")
    )]
    InvalidProjectStringUnicode { path: std::path::PathBuf },

    #[error("Error occurred attempting to initialize tera instance")]
    #[diagnostic(code(kopye::template::tera_instance_initialization))]
    TeraInstanceInitialization {
        pattern: String,
        #[source]
        source: tera::Error,
    },

    #[error("Error occurred attempting to generate out file name")]
    #[diagnostic(code(kopye::template::generate_filename))]
    GenerateFileName { path: std::path::PathBuf },

    #[error("Error occurrend attempting to render template")]
    #[diagnostic(code(kopye::template::render))]
    Render {
        context: Context,
        #[source]
        source: tera::Error,
    },

    #[error("unable to strip prefix from directory")]
    #[diagnostic(code(kopye::template::strip_prefix))]
    StripPrefix {
        path: std::path::PathBuf,
        dir: std::path::PathBuf,
        source: std::path::StripPrefixError,
    },
}

const TERA_FILE_EXTENSION: &str = "tera";

/// Loops over path segments/components and renders them as tera templates and returns `Some(PathBuf)`
/// It returns `None` if ANY segment is empty (I.E parent directory is conditionally rendered).
///
/// For example, if your path segments are:
///   `["{% if integrations_tests %}tests{% endif %}", "{% if mocks %}mocks{% endif %}", "{{project}}.rs"]`
/// and `integrations_tests=false`, the first segment becomes `""`, so this returns `None`.
fn render_path_segments(
    path: &Path,
    tera: &mut Tera,
    ctx: &Context,
) -> Result<Option<PathBuf>, TemplateError> {
    let mut result = PathBuf::new();

    for component in path.components() {
        let segment_str = component.as_os_str().to_string_lossy();

        let rendered =
            tera.render_str(&segment_str, ctx)
                .map_err(|error| TemplateError::Render {
                    context: ctx.clone(),
                    source: error,
                })?;

        if rendered.trim().is_empty() {
            return Ok(None);
        }

        result.push(rendered.trim());
    }

    Ok(Some(result))
}
/// Recursively walks the `blueprint_directory`, renders each path segment as a tera template
/// and builds up a [`VirtualFS`] of all directories and files that should be created.
fn build_vfs(
    source_directory: &Path,
    tera: &mut Tera,
    ctx: &Context,
) -> Result<VirtualFS, TemplateError> {
    let mut vfs = VirtualFS::new();

    for entry in WalkDir::new(source_directory) {
        let entry = match entry {
            Ok(e) => e,
            Err(error) => {
                let path = error.path().unwrap_or_else(|| Path::new(""));

                Err(IoError::new(
                    FileOperation::Read,
                    path.to_path_buf(),
                    error.into(),
                ))?
            }
        };

        // skip blueprint config file
        let file_name = entry.file_name().to_string_lossy();
        if file_name == "blueprint.toml" {
            continue;
        }

        let full_path = entry.path();
        let relative = match full_path.strip_prefix(source_directory) {
            Ok(r) => r,
            Err(error) => Err(TemplateError::StripPrefix {
                path: full_path.to_path_buf(),
                dir: source_directory.to_path_buf(),
                source: error,
            })?,
        };

        // render the relative path segments/components as tera templates
        let rendered_rel_path = render_path_segments(relative, tera, ctx)?;

        // If `None`, at least one segment rendered to empty, therefore skip
        let Some(rendered_path) = rendered_rel_path else {
            // Skip this file or directory and it's children
            continue;
        };

        if entry.file_type().is_dir() {
            vfs.entries.push(VirtualEntry {
                destination: Some(rendered_path),
                content: None,
                is_file: false,
            });
        } else {
            let mut file_contents = std::fs::read_to_string(full_path).map_err(|error| {
                IoError::new(FileOperation::Read, full_path.to_path_buf(), error)
            })?;

            let mut final_dest = rendered_path.clone();

            let is_tera = rendered_path
                .extension()
                .map(|ext| ext == TERA_FILE_EXTENSION)
                .unwrap_or(false);

            // remove file extension and render file content if .tera extension detected
            if is_tera {
                let file_stem = final_dest.file_stem().unwrap_or_default().to_owned();
                final_dest.set_file_name(file_stem);

                let rendered = tera.render_str(&file_contents, ctx).map_err(|error| {
                    TemplateError::Render {
                        context: ctx.clone(),
                        source: error,
                    }
                })?;

                file_contents = rendered;
            }

            vfs.entries.push(VirtualEntry {
                destination: Some(final_dest),
                content: Some(file_contents),
                is_file: true,
            });
        }
    }

    Ok(vfs)
}
/// Applies directory and file creation operations from a [`VirtualFS`].
fn apply_vfs(
    vfs: &VirtualFS,
    destination_root: &Path,
    trx: &mut Transaction<Active>,
) -> Result<(), TemplateError> {
    // First create all directories
    for entry in vfs.entries.iter().filter(|e| !e.is_file) {
        let Some(rel_dest) = &entry.destination else {
            continue;
        };
        let final_path = destination_root.join(rel_dest);

        create_directory(trx, &final_path)?;
    }

    // Then create all files
    for entry in vfs.entries.iter().filter(|e| e.is_file) {
        let Some(rel_dest) = &entry.destination else {
            continue;
        };
        let final_path = destination_root.join(rel_dest);
        // create parent if necessary
        let parent = final_path.parent();
        if let Some(parent) = parent {
            create_directory(trx, parent)?;
        }

        let contents = entry.content.clone().unwrap_or_default();

        write_file(trx, &final_path, contents)?;
    }

    Ok(())
}
/// Makes a [`Tera`] [`Context`] object, hydrated with user prompt answers.
fn make_tera_context(answers: IndexMap<String, Answer>) -> Context {
    let mut base_ctx = Context::new();
    for (key, answer) in answers {
        match answer {
            Answer::String(ans) => base_ctx.insert(&key, &ans),
            Answer::Bool(ans) => base_ctx.insert(&key, &ans),
            Answer::Array(ans) => base_ctx.insert(&key, &ans),
        }
    }

    base_ctx.clone()
}
/// Renders the specified template from the given [`Source`] into `destination`,
pub fn try_render(
    config: Source,
    template: &str,
    destination: &str,
) -> Result<FinalTransactionState, TemplateError> {
    let path_to_blueprint = config
        .projects
        .get(template)
        .ok_or_else(|| TemplateError::ProjectNotFound {
            name: template.to_string(),
        })?
        .path
        .clone();

    let blueprint_directory = config.source_dir.join(normalize_path(&path_to_blueprint));

    let answers = get_answers(&blueprint_directory)?;

    let tera_context = make_tera_context(answers);

    let pattern = format!("{}/**/*.tera", blueprint_directory.display());

    let mut tera = Tera::new(&pattern)
        .map_err(|e| TemplateError::TeraInstanceInitialization { pattern, source: e })?;

    let vfs = build_vfs(&blueprint_directory, &mut tera, &tera_context)?;

    let destination_path = std::path::PathBuf::from(destination);

    preview_as_tree(&vfs, &destination_path);

    let mut trx = Transaction::<Active>::new();

    if apply_changes()? {
        apply_vfs(&vfs, &destination_path, &mut trx)?;

        Ok(FinalTransactionState::Committed(trx.commit()))
    } else {
        Ok(FinalTransactionState::Canceled(trx.cancel()))
    }
}
/// Creates all directories in the specified path if they do not exist.
///
/// This function uses [`std::fs::create_dir_all`] to ensure the entire directory path
/// is created. It then registers a [`RollbackOperation::RemoveDir`] on the provided
/// [`Transaction`] to support undoing the creation if needed.
///
/// # Errors
///
/// Returns a [`KopyeError`] if any directory creation fails due to I/O issues.
fn create_directory(
    trx: &mut Transaction<Active>,
    path: &std::path::Path,
) -> Result<(), TemplateError> {
    std::fs::create_dir_all(path)
        .map_err(|error| IoError::new(FileOperation::Mkdir, path.into(), error))?;

    trx.add_operation(RollbackOperation::RemoveDir(path.to_path_buf()));

    Ok(())
}
/// Writes a file with the provided contents to the specified path.
///
/// After the file is created or overwritten, a [`RollbackOperation::RemoveFile`] operation
/// is registered in the [`Transaction`] for potential cleanup. Additionally, this
/// function prints a message to the console indicating that the file has been created.
///
/// # Errors
///
/// Returns a [`KopyeError`] if writing to the file fails due to I/O issues.
fn write_file(
    trx: &mut Transaction<Active>,
    path: &std::path::Path,
    contents: String,
) -> Result<(), TemplateError> {
    std::fs::write(path, contents.clone())
        .map_err(|error| IoError::new(FileOperation::Write, path.into(), error))?;

    let msg = format!("{} {}", "create".green(), path.display());

    println!("{}", &msg);

    trx.add_operation(RollbackOperation::RemoveFile(path.to_path_buf()));

    Ok(())
}
