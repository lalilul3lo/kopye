use std::path::{Component, PathBuf};

use git2::Repository;
use indexmap::IndexMap;
use tera::Context;

use crate::prompt::Answer;

pub fn expand_git_short_url(url: &str) -> String {
    if let Some(stripped) = url.strip_prefix("gh:") {
        format!("https://github.com/{}.git", stripped)
    } else if let Some(stripped) = url.strip_prefix("gl:") {
        format!("https://gitlab.com/{}.git", stripped)
    } else {
        url.to_string() // TODO: Handle properly
    }
}

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

pub fn hydrate_tera_ctx(context: &mut Context, answers: IndexMap<String, Answer>) -> &mut Context {
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

pub fn is_git(source: &str) -> bool {
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

    GIT_URL_REGEX.is_match(source)
}

pub fn get_source_directory(source: &str) -> PathBuf {
    let source_directory = if is_git(source) {
        let directory = tempfile::tempdir().unwrap().into_path(); // ERROR(Tera)

        let expanded_url = expand_git_short_url(source);

        Repository::clone(&expanded_url, directory.as_path()).unwrap(); // ERROR(Git)

        directory
    } else {
        std::path::PathBuf::from(source)
    };

    source_directory
}
