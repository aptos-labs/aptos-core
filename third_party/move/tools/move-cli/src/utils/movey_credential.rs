// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Context, Result};
use move_command_line_common::movey_constants::{MOVEY_CREDENTIAL_PATH, MOVEY_URL};
use std::fs;
use toml_edit::easy::Value;

pub fn get_registry_api_token(move_home: &str) -> Result<String> {
    if let Ok(content) = get_api_token(move_home) {
        Ok(content)
    } else {
        bail!(
            "There seems to be an error with your Movey API token. \
            Please run `move movey-login` and follow the instructions."
        )
    }
}

pub fn get_api_token(move_home: &str) -> Result<String> {
    let credential_path = format!("{}{}", move_home, MOVEY_CREDENTIAL_PATH);
    let mut toml: Value = read_credential_file(&credential_path)?;
    let token = get_registry_field(&mut toml, "token")?;
    Ok(token.to_string().replace('\"', ""))
}

pub fn get_movey_url(move_home: &str) -> Result<String> {
    let credential_path = format!("{}{}", move_home, MOVEY_CREDENTIAL_PATH);
    let contents = fs::read_to_string(credential_path)?;
    let mut toml: Value = contents.parse()?;

    let movey_url = get_registry_field(&mut toml, "url");
    if let Ok(url) = movey_url {
        Ok(url.to_string().replace('\"', ""))
    } else {
        Ok(MOVEY_URL.to_string())
    }
}

fn get_registry_field<'a>(toml: &'a mut Value, field: &'a str) -> Result<&'a mut Value> {
    let registry = toml
        .as_table_mut()
        .context(format!("Error parsing {}", MOVEY_CREDENTIAL_PATH))?
        .get_mut("registry")
        .context(format!("Error parsing {}", MOVEY_CREDENTIAL_PATH))?;
    let value = registry
        .as_table_mut()
        .context("Error parsing registry table")?
        .get_mut(field)
        .context("Error parsing token")?;
    Ok(value)
}

pub fn read_credential_file(credential_path: &str) -> Result<Value> {
    let content = match fs::read_to_string(credential_path) {
        Ok(content) => content,
        Err(error) => bail!("Error reading input: {}", error),
    };
    content.parse().map_err(|e| {
        anyhow::Error::from(e).context(format!(
            "could not parse input at {} as TOML",
            &credential_path
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs::File};

    fn setup_move_home(test_path: &str) -> (String, String) {
        let cwd = env::current_dir().unwrap();
        let mut move_home: String = String::from(cwd.to_string_lossy());
        move_home.push_str(test_path);
        let credential_path = move_home.clone() + MOVEY_CREDENTIAL_PATH;

        (move_home, credential_path)
    }

    fn clean_up(move_home: &str) {
        let _ = fs::remove_dir_all(move_home);
    }

    #[test]
    fn get_api_token_works() {
        let test_path = String::from("/get_api_token_works");
        let (move_home, credential_path) = setup_move_home(&test_path);
        let _ = fs::create_dir_all(&move_home);
        File::create(&credential_path).unwrap();

        let content = r#"
            [registry]
            token = "test-token"
            "#;
        fs::write(&credential_path, content).unwrap();

        let token = get_registry_api_token(&move_home).unwrap();
        assert!(token.contains("test-token"));

        clean_up(&move_home)
    }

    #[test]
    fn get_api_token_fails_if_there_is_no_move_home_directory() {
        let test_path = String::from("/get_api_token_fails_if_there_is_no_move_home_directory");
        let (move_home, _) = setup_move_home(&test_path);
        let _ = fs::remove_dir_all(&move_home);

        let token = get_registry_api_token(&move_home);
        assert!(token.is_err());

        clean_up(&move_home)
    }

    #[test]
    fn get_api_token_fails_if_there_is_no_credential_file() {
        let test_path = String::from("/get_api_token_fails_if_there_is_no_credential_file");
        let (move_home, _) = setup_move_home(&test_path);
        let _ = fs::remove_dir_all(&move_home);
        fs::create_dir_all(&move_home).unwrap();

        let token = get_registry_api_token(&move_home);
        assert!(token.is_err());

        clean_up(&move_home)
    }

    #[test]
    fn get_api_token_fails_if_credential_file_is_in_wrong_format() {
        let test_path = String::from("/get_api_token_fails_if_credential_file_is_in_wrong_format");
        let (move_home, credential_path) = setup_move_home(&test_path);
        let _ = fs::remove_dir_all(&move_home);
        fs::create_dir_all(&move_home).unwrap();
        File::create(&credential_path).unwrap();

        let missing_double_quote = r#"
            [registry]
            token = test-token
            "#;
        fs::write(&credential_path, missing_double_quote).unwrap();
        let token = get_registry_api_token(&move_home);
        assert!(token.is_err());

        let wrong_token_field = r#"
            [registry]
            tokens = "test-token"
            "#;
        fs::write(&credential_path, wrong_token_field).unwrap();
        let token = get_registry_api_token(&move_home);
        assert!(token.is_err());

        clean_up(&move_home)
    }

    #[test]
    fn get_movey_url_works() {
        let test_path = String::from("/get_movey_url_works");
        let (move_home, credential_path) = setup_move_home(&test_path);
        let _ = fs::create_dir_all(&move_home);
        File::create(&credential_path).unwrap();
        let content = r#"
            [registry]
            token = "test-token"
            url = "test-url"
            "#;
        fs::write(&credential_path, content).unwrap();

        let url = get_movey_url(&move_home).unwrap();
        assert_eq!(url, "test-url");

        clean_up(&move_home)
    }

    #[test]
    fn get_movey_url_returns_default_url_if_url_field_not_existed() {
        let test_path = String::from("/get_movey_url_returns_default_url_if_url_field_not_existed");
        let (move_home, credential_path) = setup_move_home(&test_path);
        let _ = fs::create_dir_all(&move_home);
        File::create(&credential_path).unwrap();
        let content = r#"
            [registry]
            token = "test-token"
            "#;
        fs::write(&credential_path, content).unwrap();

        let url = get_movey_url(&move_home).unwrap();
        assert_eq!(url, MOVEY_URL);

        clean_up(&move_home)
    }
}
