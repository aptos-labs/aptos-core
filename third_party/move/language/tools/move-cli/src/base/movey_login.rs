// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::utils::movey_credential::read_credential_file;
use anyhow::{bail, Result};
use clap::Parser;
use move_command_line_common::{
    env::MOVE_HOME,
    movey_constants::{MOVEY_CREDENTIAL_PATH, MOVEY_URL},
};
use std::{fs, fs::File, io, path::PathBuf};
use toml_edit::easy::{map::Map, Value};

#[derive(Parser)]
#[clap(name = "movey-login")]
pub struct MoveyLogin;

impl MoveyLogin {
    pub fn execute(self) -> Result<()> {
        println!(
            "Please paste the API Token found on {}/settings/tokens below",
            MOVEY_URL
        );
        let mut line = String::new();
        loop {
            match io::stdin().read_line(&mut line) {
                Ok(_) => {
                    line = line.trim().to_string();
                    if !line.is_empty() {
                        break;
                    }
                    println!("Invalid API Token. Try again!");
                }
                Err(err) => {
                    bail!("Error reading file: {}", err);
                }
            }
        }
        Self::save_credential(line, MOVE_HOME.clone())?;
        println!("Token for Movey saved.");
        Ok(())
    }

    pub fn save_credential(token: String, move_home: String) -> Result<()> {
        fs::create_dir_all(&move_home)?;
        let credential_path = move_home + MOVEY_CREDENTIAL_PATH;
        let credential_file = PathBuf::from(&credential_path);
        if !credential_file.exists() {
            create_credential_file(&credential_path)?;
        }

        let mut toml: Value = read_credential_file(&credential_path)?;
        // only update token key, keep the rest of the file intact
        if let Some(registry) = toml.as_table_mut().unwrap().get_mut("registry") {
            if let Some(toml_token) = registry.as_table_mut().unwrap().get_mut("token") {
                *toml_token = Value::String(token);
            } else {
                registry
                    .as_table_mut()
                    .unwrap()
                    .insert(String::from("token"), Value::String(token));
            }
        } else {
            let mut value = Map::new();
            value.insert(String::from("token"), Value::String(token));
            toml.as_table_mut()
                .unwrap()
                .insert(String::from("registry"), Value::Table(value));
        }

        let new_contents = toml.to_string();
        fs::write(credential_file, new_contents).expect("Unable to write file");
        Ok(())
    }
}

#[cfg(unix)]
fn create_credential_file(credential_path: &str) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let credential_file = File::create(credential_path)?;

    let mut perms = credential_file.metadata()?.permissions();
    perms.set_mode(0o600);
    credential_file.set_permissions(perms)?;
    Ok(())
}

#[cfg(windows)]
#[allow(unused)]
fn create_credential_file(credential_path: &str) -> Result<()> {
    let windows_path = credential_path.replace("/", "\\");
    File::create(&windows_path)?;
    Ok(())
}

