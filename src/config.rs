use std::{fs, path::Path};

use indexmap::IndexMap;
use serde::Deserialize;

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
