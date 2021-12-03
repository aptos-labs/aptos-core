// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{shared, shared::Home};
use anyhow::Result;
use diem_types::account_address::AccountAddress;
use include_dir::{include_dir, Dir};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Default blockchain configuration
pub const DEFAULT_BLOCKCHAIN: &str = "goodday";

/// Embedded directory holding project template used on creation of new shuffle projects
const EXAMPLES_DIR: Dir = include_dir!("../move/examples");

/// Embedded directory holding the Move stdlib for new projects
const MOVE_STDLIB_DIR: Dir = include_dir!("../../language/move-stdlib");

pub fn handle(home: &Home, blockchain: String, pathbuf: PathBuf) -> Result<()> {
    let project_path = pathbuf.as_path();
    println!("Creating shuffle project in {}", project_path.display());
    fs::create_dir_all(project_path)?;

    let config = shared::ProjectConfig::new(blockchain);
    write_shuffle_project_files(project_path, &config)?;
    write_move_project_template(project_path)?;

    home.generate_shuffle_path_if_nonexistent()?;
    home.write_default_networks_config_into_toml_if_nonexistent()?;

    shared::codegen_typescript_libraries(
        project_path,
        &AccountAddress::from_hex_literal(shared::PLACEHOLDER_ADDRESS)?,
    )?;
    Ok(())
}

fn write_shuffle_project_files(path: &Path, config: &shared::ProjectConfig) -> Result<()> {
    let toml_path = path.join("Shuffle.toml");
    let toml_string = toml::to_string(config)?;
    fs::write(toml_path, toml_string)?;
    Ok(())
}

// Writes the move packages for a new project
pub(crate) fn write_move_project_template(project_path: &Path) -> Result<()> {
    write_to(EXAMPLES_DIR, project_path)?;
    write_to(MOVE_STDLIB_DIR, &project_path.join("stdlib"))?;
    Ok(())
}

fn write_to(dir: Dir, path: &Path) -> Result<()> {
    for entry in dir.find("**/*").unwrap() {
        match entry {
            include_dir::DirEntry::Dir(d) => {
                fs::create_dir_all(path.join(d.path()))?;
            }
            include_dir::DirEntry::File(f) => {
                let dst = path.join(f.path());
                fs::write(dst.as_path(), f.contents())?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use shared::ProjectConfig;
    use tempfile::tempdir;

    #[test]
    fn test_write_project_config() {
        let dir = tempdir().unwrap();
        let config = ProjectConfig::new(String::from(DEFAULT_BLOCKCHAIN));

        write_shuffle_project_files(dir.path(), &config).unwrap();

        let config_string =
            fs::read_to_string(dir.path().join("Shuffle").with_extension("toml")).unwrap();
        let read_config: ProjectConfig = toml::from_str(config_string.as_str()).unwrap();
        assert_eq!(config, read_config);
    }

    #[test]
    fn test_handle_e2e() {
        let dir = tempdir().unwrap();
        let home = Home::new(dir.path()).unwrap();
        handle(
            &home,
            String::from(DEFAULT_BLOCKCHAIN),
            PathBuf::from(dir.path()),
        )
        .unwrap();

        // spot check move starter files
        let expected_example_content = String::from_utf8_lossy(include_bytes!(
            "../../move/examples/main/sources/Message.move"
        ));
        let actual_example_content =
            fs::read_to_string(dir.path().join("main/sources/Message.move")).unwrap();
        assert_eq!(expected_example_content, actual_example_content);

        // spot check stdlib
        let expected_example_content = String::from_utf8_lossy(include_bytes!(
            "../../../language/move-stdlib/sources/GUID.move"
        ));
        let actual_example_content =
            fs::read_to_string(dir.path().join("stdlib/sources/GUID.move")).unwrap();
        assert_eq!(expected_example_content, actual_example_content);
    }
}