#[cfg(not(any(unix, windows)))]
#[allow(unused)]
fn create_credential_file(credential_path: &str) -> Result<()> {
    bail!("OS not supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn setup_move_home(test_path: &str) -> (String, String) {
        let cwd = env::current_dir().unwrap();
        let mut move_home: String = String::from(cwd.to_string_lossy());
        if !test_path.is_empty() {
            move_home.push_str(test_path);
        } else {
            move_home.push_str("/test");
        }
        let credential_path = move_home.clone() + MOVEY_CREDENTIAL_PATH;
        (move_home, credential_path)
    }

    fn clean_up(move_home: &str) {
        let _ = fs::remove_dir_all(move_home);
    }

    #[test]
    fn save_credential_works_if_no_credential_file_exists() {
        let (move_home, credential_path) =
            setup_move_home("/save_credential_works_if_no_credential_file_exists");
        let _ = fs::remove_dir_all(&move_home);
        MoveyLogin::save_credential(String::from("test_token"), move_home.clone()).unwrap();

        let contents = fs::read_to_string(&credential_path).expect("Unable to read file");
        let mut toml: Value = contents.parse().unwrap();
        let registry = toml.as_table_mut().unwrap().get_mut("registry").unwrap();
        let token = registry.as_table_mut().unwrap().get_mut("token").unwrap();
        assert!(token.to_string().contains("test_token"));

        clean_up(&move_home);
    }

    #[test]
    fn save_credential_works_if_empty_credential_file_exists() {
        let (move_home, credential_path) =
            setup_move_home("/save_credential_works_if_empty_credential_file_exists");

        let _ = fs::remove_dir_all(&move_home);
        fs::create_dir_all(&move_home).unwrap();
        File::create(&credential_path).unwrap();

        let contents = fs::read_to_string(&credential_path).expect("Unable to read file");
        let mut toml: Value = contents.parse().unwrap();
        assert!(toml.as_table_mut().unwrap().get_mut("registry").is_none());

        MoveyLogin::save_credential(String::from("test_token"), move_home.clone()).unwrap();

        let contents = fs::read_to_string(&credential_path).expect("Unable to read file");
        let mut toml: Value = contents.parse().unwrap();
        let registry = toml.as_table_mut().unwrap().get_mut("registry").unwrap();
        let token = registry.as_table_mut().unwrap().get_mut("token").unwrap();
        assert!(token.to_string().contains("test_token"));

        clean_up(&move_home);
    }

    #[test]
    fn save_credential_works_if_token_field_exists() {
        let (move_home, credential_path) =
            setup_move_home("/save_credential_works_if_token_field_exists");

        let _ = fs::remove_dir_all(&move_home);
        fs::create_dir_all(&move_home).unwrap();
        File::create(&credential_path).unwrap();

        let old_content =
            String::from("[registry]\ntoken = \"old_test_token\"\nversion = \"0.0.0\"\n");
        fs::write(&credential_path, old_content).expect("Unable to write file");

        let contents = fs::read_to_string(&credential_path).expect("Unable to read file");
        let mut toml: Value = contents.parse().unwrap();
        let registry = toml.as_table_mut().unwrap().get_mut("registry").unwrap();
        let token = registry.as_table_mut().unwrap().get_mut("token").unwrap();
        assert!(token.to_string().contains("old_test_token"));
        assert!(!token.to_string().contains("new_world"));

        MoveyLogin::save_credential(String::from("new_world"), move_home.clone()).unwrap();

        let contents = fs::read_to_string(&credential_path).expect("Unable to read file");
        let mut toml: Value = contents.parse().unwrap();
        let registry = toml.as_table_mut().unwrap().get_mut("registry").unwrap();
        let token = registry.as_table_mut().unwrap().get_mut("token").unwrap();
        assert!(token.to_string().contains("new_world"));
        assert!(!token.to_string().contains("old_test_token"));
        let version = registry.as_table_mut().unwrap().get_mut("version").unwrap();
        assert!(version.to_string().contains("0.0.0"));

        clean_up(&move_home);
    }

    #[test]
    fn save_credential_works_if_empty_token_field_exists() {
        let (move_home, credential_path) =
            setup_move_home("/save_credential_works_if_empty_token_field_exists");

        let _ = fs::remove_dir_all(&move_home);
        fs::create_dir_all(&move_home).unwrap();
        File::create(&credential_path).unwrap();

        let old_content = String::from("[registry]\ntoken = \"\"\nversion = \"0.0.0\"\n");
        fs::write(&credential_path, old_content).expect("Unable to write file");

        let contents = fs::read_to_string(&credential_path).expect("Unable to read file");
        let mut toml: Value = contents.parse().unwrap();
        let registry = toml.as_table_mut().unwrap().get_mut("registry").unwrap();
        let token = registry.as_table_mut().unwrap().get_mut("token").unwrap();
        assert!(!token.to_string().contains("test_token"));

        MoveyLogin::save_credential(String::from("test_token"), move_home.clone()).unwrap();

        let contents = fs::read_to_string(&credential_path).expect("Unable to read file");
        let mut toml: Value = contents.parse().unwrap();
        let registry = toml.as_table_mut().unwrap().get_mut("registry").unwrap();
        let token = registry.as_table_mut().unwrap().get_mut("token").unwrap();
        assert!(token.to_string().contains("test_token"));
        let version = registry.as_table_mut().unwrap().get_mut("version").unwrap();
        assert!(version.to_string().contains("0.0.0"));

        clean_up(&move_home);
    }
}
