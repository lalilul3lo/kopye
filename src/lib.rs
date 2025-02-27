use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::{
    fmt, fs,
    path::{Component, Path, PathBuf},
};

// TODO(Args): implement newtype for repo
// TODO(Args): implement newtype for destination
const TERA_FILE_EXTENSION: &str = "tera";

#[derive(Debug, Deserialize)]
pub enum QuestionType {
    Text,
    Confirm,
    Select,
    MultiSelect,
}
#[derive(Debug, Deserialize)]
pub struct Question {
    pub r#type: QuestionType,
    pub help: String,
    pub choices: Option<Vec<String>>,
    pub multiselect: Option<bool>,
}
#[derive(Debug, Deserialize)]
pub struct QuestionsFile(pub IndexMap<String, Question>);
impl QuestionsFile {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let parsed: QuestionsFile = toml::from_str(&content)?;
        Ok(parsed)
    }
}

#[derive(Debug, Deserialize)]
pub struct BlueprintFIle(pub IndexMap<String, String>);

#[derive(Debug, Serialize)]
pub enum Answer {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<String>),
}

#[derive(Debug, PartialEq)]
pub enum Source {
    Git,
    Local,
}
impl Source {
    fn as_str(&self) -> &str {
        match self {
            Self::Git => "git",
            Self::Local => "local",
        }
    }
}
impl From<&str> for Source {
    fn from(value: &str) -> Self {
        lazy_static::lazy_static! {
            static ref GIT_URL_REGEX: regex::Regex = regex::Regex::new(
                r"(?x)        # Enable extended mode
            ^(?:
                # 1) gh:account/repo
                gh:[^/]+/[^/]+
                |
                # 2) gl:account/repo
                gl:[^/]+/[^/]+
                |
                # 3) git@host:account/repo.git
                git@[A-Za-z0-9._-]+:[^/]+/[^/]+\.git
                |
                # 4) git+http(s)://...
                git\+https?://.*
            )$"
            ).unwrap();
        }
        if GIT_URL_REGEX.is_match(value) {
            Self::Git
        } else {
            Self::Local
        }
    }
}
impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Git => {
                write!(f, "{}", self.as_str())
            }
            Self::Local => {
                write!(f, "{}", self.as_str())
            }
        }
    }
}
pub fn expand_git_short_url(url: &str) -> String {
    if let Some(stripped) = url.strip_prefix("gh:") {
        format!("https://github.com/{}.git", stripped)
    } else if let Some(stripped) = url.strip_prefix("gl:") {
        format!("https://gitlab.com/{}.git", stripped)
    } else {
        url.to_string() // TODO: Handle properly
    }
}
#[derive(Debug, Deserialize)]
pub struct BlueprintInfo {
    pub path: String,
}
#[derive(Debug, Deserialize)]
pub struct Config(pub IndexMap<String, BlueprintInfo>); // https://www.howtocodeit.com/articles/ultimate-guide-rust-newtypes
impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let parsed: Config = toml::from_str(&content)?;
        Ok(parsed)
    }
}

#[derive(Debug, Deserialize)]
pub struct TemplateQuestion {
    pub r#type: String,
    pub help: String,
    pub choices: Option<Vec<String>>,
    pub multiselect: Option<bool>,
}
#[derive(Debug, Deserialize)]
pub struct KopyeTomlQuestion(pub IndexMap<String, TemplateQuestion>); // https://www.howtocodeit.com/articles/ultimate-guide-rust-newtypes

pub fn normalize_path(source: &String) -> PathBuf {
    let input = PathBuf::from(source);

    let mut new_path = PathBuf::new();

    for component in input.components() {
        match component {
            // Skip the current-dir marker "."
            Component::CurDir => {}

            // For "..", pop the last component if possible
            Component::ParentDir => {
                new_path.pop();
            }

            // For normal components, push them
            other => new_path.push(other.as_os_str()),
        }
    }

    new_path
}

mod transactions {
    use std::{fs, path::PathBuf};

    pub enum RollbackOperation {
        RemoveFile(PathBuf),
        RemoveDir(PathBuf),
    }

    pub struct Transaction {
        rollback_operations: Vec<RollbackOperation>,
    }

    impl Transaction {
        pub fn new() -> Self {
            Transaction {
                rollback_operations: vec![],
            }
        }

        pub fn add_operation(&mut self, operation: RollbackOperation) {
            self.rollback_operations.push(operation);
        }

        pub fn commit(&mut self) {
            self.rollback_operations.clear();
        }

        pub fn rollback(&mut self) {
            while let Some(operation) = self.rollback_operations.pop() {
                match operation {
                    RollbackOperation::RemoveDir(path) => {
                        let _ = fs::remove_file(&path);
                    }
                    RollbackOperation::RemoveFile(path) => {
                        let _ = fs::remove_dir_all(&path);
                    }
                }
            }
        }
    }

    impl Drop for Transaction {
        fn drop(&mut self) {
            if !self.rollback_operations.is_empty() {
                self.rollback();
            } else {
                self.commit();
            }
        }
    }
}

