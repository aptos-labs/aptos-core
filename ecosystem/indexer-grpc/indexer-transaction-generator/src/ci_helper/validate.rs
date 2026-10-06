// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Validation of the PR-controlled `imported_transactions.yaml` and `move_fixtures/`.

use super::Network;
use crate::config::{load_script_transactions, TransactionImporterConfig};
use anyhow::{ensure, Context, Result};
use std::{fs, path::Path};

/// Parses and validates `imported_transactions.yaml` and its sibling `move_fixtures/` folder.
pub fn validate_config_file(config_path: &Path) -> Result<TransactionImporterConfig> {
    let raw = fs::read_to_string(config_path)
        .with_context(|| format!("failed to read configuration at {}", config_path.display()))?;
    let config: TransactionImporterConfig = serde_yaml::from_str(&raw)
        .with_context(|| format!("configuration at {} is invalid", config_path.display()))?;
    validate_importer_config(&config)?;
    let move_folder = config_path
        .parent()
        .context("configuration path has no parent directory")?
        .join("move_fixtures");
    if move_folder.exists() {
        load_script_transactions(&move_folder)?;
    }
    Ok(config)
}

fn validate_importer_config(config: &TransactionImporterConfig) -> Result<()> {
    ensure!(
        config.configs.len() == Network::ALL.len()
            && Network::ALL
                .iter()
                .all(|network| config.configs.contains_key(network.name())),
        "configuration must contain only testnet and mainnet"
    );
    for network in Network::ALL {
        let network_config = &config.configs[network.name()];
        ensure!(
            network_config.api_key.as_deref() == Some(network.placeholder()),
            "{} api_key must be the placeholder {}",
            network.name(),
            network.placeholder()
        );
        let endpoint = &network_config.transaction_stream_endpoint;
        ensure!(
            endpoint.scheme() == "https" && endpoint.host_str() == Some(network.endpoint_host()),
            "{} endpoint must be https://{}",
            network.name(),
            network.endpoint_host()
        );
        ensure!(
            !network_config.versions_to_import.is_empty(),
            "{} versions_to_import must be non-empty",
            network.name()
        );
    }
    config.validate()
}

#[cfg(test)]
mod tests {
    use crate::ci_helper::test_utils::{assert_error, validate, Files, VALID_CONFIG};

    #[test]
    fn validate_rejects_unsafe_importer_configs() {
        let cases = [
            ("extra network", "mainnet:\n", "devnet:\n  transaction_stream_endpoint: https://grpc.devnet.aptoslabs.com\n  versions_to_import:\n    3: devnet_three\nmainnet:\n", "only testnet and mainnet"),
            ("renamed network", "mainnet:\n", "main:\n", "only testnet and mainnet"),
            ("committed key", "api_key: TESTNET_API_KEY", "api_key: real-key", "testnet api_key must be the placeholder"),
            ("missing key", "  api_key: MAINNET_API_KEY\n", "", "mainnet api_key must be the placeholder"),
            ("plain HTTP", "https://grpc.testnet", "http://grpc.testnet", "testnet endpoint must be https://grpc.testnet.aptoslabs.com"),
            ("foreign host", "grpc.mainnet.aptoslabs.com", "grpc.mainnet.example.com", "mainnet endpoint must be https://grpc.mainnet.aptoslabs.com"),
            ("userinfo host", "grpc.testnet.aptoslabs.com:443", "grpc.testnet.aptoslabs.com@evil.example", "testnet endpoint must be"),
            ("empty versions", "    1: testnet_one\n", "    {}\n", "testnet versions_to_import must be non-empty"),
            ("path traversal", "testnet_one", "../testnet_one", "must be a plain file name"),
            ("json extension collision", "    1: testnet_one\n", "    1: testnet_one\n    3: testnet_one.json\n", "is duplicated"),
            ("other extension collision", "    1: testnet_one\n", "    1: testnet_one\n    3: testnet_one.txt\n", "is duplicated"),
            ("cross-network collision", "mainnet_two", "testnet_one", "is duplicated"),
        ];
        for (case, from, to, expected) in cases {
            assert!(
                VALID_CONFIG.contains(from),
                "{case}: fixture does not contain {from:?}"
            );
            assert_error(
                validate(&VALID_CONFIG.replacen(from, to, 1), &[]),
                expected,
                case,
            );
        }
    }

    #[test]
    fn validate_rejects_unsafe_script_fixtures() {
        let script = |output_name: &str, sender: &str| {
            format!("transactions:\n  - script_path: script\n    output_name: {output_name}\n    sender_address: {sender}\n")
        };
        let same_a = script("same", "a");
        let same_b = script("same", "b");
        let same_twice = format!(
            "{same_a}  - script_path: other\n    output_name: same.json\n    sender_address: a\n"
        );
        let escape = script("../escape", "a");
        let two_a = script("two", "a");
        let cases: [(&str, Files, &str); 5] = [
            (
                "duplicate across files",
                &[("a.yaml", same_a.as_str()), ("b.yaml", same_b.as_str())],
                "is duplicated",
            ),
            (
                "json extension collision",
                &[("a.yaml", same_twice.as_str())],
                "is duplicated",
            ),
            (
                "path traversal",
                &[("a.yaml", escape.as_str())],
                "must be a plain file name",
            ),
            (
                "empty file",
                &[("a.yaml", "transactions: []\n")],
                "No transactions found",
            ),
            (
                "shared sender",
                &[("a.yaml", same_a.as_str()), ("b.yaml", two_a.as_str())],
                "already being used",
            ),
        ];
        for (case, fixtures, expected) in cases {
            assert_error(validate(VALID_CONFIG, fixtures), expected, case);
        }
    }
}
