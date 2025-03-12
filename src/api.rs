use crate::{
    prompt,
    source::{self, Source},
    template,
};

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum KopyeError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Source(#[from] source::SourceError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Template(#[from] template::TemplateError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Prompt(#[from] prompt::PromptError),
}

/// Copies a template from the specified source directory to the provided destination path.
///
/// # Errors
///
/// Returns a [`KopyeError`] if:
///
/// - The configuration could not be built from the `source`.
/// - The template or its files cannot be located or read.
/// - A directory or file cannot be created or written to.
/// - Tera fails to initialize or render a template.
pub fn copy_template(src: &str, template: &str, destination: &str) -> Result<(), KopyeError> {
    let source = Source::build_from(src)?;

    log::debug!(
        "Attempting to build source from: {}",
        source.source_dir.display()
    );

    template::try_render(source, template, destination)?;

    Ok(())
}

/// Interactively lists and selects a template from the specified source directory, then copies it
/// to a user-provided destination path.
///
/// This function also builds a [`Source`] from the given `source`, then prompts the user to
/// select a template and a destination directory.  files.
///
/// # Errors
///
/// Returns a [`KopyeError`] if:
///
/// - The configuration could not be built from the `source`.
/// - User prompts fail or the user cancels the input.
/// - The template or its files cannot be located or read.
/// - A directory or file cannot be created or written to.
/// - Tera fails to initialize or render a template.
pub fn list_templates(src: &str) -> Result<(), KopyeError> {
    let source = Source::build_from(src)?;

    let template = prompt::get_project(source.clone())?;

    let destination = prompt::get_destination()?;

    template::try_render(source, &template, &destination)?;

    Ok(())
}
