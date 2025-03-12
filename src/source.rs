use crate::errors::{FileOperation, IoError};
use git2::Repository;
use indexmap::IndexMap;
use miette::Diagnostic;
use serde::Deserialize;
use std::{fs, path::PathBuf};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum SourceError {
    #[error("I/O error within source domain")]
    #[diagnostic(code(kopye::source::io))]
    Io(#[from] IoError),

    #[error("Unable to parse toml file at '{path}': {source}")]
    #[diagnostic(code(kopye::source::parse_toml), help("Review toml file"))]
    ParseToml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("unable to clone repo at: '{url}': {source}")]
    #[diagnostic(
        code(kopye::source::git_clone),
        help("Make sure that username and project name are correct")
    )]
    GitClone {
        url: String,
        path: PathBuf,
        source: git2::Error,
    },

    #[error("invalid github prefix provided: {url}")]
    #[diagnostic(
        code(kopye::source::invalid_git_prefix),
        help("Valid git prefix are: ['gh', 'gl']")
    )]
    InvalidGitPrefix { url: String },
}

#[derive(Debug, Deserialize, Clone)]
pub struct BlueprintInfo {
    pub path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Source {
    pub projects: IndexMap<String, BlueprintInfo>,
    pub source_dir: PathBuf,
}
impl Source {
    fn is_git(source: &str) -> bool {
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
            ).expect("a valid regex pattern");
        }

        GIT_URL_REGEX.is_match(source)
    }

    fn expand_git_short_url(url: &str) -> Result<String, SourceError> {
        if let Some(stripped) = url.strip_prefix("gh:") {
            Ok(format!("https://github.com/{}.git", stripped))
        } else if let Some(stripped) = url.strip_prefix("gl:") {
            Ok(format!("https://gitlab.com/{}.git", stripped))
        } else {
            Err(SourceError::InvalidGitPrefix {
                url: url.to_string(),
            })
        }
    }

    pub fn build_from(source: &str) -> Result<Self, SourceError> {
        let source_directory = if Source::is_git(source) {
            let directory = tempfile::tempdir()
                .map_err(|error| IoError::new(FileOperation::Mkdir, PathBuf::new(), error))?
                .into_path();

            let expanded_url = Source::expand_git_short_url(source)?;

            Repository::clone(&expanded_url, directory.as_path()).map_err(|err| {
                SourceError::GitClone {
                    url: expanded_url.clone(),
                    path: directory.clone(),
                    source: err,
                }
            })?;

            directory
        } else {
            std::path::PathBuf::from(source)
        };

        let source_file = source_directory.join("blueprints.toml");

        let content = fs::read_to_string(source_file.clone())
            .map_err(|error| IoError::new(FileOperation::Read, source_file.clone(), error))?;

        let parsed = toml::from_str(&content).map_err(|err| SourceError::ParseToml {
            path: source_file.clone(),
            source: err,
        })?;

        Ok(Source {
            source_dir: source_directory,
            projects: parsed, // TODO: rename to blueprints
        })
    }
}
