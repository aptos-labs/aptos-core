// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};
use toml;

/// Default blockchain configuration
pub const DEFAULT_BLOCKCHAIN: &str = "dpn";

pub fn handle(blockchain: String, path: PathBuf) -> Result<()> {
    println!("Creating shuffle project in {}", path.display());
    fs::create_dir_all(path.as_path())?;

    let config = Config { blockchain };
    write_config(path.as_path(), config)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct Config {
    blockchain: String,
}

fn write_config(path: &Path, config: Config) -> Result<()> {
    let toml_path = PathBuf::from(path).join("Shuffle").with_extension("toml");
    let toml_string = toml::to_string(&config)?;
    fs::write(toml_path, toml_string)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_handle() {
        let dir = tempdir().unwrap();
        handle(String::from(DEFAULT_BLOCKCHAIN), PathBuf::from(dir.path())).unwrap();
        let expectation = r#"blockchain = "dpn"
"#;
        let actual = fs::read_to_string(dir.path().join("Shuffle").with_extension("toml")).unwrap();
        assert_eq!(expectation, actual);
    }
}
