// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use httpmock::{prelude::*, Mock};
use move_cli::sandbox::commands::test;
use move_command_line_common::{
    files,
    movey_constants::{MOVEY_CREDENTIAL_PATH, MOVEY_URL},
};
use serde_json::json;
#[cfg(unix)]
use std::fs::File;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    env, fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};
use toml_edit::easy::Value;

pub const CLI_METATEST_PATH: [&str; 3] = ["tests", "metatests", "args.txt"];

fn get_cli_binary_path() -> PathBuf {
    let cli_exe = env!("CARGO_BIN_EXE_move");
    PathBuf::from(cli_exe)
}

fn get_metatest_path() -> PathBuf {
    CLI_METATEST_PATH.iter().collect()
}

#[test]
fn run_metatest() {
    let path_cli_binary = get_cli_binary_path();
    let path_metatest = get_metatest_path();

    // local workspace + with coverage
    assert!(test::run_all(&path_metatest, path_cli_binary.as_path(), false, true).is_ok());

    // temp workspace + with coverage
    assert!(test::run_all(&path_metatest, &path_cli_binary, true, true).is_ok());

    // local workspace + without coverage
    assert!(test::run_all(&path_metatest, &path_cli_binary, false, false).is_ok());

    // temp workspace + without coverage
    assert!(test::run_all(&path_metatest, &path_cli_binary, true, false).is_ok());
}

#[test]
fn cross_process_locking_git_deps() {
    let cli_exe = env!("CARGO_BIN_EXE_move");
    let handle = std::thread::spawn(move || {
        Command::new(cli_exe)
            .current_dir("./tests/cross_process_tests/Package1")
            .args(["package", "build"])
            .output()
            .expect("Package1 failed");
    });
    let cli_exe = env!("CARGO_BIN_EXE_move").to_string();
    Command::new(cli_exe)
        .current_dir("./tests/cross_process_tests/Package2")
        .args(["package", "build"])
        .output()
        .expect("Package2 failed");
    handle.join().unwrap();
}

const UPLOAD_PACKAGE_PATH: &str = "./tests/upload_tests";
#[test]
fn upload_package_to_movey_works() {
    let package_path = format!("{}/valid_package1", UPLOAD_PACKAGE_PATH);
    init_git(&package_path, true);
    let server = MockServer::start();
    let server_mock = mock_movey_upload_with_response_body_and_status_code(&server, 200, None);
    init_stub_registry_file(&package_path, &server.base_url());
    let relative_package_path = PathBuf::from(&package_path);
    let absolute_package_path =
        files::path_to_string(&relative_package_path.canonicalize().unwrap()).unwrap();

    let cli_exe = env!("CARGO_BIN_EXE_move");
    let output = Command::new(cli_exe)
        .env("MOVE_HOME", &absolute_package_path)
        .current_dir(&absolute_package_path)
        .args(["movey-upload"])
        .output()
        .unwrap();

    server_mock.assert();
    assert!(output.status.success());
    let output = String::from_utf8_lossy(output.stdout.as_slice()).to_string();
    assert!(
        output.contains("Your package has been successfully uploaded to Movey"),
        "{}",
        output
    );

    clean_up(&absolute_package_path);
}

#[test]
fn upload_package_to_movey_prints_error_message_if_server_respond_4xx() {
    let package_path = format!("{}/valid_package2", UPLOAD_PACKAGE_PATH);
    init_git(&package_path, true);
    let server = MockServer::start();
    let server_mock = mock_movey_upload_with_response_body_and_status_code(
        &server,
        400,
        Some("Invalid Api token"),
    );
    init_stub_registry_file(&package_path, &server.base_url());
    let relative_package_path = PathBuf::from(&package_path);
    let absolute_package_path =
        files::path_to_string(&relative_package_path.canonicalize().unwrap()).unwrap();

    let cli_exe = env!("CARGO_BIN_EXE_move");
    let output = Command::new(cli_exe)
        .env("MOVE_HOME", &absolute_package_path)
        .current_dir(&absolute_package_path)
        .args(["movey-upload"])
        .output()
        .unwrap();

    server_mock.assert();
    assert!(!output.status.success());
    let output = String::from_utf8_lossy(output.stderr.as_slice()).to_string();
    assert!(output.contains("Error: Invalid Api token"), "{}", output);

    clean_up(&absolute_package_path);
}

#[test]
fn upload_package_to_movey_prints_hardcoded_error_message_if_server_respond_5xx() {
    let package_path = format!("{}/valid_package3", UPLOAD_PACKAGE_PATH);
    init_git(&package_path, true);
    let server = MockServer::start();
    let server_mock = mock_movey_upload_with_response_body_and_status_code(
        &server,
        500,
        Some("Invalid Api token"),
    );
    init_stub_registry_file(&package_path, &server.base_url());
    let relative_package_path = PathBuf::from(&package_path);
    let absolute_package_path =
        files::path_to_string(&relative_package_path.canonicalize().unwrap()).unwrap();

    let cli_exe = env!("CARGO_BIN_EXE_move");
    let output = Command::new(cli_exe)
        .env("MOVE_HOME", &absolute_package_path)
        .current_dir(&absolute_package_path)
        .args(["movey-upload"])
        .output()
        .unwrap();

    server_mock.assert();
    assert!(!output.status.success());
    let output = String::from_utf8_lossy(output.stderr.as_slice()).to_string();
    assert!(
        output.contains("Error: An unexpected error occurred. Please try again later"),
        "{}",
        output
    );

    clean_up(&absolute_package_path);
}

