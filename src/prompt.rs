use indexmap::IndexMap;
use inquire::{required, validator::MinLengthValidator, Confirm, MultiSelect, Select, Text};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

use crate::config::Config;

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

#[derive(Debug, Serialize)]
pub enum Answer {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<String>),
}

pub fn get_answers(template_path: &Path) -> IndexMap<String, Answer> {
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

pub fn get_project(config: Config) {
    let choices = config.0.keys().collect();

    let answer = Select::new("Select template:", choices).prompt().unwrap();

    println!("selection: {}", answer);
}
