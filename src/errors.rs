use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum FileOperation {
    #[error("reading a file")]
    Read,
    #[error("writing a file")]
    Write,
    #[error("creating a directory")]
    Mkdir,
}
#[derive(Debug, Error, Diagnostic)]
#[error("I/O error: {operation} on path '{path}'")]
#[diagnostic(
    code(kopye::io),
    help("Check file permissions, disk space, or that the path is correct.")
)]
pub struct IoError {
    pub operation: FileOperation,
    pub path: std::path::PathBuf,
    #[source]
    pub source: std::io::Error,
}
impl IoError {
    pub fn new(operation: FileOperation, path: std::path::PathBuf, error: std::io::Error) -> Self {
        Self {
            operation,
            path,
            source: error,
        }
    }
}

#[derive(Debug, Error, Diagnostic)]
pub enum FileFormat {
    #[error("toml")]
    Toml,
}
#[derive(Debug, Error, Diagnostic)]
#[error("Parsing error: {file_format} on '{path}'")]
#[diagnostic(code(kopye::parse), help("Review file"))]
pub struct ParseError {
    pub file_format: FileFormat,
    pub path: std::path::PathBuf,
    #[source]
    pub source: toml::de::Error,
}
impl ParseError {
    pub fn new(file_format: FileFormat, path: std::path::PathBuf, error: toml::de::Error) -> Self {
        Self {
            file_format,
            path,
            source: error,
        }
    }
}