// Public API
pub mod actions {
    use crate::{
        expand_git_short_url, normalize_path,
        transactions::{RollbackOperation, Transaction},
        Answer, Config, QuestionType, QuestionsFile, Source, TERA_FILE_EXTENSION,
    };
    use colored::Colorize;
    use git2::Repository;
    use indexmap::IndexMap;
    use inquire::{required, validator::MinLengthValidator, Confirm, MultiSelect, Select, Text};
    use std::{
        fs, io,
        path::{Path, PathBuf},
    };
    use tera::{Context, Tera};

    fn get_answers(template_path: &Path) -> IndexMap<String, Answer> {
        let file = QuestionsFile::from_file(template_path.join("kopye.questions.toml")).unwrap();

        let mut answers: IndexMap<String, Answer> = IndexMap::new();

        for (question, config) in &file.0 {
            match config.r#type {
                QuestionType::Text => {
                    let answer = Text::new(question)
                        .with_help_message(&config.help)
                        .with_validator(required!(format!("{} is required", question)))
                        .prompt()
                        .unwrap();

                    answers.insert(question.clone(), Answer::String(answer));
                }
                QuestionType::Confirm => {
                    let answer = Confirm::new(question)
                        .with_help_message(&config.help)
                        .prompt()
                        .unwrap();

                    answers.insert(question.clone(), Answer::Bool(answer));
                }
                QuestionType::Select => {
                    let choices = config.choices.clone().unwrap();

                    let answer = Select::new(question, choices)
                        .with_help_message(&config.help)
                        .prompt()
                        .unwrap();

                    answers.insert(question.clone(), Answer::String(answer));
                }
                QuestionType::MultiSelect => {
                    let choices = config.choices.clone().unwrap();

                    let answer = MultiSelect::new(question, choices)
                        .with_help_message(&config.help)
                        .with_validator(MinLengthValidator::new(1))
                        .prompt()
                        .unwrap();

                    answers.insert(question.clone(), Answer::Array(answer));
                }
            }
        }

        answers
    }

    fn make_context(answers: IndexMap<String, Answer>) -> Context {
        let mut context = Context::new();

        for (key, answer) in answers {
            match answer {
                Answer::String(s) => {
                    context.insert(&key, &s);
                }
                Answer::Int(i) => {
                    context.insert(&key, &i);
                }
                Answer::Float(f) => {
                    context.insert(&key, &f);
                }
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

    fn create_directory(trx: &mut Transaction, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path).unwrap();

        trx.add_operation(RollbackOperation::RemoveDir(path.to_path_buf()));

        Ok(())
    }

    fn write_file(trx: &mut Transaction, path: &Path, contents: String) -> io::Result<()> {
        fs::write(path, contents).unwrap();

        let msg = format!("{} {}", "create".green(), path.display());

        println!("{}", &msg);

        trx.add_operation(RollbackOperation::RemoveFile(path.to_path_buf()));

        Ok(())
    }

    pub fn copy_template(source: &str, template: &str, destination: &str) {
        let source_directory = match Source::from(source) {
            Source::Git => {
                let directory = tempfile::tempdir().unwrap().into_path();

                let expanded_url = expand_git_short_url(source);

                Repository::clone(&expanded_url, directory.as_path()).unwrap();

                directory
            }
            Source::Local => PathBuf::from(source),
        };

        let config = source_directory.join("kopye.toml");

        let parsed_config = Config::from_file(config).unwrap();

        let path_to_blueprint = &parsed_config.0.get(template).unwrap().path;

        let blueprint_directory = source_directory.join(normalize_path(path_to_blueprint));

        let blueprint_directory_str = blueprint_directory.to_str().unwrap();

        let answers = get_answers(&blueprint_directory);

        let tera_context = make_context(answers);

        let pattern = format!("{}/**/*.{}", blueprint_directory_str, TERA_FILE_EXTENSION);

        let mut tera = Tera::new(&pattern).unwrap();

        let destination_path = PathBuf::from(destination);

        let mut trx = Transaction::new();

        for entry in walkdir::WalkDir::new(&blueprint_directory) {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.is_file()
                && path
                    .extension()
                    .map(|ext| ext == TERA_FILE_EXTENSION)
                    .unwrap_or(false)
            {
                let relative_path = path.strip_prefix(&blueprint_directory).unwrap();
                let parent = relative_path
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new(""));
                let out_file_name = path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .replace(&format!(".{}", TERA_FILE_EXTENSION), "");

                let file_destination_path = destination_path.join(parent).join(out_file_name);

                let template_content = fs::read_to_string(path).unwrap();

                let rendered = tera.render_str(&template_content, &tera_context).unwrap();

                if let Some(parent_dir) = file_destination_path.parent() {
                    let copy = parent_dir.to_owned();

                    create_directory(&mut trx, copy.as_path()).unwrap();
                }

                write_file(&mut trx, file_destination_path.as_path(), rendered).unwrap();
            }
        }
    }

    pub fn list_templates(_repo: &str) {
        //
        println!("hello world");
    }
}
