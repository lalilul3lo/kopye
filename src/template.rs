use crate::{
    errors::{FileOperation, IoError},
    prompt::{get_answers, Answer, PromptError},
    source::Source,
    transactions::{Active, Committed, RollbackOperation, Transaction},
    utils::normalize_path,
};
use colored::Colorize;
use indexmap::IndexMap;
use miette::Diagnostic;
use tera::{Context, Tera};
use thiserror::Error;

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

/// Renders the specified template from the given [`Source`] into the specified `destination` directory.
///
/// # Process Overview
/// 1. Looks up the `template` in the [`Source`]'s `projects` map.
/// 2. Builds a Tera pattern for all `*.tera` files within the template directory.
/// 3. Iterates over those files, rendering each one with a prompt-driven [`tera::Context`].
/// 4. Writes out the rendered files into `destination`, creating directories as needed.
/// 5. Tracks operations using a [`Transaction`] for potential rollback.
///
/// # Errors
/// This function returns a [`TemplateError`] if:
/// - The `template` is missing from the config.
/// - The template directory path contains invalid UTF-8.
/// - Tera fails to initialize or render a file.
/// - I/O operations fail when reading or writing files.
/// - No matching `.tera` files are found.
pub fn try_render(
    config: Source,
    template: &str,
    destination: &str,
) -> Result<Transaction<Committed>, TemplateError> {
    let path_to_blueprint = &config
        .projects
        .get(template)
        .ok_or_else(|| TemplateError::ProjectNotFound {
            name: template.to_string(),
        })?
        .path;

    let blueprint_directory = config.source_dir.join(normalize_path(path_to_blueprint));

    let answers = get_answers(&blueprint_directory)?;

    let mut base_ctx = Context::new();

    let hydrated_ctx = hydrate_tera_ctx(&mut base_ctx, answers);

    let blueprint_directory_str =
        blueprint_directory
            .to_str()
            .ok_or_else(|| TemplateError::InvalidProjectStringUnicode {
                path: blueprint_directory.clone(),
            })?;

    log::debug!("blueprint directory string: {}", blueprint_directory_str);

    let pattern = format!("{}/**/*.{}", blueprint_directory_str, TERA_FILE_EXTENSION);

    log::debug!("tera pattern: {}", pattern);

    let mut tera =
        Tera::new(&pattern).map_err(|err| TemplateError::TeraInstanceInitialization {
            pattern: pattern.clone(),
            source: err,
        })?;

    let destination_path = std::path::PathBuf::from(destination);

    log::debug!("destination: {}", destination_path.display());

    let mut trx = Transaction::new();

    for entry in walkdir::WalkDir::new(&blueprint_directory) {
        match entry {
            Ok(entry) => {
                let path = entry.path();

                log::debug!("path: {}", path.display());

                let file_name = path
                    .file_name()
                    .ok_or_else(|| TemplateError::GenerateFileName {
                        path: path.to_path_buf(),
                    })?
                    .to_string_lossy();

                if path.is_file() && file_name != "blueprint.toml" {
                    let relative_path = path.strip_prefix(&blueprint_directory).map_err(|err| {
                        TemplateError::StripPrefix {
                            path: path.into(),
                            dir: blueprint_directory.clone(),
                            source: err,
                        }
                    })?;
                    let parent = relative_path
                        .parent()
                        .unwrap_or_else(|| std::path::Path::new(""));

                    let out_file_name = if path
                        .extension()
                        .is_some_and(|ext| ext == TERA_FILE_EXTENSION)
                    {
                        file_name.replace(&format!(".{}", TERA_FILE_EXTENSION), "")
                    } else {
                        file_name.to_string()
                    };

                    let file_destination_path = destination_path.join(parent).join(out_file_name);

                    let content = std::fs::read_to_string(path).map_err(|err| {
                        IoError::new(FileOperation::Read, path.to_path_buf(), err)
                    })?;

                    let rendered = tera.render_str(&content, hydrated_ctx).map_err(|err| {
                        TemplateError::Render {
                            context: hydrated_ctx.clone(),
                            source: err,
                        }
                    })?;

                    if let Some(parent_dir) = file_destination_path.parent() {
                        let copy = parent_dir.to_owned();

                        log::debug!("attempting to create directory: {}", copy.display());

                        // PERF: might be creating multiple dirs
                        create_directory(&mut trx, copy.as_path())?;
                    }

                    log::debug!(
                        "attempting to write file: {}",
                        file_destination_path.display()
                    );

                    write_file(&mut trx, file_destination_path.as_path(), rendered)?;
                }
            }
            Err(err) => {
                let path = err.path().unwrap_or(std::path::Path::new(""));

                Err(IoError::new(
                    FileOperation::Read,
                    path.to_path_buf(),
                    err.into(),
                ))?
            }
        }
    }

    Ok(trx.commit())
}

fn hydrate_tera_ctx(context: &mut Context, answers: IndexMap<String, Answer>) -> &mut Context {
    for (key, answer) in answers {
        match answer {
            Answer::String(s) => {
                context.insert(&key, &s);
            }
            // Answer::Int(i) => {
            //     context.insert(&key, &i);
            // }
            // Answer::Float(f) => {
            //     context.insert(&key, &f);
            // }
            Answer::Bool(b) => {
                context.insert(&key, &b);
            }
            Answer::Array(arr) => {
                context.insert(&key, &arr);
            }
        }
    }

    context
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
