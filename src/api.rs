use crate::{
    config::Config,
    prompt::{get_answers, get_project},
    transactions::{RollbackOperation, Transaction},
    utils::{get_source_directory, hydrate_tera_ctx, normalize_path},
};
use colored::Colorize;
use tera::{Context, Tera};

fn create_directory(trx: &mut Transaction, path: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path).unwrap();

    trx.add_operation(RollbackOperation::RemoveDir(path.to_path_buf()));

    Ok(())
}

fn write_file(
    trx: &mut Transaction,
    path: &std::path::Path,
    contents: String,
) -> std::io::Result<()> {
    std::fs::write(path, contents).unwrap();

    let msg = format!("{} {}", "create".green(), path.display());

    println!("{}", &msg);

    trx.add_operation(RollbackOperation::RemoveFile(path.to_path_buf()));

    Ok(())
}

const TERA_FILE_EXTENSION: &str = "tera";

pub fn copy_template(source: &str, template: &str, destination: &str) {
    let source_directory = get_source_directory(source);

    let config = source_directory.join("kopye.toml");

    let parsed_config = Config::from_file(config).unwrap(); // ERROR: config

    let path_to_blueprint = &parsed_config.0.get(template).unwrap().path; // ERROR: Option

    let blueprint_directory = source_directory.join(normalize_path(path_to_blueprint));

    let answers = get_answers(&blueprint_directory);

    let mut base_ctx = Context::new();

    let hydrated_ctx = hydrate_tera_ctx(&mut base_ctx, answers);

    let blueprint_directory_str = blueprint_directory.to_str().unwrap(); // ERROR: Option

    let pattern = format!("{}/**/*.{}", blueprint_directory_str, TERA_FILE_EXTENSION);

    let mut tera = Tera::new(&pattern).unwrap(); // ERROR: Tera

    let destination_path = std::path::PathBuf::from(destination);

    let mut trx = Transaction::new();

    for entry in walkdir::WalkDir::new(&blueprint_directory) {
        let entry = entry.unwrap(); // ERROR: Dir
        let path = entry.path();

        if path.is_file()
            && path
                .extension()
                .map(|ext| ext == TERA_FILE_EXTENSION)
                .unwrap_or(false)
        {
            let relative_path = path.strip_prefix(&blueprint_directory).unwrap(); // ERROR(FileOperation)
            let parent = relative_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new(""));
            let out_file_name = path
                .file_name()
                .unwrap() //
                .to_string_lossy()
                .replace(&format!(".{}", TERA_FILE_EXTENSION), "");

            let file_destination_path = destination_path.join(parent).join(out_file_name);

            let content = std::fs::read_to_string(path).unwrap(); // ERROR: FileOperation

            let rendered = tera.render_str(&content, hydrated_ctx).unwrap(); // ERROR: Tera

            if let Some(parent_dir) = file_destination_path.parent() {
                let copy = parent_dir.to_owned();

                // ERROR: FileOperation
                create_directory(&mut trx, copy.as_path()).unwrap(); // NOTE: multiples dirs?
            }

            // ERROR: FileOperation
            write_file(&mut trx, file_destination_path.as_path(), rendered).unwrap();
        }
    }
}

pub fn list_templates(source: &str) {
    let source_directory = get_source_directory(source);

    let config = source_directory.join("kopye.toml");

    let parsed_config = Config::from_file(config).unwrap(); // ERROR: config

    get_project(parsed_config);
}
