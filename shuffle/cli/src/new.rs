// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::shared;
use anyhow::Result;
use diem_genesis_tool::validator_builder::ValidatorConfig;
use diem_types::on_chain_config::VMPublishingOption;
use include_dir::{include_dir, Dir};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Default blockchain configuration
pub const DEFAULT_BLOCKCHAIN: &str = "goodday";

/// Directory of generated transaction builders for helloblockchain.
const EXAMPLES_DIR: Dir = include_dir!("../move/examples");
pub const MESSAGE_EXAMPLE_PATH: &str = "Message";

const REPL_FILE_CONTENT: &[u8] = include_bytes!("../repl.ts");

pub fn handle(blockchain: String, pathbuf: PathBuf) -> Result<()> {
    let project_path = pathbuf.as_path();
    println!("Creating shuffle project in {}", project_path.display());
    fs::create_dir_all(project_path)?;

    let config = shared::Config { blockchain };
    write_project_files(project_path, &config)?;
    write_example_move_packages(project_path)?;
    generate_validator_config(project_path)?;
    Ok(())
}

fn write_project_files(path: &Path, config: &shared::Config) -> Result<()> {
    let toml_path = path.join("Shuffle.toml");
    let toml_string = toml::to_string(config)?;
    fs::write(toml_path, toml_string)?;

    let repl_ts_path = path.join("repl.ts");
    fs::write(repl_ts_path, REPL_FILE_CONTENT)?;
    Ok(())
}

// Writes the move packages for a new project
fn write_example_move_packages(root_path: &Path) -> Result<()> {
    println!("Copying Examples...");
    for entry in EXAMPLES_DIR.find("**/*").unwrap() {
        match entry {
            include_dir::DirEntry::Dir(d) => {
                fs::create_dir_all(root_path.join(d.path()))?;
            }
            include_dir::DirEntry::File(f) => {
                let dst = root_path.join(f.path());
                fs::write(dst.as_path(), f.contents())?;
            }
        }
    }
    Ok(())
}

fn generate_validator_config(project_path: &Path) -> Result<ValidatorConfig> {
    let publishing_option = VMPublishingOption::open();
    shuffle_custom_node::generate_validator_config(
        project_path.join("nodeconfig").as_path(),
        diem_framework_releases::current_module_blobs().to_vec(),
        publishing_option,
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use diem_config::config::NodeConfig;
    use shared::Config;
    use tempfile::tempdir;

    #[test]
    fn test_write_project_config() {
        let dir = tempdir().unwrap();
        let config = Config {
            blockchain: String::from(DEFAULT_BLOCKCHAIN),
        };

        write_project_files(dir.path(), &config).unwrap();

        let config_string =
            fs::read_to_string(dir.path().join("Shuffle").with_extension("toml")).unwrap();
        let read_config: Config = toml::from_str(config_string.as_str()).unwrap();
        assert_eq!(config, read_config);
    }

    #[test]
    fn test_handle_e2e() {
        let dir = tempdir().unwrap();
        handle(String::from(DEFAULT_BLOCKCHAIN), PathBuf::from(dir.path())).unwrap();

        // spot check move starter files
        let expected_example_content = String::from_utf8_lossy(include_bytes!(
            "../../move/examples/Message/sources/Message.move"
        ));
        let actual_example_content =
            fs::read_to_string(dir.path().join("Message/sources/Message.move")).unwrap();
        assert_eq!(expected_example_content, actual_example_content);

        // check if we can load generated node.yaml config file
        let _node_config =
            NodeConfig::load(dir.path().join("nodeconfig/0/node.yaml").as_path()).unwrap();
    }
}
