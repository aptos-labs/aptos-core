// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Smoke tests for the `indexer-ci-helper` binary. Logic tests live in `src/ci_helper/`.

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output},
};
use tempfile::TempDir;

fn run_helper(args: &[&str], env: &[(&str, &str)]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_indexer-ci-helper"))
        .args(args)
        .env_remove("TESTNET_API_KEY_VALUE")
        .env_remove("MAINNET_API_KEY_VALUE")
        .envs(env.iter().copied())
        .output()
        .expect("helper must start")
}

fn logs(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn committed_config() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("imported_transactions/imported_transactions.yaml")
        .display()
        .to_string()
}

#[test]
fn validate_config_accepts_the_committed_configuration() {
    let output = run_helper(&["validate-config", "--config", &committed_config()], &[]);
    assert!(output.status.success(), "{}", logs(&output));
}

#[test]
fn materialize_config_writes_keys_without_printing_them() {
    let original = fs::read_to_string(committed_config()).unwrap();
    let root = TempDir::new().unwrap();
    let materialized = root.path().join("out/imported_transactions.yaml");
    let args = [
        "materialize-config",
        "--input",
        &committed_config(),
        "--output",
        materialized.to_str().unwrap(),
    ];

    let missing = run_helper(&args, &[("TESTNET_API_KEY_VALUE", "testnet-secret")]);
    assert!(!missing.status.success());
    assert!(
        logs(&missing).contains("MAINNET_API_KEY_VALUE"),
        "{}",
        logs(&missing)
    );
    assert!(!materialized.exists());

    let output = run_helper(&args, &[
        ("TESTNET_API_KEY_VALUE", "testnet-secret"),
        ("MAINNET_API_KEY_VALUE", "mainnet-secret"),
    ]);
    assert!(output.status.success(), "{}", logs(&output));
    assert!(!logs(&output).contains("secret"));
    let mode = fs::metadata(&materialized).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "materialized config mode is {mode:o}");
    let content = fs::read_to_string(&materialized).unwrap();
    assert!(content.contains("api_key: testnet-secret"), "{content}");
    assert!(content.contains("api_key: mainnet-secret"), "{content}");
    assert_eq!(fs::read_to_string(committed_config()).unwrap(), original);
}

#[test]
fn compare_writes_all_step_outputs() {
    let root = TempDir::new().unwrap();
    let baseline = root.path().join("baseline/imported_testnet_txns");
    let generated = root.path().join("generated/imported_testnet_txns");
    fs::create_dir_all(&baseline).unwrap();
    fs::create_dir_all(&generated).unwrap();
    fs::write(baseline.join("tx.json"), "{\"value\": 1}\n").unwrap();
    fs::write(generated.join("tx.json"), "{\"value\": 2}\n").unwrap();
    let github_output = root.path().join("github-output");

    let output = run_helper(
        &[
            "compare",
            "--baseline-dir",
            root.path().join("baseline").to_str().unwrap(),
            "--generated-dir",
            root.path().join("generated").to_str().unwrap(),
            "--github-output",
            github_output.to_str().unwrap(),
        ],
        &[],
    );

    assert!(output.status.success(), "{}", logs(&output));
    assert_eq!(
        fs::read_to_string(github_output).unwrap(),
        "diff_found=true\nnew_file_found=false\ndispatch_required=true\n"
    );
}
