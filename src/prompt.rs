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
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};
use tampopo::{errors::SortError, Graph};
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

    #[error("DAG sort error within prompt domain: {details}")]
    #[diagnostic(code(kopye::prompt::sort))]
    Sort {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
        details: String,
    },
}
impl PromptError {
    /// Converts a `SortError` from the graph sorting process into a `PromptError`.
    ///
    /// This helper function is used to convert errors from the sorting domain into a
    /// `PromptError::Sort` variant, preserving the error details.
    fn from_sort_error<Node>(err: SortError<Node>) -> Self
    where
        Node: Clone + Ord + std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
    {
        let details = err.to_string();
        PromptError::Sort {
            source: Box::new(err),
            details,
        }
    }
}

/// Represents a dependency in a question configuration.
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum Dependency {
    /// A simple condition, e.g., "is_binary:true"
    Condition(String),
    /// A list of dependencies that must all be true (AND logic)
    And { all: Vec<String> },
    /// A list of dependencies where at least one must be true (OR logic)
    Or { any: Vec<String> },
}
/// The type of prompt to display.
#[derive(Debug, Deserialize, Clone)]
pub enum QuestionType {
    /// A single-line text input
    Text,
    /// A multi-line text input
    Paragraph,
    /// A confirmation (yes/no) prompt
    Confirm,
    /// A single-select prompt
    Select,
    /// A multi-select prompt
    MultiSelect,
}

/// Configuration for a single prompt question.
#[derive(Debug, Deserialize, Clone)]
pub struct Question {
    /// The type of the question (e.g., text, paragraph, confirm)
    pub r#type: QuestionType,
    /// Help text describing the prompt.
    pub help: String,
    /// Optional list of choices for selection prompts
    pub choices: Option<Vec<String>>,
    /// Optional dependency that determines whether the prompt should be displayed
    #[serde(rename = "depends_on")]
    pub raw_dependency: Option<Dependency>,
}

/// Represents a collection of questions loaded from a file.
#[derive(Debug, Deserialize, Clone)]
pub struct QuestionsFile(pub IndexMap<String, Question>);
impl QuestionsFile {
    /// Loads and parses a questions file from the given path.
    pub fn from_file(path: PathBuf) -> Result<Self, PromptError> {
        let content = fs::read_to_string(path.clone())
            .map_err(|err| IoError::new(FileOperation::Read, path.clone(), err))?;
        let parsed: QuestionsFile = toml::from_str(&content)
            .map_err(|err| ParseError::new(FileFormat::Toml, path.clone(), err))?;

        Ok(parsed)
    }

    /// Constructs an adjacency list representing dependencies between questions.
    /// Each dependency in a question is parsed into an edge from the dependency question to the current question.
    pub fn adjacency_list_from_file(file: QuestionsFile) -> Vec<(String, String)> {
        file.0
            .iter()
            .flat_map(|(question_key, question_config)| {
                let dependencies: Vec<&str> = match &question_config.raw_dependency {
                    Some(Dependency::Condition(val)) => vec![val.as_str()],
                    Some(Dependency::And { all }) => all.iter().map(String::as_str).collect(),
                    Some(Dependency::Or { any }) => any.iter().map(String::as_str).collect(),
                    None => Vec::new(),
                };

                dependencies
                    .into_iter()
                    .filter_map(|dep_str| {
                        // Split dependency string "dependency_question:expected_answer"
                        dep_str.split_once(':').map(|(dependency_question, _)| {
                            (dependency_question.to_string(), question_key.clone())
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}

/// Represents an answer to a prompt.
#[derive(Debug, Serialize, PartialEq, Clone)]
pub enum Answer {
    String(String),
    // Int(i64),
    // Float(f64),
    Bool(bool),
    Array(Vec<String>),
}

/// Prompts the user with a question based on its configuration, and stores the answer.
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

/// Reorders a topologically sorted list of nodes so that nodes with no incoming edges
/// appear first in their original order.
///
/// This function processes the original list of nodes to identify independent nodes (with zero in-degree)
/// and places them at the beginning, followed by the rest of the nodes in the order provided by the sort.
pub fn stablize_topological_order<Node: std::hash::Hash + Eq + Clone>(
    graph: &Graph<Node>,
    sorted: Vec<Node>,
) -> Vec<Node> {
    // First, calculate the in-degree for each node.
    let mut in_degrees: HashMap<&Node, usize> = HashMap::new();
    for node in &graph.nodes {
        in_degrees.insert(node, 0);
    }
    for (_src, dest) in &graph.edges {
        if let Some(count) = in_degrees.get_mut(dest) {
            *count += 1;
        }
    }
    // Build a set of nodes that have no incoming edges.
    let independent: HashSet<&Node> = in_degrees
        .iter()
        .filter(|(_, &count)| count == 0)
        .map(|(&node, _)| node)
        .collect();

    // First, take all independent nodes in the original order.
    let mut stable_order = graph
        .nodes
        .iter()
        .filter(|node| independent.contains(*node))
        .cloned()
        .collect::<Vec<_>>();

    // Then, append the remaining nodes from the topologically sorted order,
    for node in sorted {
        if !independent.contains(&node) {
            stable_order.push(node);
        }
    }

    stable_order
}

/// Checks whether a dependency condition is satisfied based on previous answers.
/// The dependency string should be in the format "question:expected_value".
fn check_dependency(dep: &str, answers: &IndexMap<String, Answer>) -> bool {
    // TODO: create newtype to validate format of ":"
    if let Some((question, expected)) = dep.split_once(':') {
        if let Some(answer) = answers.get(question) {
            match answer {
                Answer::String(ans) => ans == expected,
                Answer::Bool(ans) => Ok(*ans) == expected.parse::<bool>(),
                Answer::Array(arr) => arr.contains(&expected.to_string()),
            }
        } else {
            false
        }
    } else {
        false
    }
}

/// Processes the questions file and gathers user answers.
///
/// This function reads a blueprint TOML file, constructs a dependency graph,
/// computes a topological order (with stabilization), and then prompts the user for answers
/// based on each question's configuration and dependencies.
pub fn get_answers(template_path: &Path) -> Result<IndexMap<String, Answer>, PromptError> {
    let file = QuestionsFile::from_file(template_path.join("blueprint.toml"))?;
    let nodes: Vec<String> = file.0.keys().cloned().collect();
    let edges = QuestionsFile::adjacency_list_from_file(file.clone());
    let graph = Graph { nodes, edges };
    let order = tampopo::sort_graph(&graph).map_err(PromptError::from_sort_error)?;
    let stablized_order = stablize_topological_order(&graph, order);
    let questions = file.0;
    let mut answers = IndexMap::new();

    for question_name in stablized_order {
        if let Some(config) = questions.get(&question_name) {
            let should_prompt = config.raw_dependency.as_ref().is_none_or(|dep| match dep {
                Dependency::Condition(val) => check_dependency(val, &answers),
                Dependency::And { all } => all.iter().all(|d| check_dependency(d, &answers)),
                Dependency::Or { any } => any.iter().any(|d| check_dependency(d, &answers)),
            });

            if should_prompt {
                try_prompt(&question_name, config, &mut answers)?;
            }
        }
    }

    Ok(answers)
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

pub fn apply_changes() -> Result<bool, PromptError> {
    let question = String::from("Apply changes?");

    let answer = Confirm::new(&question)
        .prompt()
        .map_err(|error| PromptError::Prompt {
            question: question.to_string(),
            source: error,
        })?;

    Ok(answer.to_owned())
}