#[test]
fn upload_package_to_movey_with_no_remote_should_panic() {
    let package_path = format!("{}/no_git_remote_package", UPLOAD_PACKAGE_PATH);
    init_git(&package_path, false);

    let cli_exe = env!("CARGO_BIN_EXE_move");
    let output = Command::new(cli_exe)
        .current_dir(&package_path)
        .args(["movey-upload"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let error = String::from_utf8_lossy(output.stderr.as_slice()).to_string();
    assert!(error.contains("invalid git repository"));

    clean_up(&package_path);
}

// is_valid == true: all git commands are run
// is_valid == false: missing git remote add command
fn init_git(package_path: &str, is_valid: bool) {
    Command::new("git")
        .current_dir(package_path)
        .args(["init"])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(package_path)
        .args(["add", "."])
        .output()
        .unwrap();
    if is_valid {
        Command::new("git")
            .current_dir(package_path)
            .args([
                "remote",
                "add",
                "test-origin",
                "git@github.com:move-language/move.git",
            ])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(package_path)
            .args(["config", "user.email", "\"you@example.com\""])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(package_path)
            .args(["config", "user.name", "\"Your Name\""])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(package_path)
            .args(["commit", "--allow-empty", "-m", "initial commit"])
            .output()
            .unwrap();
    }
}
#[test]
fn save_credential_works() {
    let cli_exe = env!("CARGO_BIN_EXE_move");
    let (move_home, credential_path) = setup_move_home("/save_credential_works");
    assert!(fs::read_to_string(&credential_path).is_err());

    match Command::new(cli_exe)
        .env("MOVE_HOME", &move_home)
        .current_dir(".")
        .args(["movey-login"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(child) => {
            let token = "test_token";
            child
                .stdin
                .as_ref()
                .unwrap()
                .write_all(token.as_bytes())
                .unwrap();
            match child.wait_with_output() {
                Ok(output) => {
                    assert!(String::from_utf8_lossy(&output.stdout).contains(&format!(
                        "Please paste the API Token found on {}/settings/tokens below",
                        MOVEY_URL
                    )));
                    Ok(())
                }
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    }
    .unwrap();

    let contents = fs::read_to_string(&credential_path).expect("Unable to read file");
    let mut toml: Value = contents.parse().unwrap();
    let registry = toml.as_table_mut().unwrap().get_mut("registry").unwrap();
    let token = registry.as_table_mut().unwrap().get_mut("token").unwrap();
    assert!(token.to_string().contains("test_token"));

    let _ = fs::remove_dir_all(move_home);
}

#[cfg(unix)]
#[test]
fn save_credential_fails_if_undeletable_credential_file_exists() {
    let cli_exe = env!("CARGO_BIN_EXE_move");
    let (move_home, credential_path) =
        setup_move_home("/save_credential_fails_if_undeletable_credential_file_exists");
    let file = File::create(&credential_path).unwrap();
    let mut perms = file.metadata().unwrap().permissions();
    perms.set_mode(0o000);
    file.set_permissions(perms).unwrap();

    match std::process::Command::new(cli_exe)
        .env("MOVE_HOME", &move_home)
        .current_dir(".")
        .args(["movey-login"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => {
            let token = "test_token";
            child
                .stdin
                .as_ref()
                .unwrap()
                .write_all(token.as_bytes())
                .unwrap();
            match child.wait_with_output() {
                Ok(output) => {
                    assert!(String::from_utf8_lossy(&output.stdout).contains(&format!(
                        "Please paste the API Token found on {}/settings/tokens below",
                        MOVEY_URL
                    )));
                    assert!(String::from_utf8_lossy(&output.stderr)
                        .contains("Error: Error reading input: Permission denied (os error 13)"));
                    Ok(())
                }
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    }
    .unwrap();

    let mut perms = file.metadata().unwrap().permissions();
    perms.set_mode(0o600);
    file.set_permissions(perms).unwrap();
    let _ = fs::remove_file(&credential_path);

    let _ = fs::remove_dir_all(move_home);
}

fn setup_move_home(test_path: &str) -> (String, String) {
    let cwd = env::current_dir().unwrap();
    let mut move_home: String = String::from(cwd.to_string_lossy());
    move_home.push_str(test_path);
    let _ = fs::remove_dir_all(&move_home);
    fs::create_dir_all(&move_home).unwrap();
    let credential_path = move_home.clone() + MOVEY_CREDENTIAL_PATH;
    (move_home, credential_path)
}

fn clean_up(package_path: &str) {
    fs::remove_dir_all(format!("{}/.git", package_path)).unwrap();
    let credential_path = format!("{}{}", package_path, MOVEY_CREDENTIAL_PATH);
    let _ = fs::remove_file(&credential_path);
}

// create a dummy move_credential.toml file for testing
fn init_stub_registry_file(package_path: &str, base_url: &str) {
    let credential_path = format!("{}{}", package_path, MOVEY_CREDENTIAL_PATH);
    let content = format!(
        r#"
        [registry]
        token = "test-token"
        url = "{}"
        "#,
        base_url
    );
    fs::write(credential_path, content).expect("Unable to write file");
}

// create a mock server to check if the request is sent or not, also returns a stub response for testing
fn mock_movey_upload_with_response_body_and_status_code<'a>(
    server: &'a MockServer,
    status_code: u16,
    response_body: Option<&str>,
) -> Mock<'a> {
    server.mock(|when, then| {
        when.method(POST)
            .path("/api/v1/packages/upload")
            .header("content-type", "application/json")
            .json_body(json!({
            "github_repo_url": "https://github.com/move-language/move",
            "total_files": 2,
            "token": "test-token",
            "subdir": '\n'
            }));
        then.status(status_code).body(response_body.unwrap_or(""));
    })
}
