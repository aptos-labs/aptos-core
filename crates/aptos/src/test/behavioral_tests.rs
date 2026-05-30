// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::cli_runner::run_cmd;
use crate::common::utils::cli_build_information;
use aptos_types::account_address::{create_resource_address, AccountAddress};
use serde_json::Value;
use std::{collections::BTreeMap, fs};

fn assert_contains(message: &str, expected: &str) {
    assert!(
        message.contains(expected),
        "Expected message to contain {expected:?}, but it did not. Message: {message:?}"
    );
}

fn parse_cli_json_output(output: &str) -> Value {
    let wrapper: Value = serde_json::from_str(output).expect("CLI should emit JSON");
    wrapper
        .get("Result")
        .cloned()
        .expect("CLI success output should contain Result")
}

fn parse_cli_error(output: &str) -> String {
    if let Ok(wrapper) = serde_json::from_str::<Value>(output) {
        if let Some(err) = wrapper.get("Error").and_then(Value::as_str) {
            return err.to_string();
        }
    }
    output.to_string()
}

/// `aptos info` returns JSON build metadata on stdout.
#[tokio::test]
async fn info_returns_build_metadata_json() {
    let output = run_cmd(&["aptos", "info"]).await.expect("aptos info should succeed");
    let parsed: BTreeMap<String, String> =
        serde_json::from_value(parse_cli_json_output(&output)).expect("aptos info result JSON");
    let expected = cli_build_information();
    for key in expected.keys() {
        assert!(
            parsed.contains_key(key),
            "aptos info JSON missing expected key {key:?}"
        );
    }
}

/// `aptos key generate` with a fixed seed is deterministic and writes key files.
#[tokio::test]
async fn key_generate_with_seed_is_deterministic() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let output_file = temp_dir.path().join("test-key");

    let seed = "0".repeat(64);
    let args = [
        "aptos",
        "key",
        "generate",
        "--output-file",
        output_file.to_str().unwrap(),
        "--random-seed",
        &seed,
        "--assume-yes",
    ];
    run_cmd(&args).await.expect("first key generate should succeed");
    let first_private = fs::read_to_string(&output_file).expect("private key file");

    fs::remove_file(&output_file).ok();
    fs::remove_file(output_file.with_extension("pub")).ok();

    run_cmd(&args).await.expect("second key generate should succeed");
    let second_private = fs::read_to_string(&output_file).expect("private key file");

    assert_eq!(first_private, second_private, "generated keys should match");
    assert!(
        output_file.with_extension("pub").exists(),
        "public key file should be created"
    );
}

/// Vanity prefix is rejected for non-ed25519 key types.
#[tokio::test]
async fn key_generate_rejects_vanity_for_bls12381() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let output_file = temp_dir.path().join("bls-key");
    let error = run_cmd(&[
        "aptos",
        "key",
        "generate",
        "--key-type",
        "bls12381",
        "--vanity-prefix",
        "0xace",
        "--output-file",
        output_file.to_str().unwrap(),
        "--assume-yes",
    ])
    .await
    .expect_err("should reject vanity prefix for bls12381");
    assert_contains(&error, "Vanity prefixes are only accepted");
}

/// `aptos account derive-resource-account-address` returns a stable address for fixed inputs.
#[tokio::test]
async fn derive_resource_account_address_is_deterministic() {
    let output = run_cmd(&[
        "aptos",
        "account",
        "derive-resource-account-address",
        "--address",
        "0x1",
        "--seed",
        "cli-behavioral-test",
        "--seed-encoding",
        "utf8",
    ])
    .await
    .expect("derive-resource-account-address should succeed");

    let address: AccountAddress = serde_json::from_value::<String>(parse_cli_json_output(&output))
        .expect("address should be JSON string")
        .parse()
        .expect("valid account address");
    let expected = create_resource_address(AccountAddress::ONE, b"cli-behavioral-test");
    assert_eq!(address, expected, "resource account address should be deterministic");

    // Re-run and compare exact JSON output.
    let output_again = run_cmd(&[
        "aptos",
        "account",
        "derive-resource-account-address",
        "--address",
        "0x1",
        "--seed",
        "cli-behavioral-test",
        "--seed-encoding",
        "utf8",
    ])
    .await
    .expect("second derive should succeed");
    assert_eq!(output, output_again);
}

/// `aptos account balance` without REST URL or profile fails with a clear error.
#[tokio::test]
async fn account_balance_requires_rest_url_or_profile() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let error = run_cmd_in_dir(
        temp_dir.path(),
        &["aptos", "account", "balance", "--account", "0x1"],
    )
    .await
    .expect_err("balance without config should fail");
    let message = parse_cli_error(&error);
    assert!(
        message.contains("No rest url given") || message.contains("Unable to find config"),
        "unexpected error: {message:?}"
    );
}

/// `aptos config show-profiles` reports a helpful error when no local config exists.
#[tokio::test]
async fn config_show_profiles_requires_local_config() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let error = run_cmd_in_dir(temp_dir.path(), &["aptos", "config", "show-profiles"])
        .await
        .expect_err("show-profiles without config should fail");
    assert_contains(&parse_cli_error(&error), "Unable to find config");
}

async fn run_cmd_in_dir(dir: &std::path::Path, args: &[&str]) -> crate::CliResult {
    let restore_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    std::env::set_current_dir(dir).expect("set current dir");
    let result = run_cmd(args).await;
    std::env::set_current_dir(restore_dir).expect("restore current dir");
    result
}

/// `aptos node check-network-connectivity` validates inputs before connecting.
#[tokio::test]
async fn node_check_network_connectivity_validates_inputs() {
    let invalid_address = run_cmd(&[
        "aptos",
        "node",
        "check-network-connectivity",
        "--address",
        "invalid-address",
        "--chain-id",
        "mainnet",
    ])
    .await
    .expect_err("invalid address should fail");
    assert_contains(&invalid_address, "Invalid address");

    let invalid_chain = run_cmd(&[
        "aptos",
        "node",
        "check-network-connectivity",
        "--address",
        "/ip4/34.70.116.169/tcp/6182/noise-ik/0x249f3301db104705652e0a0c471b46d13172b2baf14e31f007413f3baee46b0c/handshake/0",
        "--chain-id",
        "invalid-chain",
    ])
    .await
    .expect_err("invalid chain-id should fail");
    assert_contains(&invalid_chain, "invalid value");
}
