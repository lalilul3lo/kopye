use crate::{
    errors::{FileFormat, FileOperation, IoError, ParseError},
    source::Source,
};
use indexmap::IndexMap;
use inquire::{
    required, validator::MinLengthValidator, Confirm, Editor, InquireError, MultiSelect, Select,
    Text,
};
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum PromptError {
    #[error("I/O error within prompt domain")]
    #[diagnostic(code(kopye::prompt::io))]
    Io(#[from] IoError),

    #[error("Parsing error within prompt domain")]
    #[diagnostic(code(kopye::prompt::parse))]
    Parse(#[from] ParseError),

    #[error("I/O error within prompt domain")]
    #[diagnostic(code(kopye::prompt::prompt))]
    Prompt {
        question: String,
        source: InquireError,
    },
}

#[derive(Debug, Deserialize, Clone)]
pub enum QuestionType {
    Text,
    Paragraph,
    Confirm,
    Select,
    MultiSelect,
}
#[derive(Debug, Deserialize, Clone)]
pub struct Question {
    pub r#type: QuestionType,
    pub help: String,
    pub choices: Option<Vec<String>>,
    // will populate when converting (QuestionFile::from_file)
    #[serde(skip)]
    pub dependency: Option<(String, String)>,
    #[serde(rename = "depends_on")]
    pub raw_dependency: Option<String>,
}
#[derive(Debug, Deserialize, Clone)]
pub struct QuestionsFile(pub IndexMap<String, Question>);
impl QuestionsFile {
    pub fn from_file(path: PathBuf) -> Result<Self, PromptError> {
        let content = fs::read_to_string(path.clone())
            .map_err(|err| IoError::new(FileOperation::Read, path.clone(), err))?;
        let mut parsed: QuestionsFile = toml::from_str(&content)
            .map_err(|err| ParseError::new(FileFormat::Toml, path.clone(), err))?;

        for (_qkey, qval) in &mut parsed.0 {
            if let Some(dep) = &qval.raw_dependency {
                let parts: Vec<&str> = dep.split("::").collect();

                if parts.len() == 2 {
                    let dependent_question = parts[0].to_string();
                    let expected_answer = parts[1].to_string();

                    qval.dependency = Some((dependent_question, expected_answer));
                }
            }
        }
        Ok(parsed)
    }
}

#[derive(Debug, Serialize, PartialEq, Clone)]
pub enum Answer {
    String(String),
    // Int(i64),
    // Float(f64),
    Bool(bool),
    Array(Vec<String>),
}

fn try_prompt(
    question: &str,
    config: &Question,
    answers: &mut IndexMap<String, Answer>,
) -> Result<(), PromptError> {
    match config.r#type {
        QuestionType::Text => {
            let answer = Text::new(question)
                .with_help_message(&config.help)
                .with_validator(required!(format!("{} is required", question)))
                .prompt()
                .map_err(|error| PromptError::Prompt {
                    question: question.to_string(),
                    source: error,
                })?;

            answers.insert(question.to_string(), Answer::String(answer));
        }
        QuestionType::Paragraph => {
            let answer = Editor::new(question)
                .with_formatter(&|submission| {
                    if submission.is_empty() {
                        String::from("<skipped>")
                    } else {
                        submission.into()
                    }
                })
                .with_help_message(&config.help)
                .prompt()
                .map_err(|error| PromptError::Prompt {
                    question: question.to_string(),
                    source: error,
                })?;

            answers.insert(question.to_string(), Answer::String(answer));
        }
        QuestionType::Confirm => {
            let answer = Confirm::new(question)
                .with_help_message(&config.help)
                .prompt()
                .map_err(|error| PromptError::Prompt {
                    question: question.to_string(),
                    source: error,
                })?;

            answers.insert(question.to_string(), Answer::Bool(answer));
        }
        QuestionType::Select => {
            if let Some(choices) = config.choices.clone() {
                let answer = Select::new(question, choices)
                    .with_help_message(&config.help)
                    .prompt()
                    .map_err(|error| PromptError::Prompt {
                        question: question.to_string(),
                        source: error,
                    })?;

                answers.insert(question.to_string(), Answer::String(answer));
            }
        }
        QuestionType::MultiSelect => {
            if let Some(choices) = config.choices.clone() {
                let answer = MultiSelect::new(question, choices)
                    .with_help_message(&config.help)
                    .with_validator(MinLengthValidator::new(1))
                    .prompt()
                    .map_err(|error| PromptError::Prompt {
                        question: question.to_string(),
                        source: error,
                    })?;

                answers.insert(question.to_string(), Answer::Array(answer));
            }
        }
    }

    Ok(())
}

fn do_work(
    question: &str,
    config: &Question,
    answers: &mut IndexMap<String, Answer>,
) -> Result<(), PromptError> {
    if let Some((dependent_question, expected_answer)) = &config.dependency {
        if let Some(Answer::String(current_answer)) = answers.get(dependent_question) {
            if expected_answer == current_answer {
                try_prompt(question, config, answers)?;
            }
        }
    } else {
        try_prompt(question, config, answers)?;
    }

    Ok(())
}

/// Marker type indicating that non-dependent prompts have **not** been processed yet.
struct NonDependentProcessed;
/// Marker type indicating that non-dependent prompts **have** been processed
/// successfully, allowing dependent prompts to be processed next.
struct NonDependentUnprocessed;

/// A manager that wraps the blueprint file and user answers, enforcing
/// a correct prompting order via Rust’s type system.
///
/// # Type Parameter
///
/// - `State`: A marker type indicating the prompting state. Can be:
///   - `NonDependentUnprocessed`: Have not yet asked non-dependent questions.
///   - `NonDependentProcessed`: Have asked non-dependent questions and can move on to dependent ones.
///
/// This design uses the "type-state" pattern to ensure at compile time
/// that dependent questions are never prompted before non-dependent ones.
struct PromptManager<State = NonDependentUnprocessed> {
    file: QuestionsFile,
    answers: IndexMap<String, Answer>,
    state: std::marker::PhantomData<State>,
}

impl PromptManager<NonDependentUnprocessed> {
    /// Prompts for all questions that do **not** have a dependency,
    /// transitioning from the `NonDependentUnprocessed` state to
    /// the `NonDependentProcessed` state.
    ///
    /// # Returns
    ///
    /// On success, returns a `PromptManager<NonDependentProcessed>` that
    /// can safely prompt for questions that **do** have dependencies.
    pub fn prompt_non_dependent(
        &mut self,
    ) -> Result<PromptManager<NonDependentProcessed>, PromptError> {
        for (question, config) in &self.file.0 {
            if config.dependency.is_none() {
                do_work(question, config, &mut self.answers)?;
            }
        }
        Ok(PromptManager {
            file: self.file.clone(),
            answers: self.answers.clone(),
            state: std::marker::PhantomData,
        })
    }
}
impl PromptManager<NonDependentProcessed> {
    /// Prompts for all questions that **do** have a dependency,
    /// remaining in the `NonDependentProcessed` state afterwards.
    ///
    /// # Returns
    ///
    /// On success, returns an updated `PromptManager<NonDependentProcessed>`.
    /// You may then call [`get_answers`](Self::get_answers) to retrieve the final results.
    pub fn prompt_dependent(&mut self) -> Result<Self, PromptError> {
        for (question, config) in &self.file.0 {
            if config.dependency.is_some() {
                do_work(question, config, &mut self.answers)?;
            }
        }
        Ok(PromptManager {
            file: self.file.clone(),
            answers: self.answers.clone(),
            state: std::marker::PhantomData,
        })
    }

    /// Consumes this manager and returns the collected answers.
    ///
    /// This is typically the final step after having prompted for
    /// both non-dependent and dependent questions.
    pub fn get_answers(self) -> IndexMap<String, Answer> {
        self.answers
    }
}
impl PromptManager {
    pub fn new(file: QuestionsFile) -> Self {
        PromptManager {
            file,
            answers: IndexMap::new(),
            state: std::marker::PhantomData,
        }
    }
}

/// Retrieves all answers by enforcing the correct prompting sequence:
/// 1) non-dependent questions, then 2) dependent questions, and finally
///    returns the gathered answers.
///
/// # Errors
///
/// Returns `PromptError` if any prompt operation fails.
pub fn get_answers(template_path: &Path) -> Result<IndexMap<String, Answer>, PromptError> {
    let file = QuestionsFile::from_file(template_path.join("blueprint.toml"))?;

    let mut manager = PromptManager::new(file);

    let mut non_dependent_prompted_manager = manager.prompt_non_dependent()?;

    let dependent_prompted_manager = non_dependent_prompted_manager.prompt_dependent()?;

    Ok(dependent_prompted_manager.get_answers())
}

pub fn get_project(config: Source) -> Result<String, PromptError> {
    let choices = config.projects.keys().collect();

    let question = String::from("Select template:");

    let answer = Select::new(&question, choices)
        .prompt()
        .map_err(|error| PromptError::Prompt {
            question: question.to_string(),
            source: error,
        })?;

    Ok(answer.to_owned())
}

pub fn get_destination() -> Result<String, PromptError> {
    let question = String::from("Destination");

    let answer = Text::new(&question)
        .prompt()
        .map_err(|error| PromptError::Prompt {
            question: question.to_string(),
            source: error,
        })?;

    Ok(answer.to_owned())
}
