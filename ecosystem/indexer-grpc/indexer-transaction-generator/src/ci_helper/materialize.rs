// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Materialization of the protected API keys into a private copy of the configuration.

use super::Network;
use crate::config::TransactionImporterConfig;
use anyhow::{Context, Result};
use std::{fs, io::Write, os::unix::fs::OpenOptionsExt, path::Path};

/// Returns the configuration as YAML with each network's `api_key` set to the protected value.
/// The value is set on the typed struct, so it cannot reach any other field.
/// Error messages never contain the value.
pub fn materialize_config(
    mut config: TransactionImporterConfig,
    api_key: impl Fn(Network) -> Option<String>,
) -> Result<String> {
    for network in Network::ALL {
        let value = api_key(network)
            .filter(|value| !value.is_empty() && !value.contains(['\n', '\r']))
            .with_context(|| {
                format!(
                    "missing or invalid protected value for {}",
                    network.secret_env()
                )
            })?;
        config
            .configs
            .get_mut(network.name())
            .context("configuration must be validated before materialization")?
            .api_key = Some(value);
    }
    serde_yaml::to_string(&config).context("failed to serialize the materialized configuration")
}

/// Writes a file that only the owner can read and write (mode 0600). The file must
/// not exist yet, so the mode always applies and a pre-placed symlink is never followed.
pub fn write_private_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .and_then(|mut file| file.write_all(content.as_bytes()))
        .with_context(|| format!("failed to write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ci_helper::test_utils::{validate, VALID_CONFIG};
    use tempfile::TempDir;

    #[test]
    fn materialize_sets_only_the_api_keys() {
        // An anchor on the placeholder must not carry the secret into an output name.
        let config = VALID_CONFIG
            .replacen(
                "api_key: TESTNET_API_KEY",
                "api_key: &key TESTNET_API_KEY",
                1,
            )
            .replacen("1: testnet_one", "1: *key", 1);
        let materialized = materialize_config(validate(&config, &[]).unwrap(), |network| {
            Some(format!("{}-secret", network.name()))
        })
        .unwrap();

        assert_eq!(materialized.matches("testnet-secret").count(), 1);
        assert_eq!(materialized.matches("mainnet-secret").count(), 1);
        let parsed: TransactionImporterConfig = serde_yaml::from_str(&materialized).unwrap();
        let testnet = &parsed.configs["testnet"];
        assert_eq!(testnet.api_key.as_deref(), Some("testnet-secret"));
        assert_eq!(testnet.versions_to_import[&1], "TESTNET_API_KEY");
        assert_eq!(
            testnet.transaction_stream_endpoint.host_str(),
            Some("grpc.testnet.aptoslabs.com")
        );
        assert_eq!(
            parsed.configs["mainnet"].api_key.as_deref(),
            Some("mainnet-secret")
        );
    }

    #[test]
    fn write_private_file_creates_an_owner_only_file() {
        use std::os::unix::fs::PermissionsExt;

        let root = TempDir::new().unwrap();
        let path = root.path().join("out/imported_transactions.yaml");
        write_private_file(&path, "api_key: secret\n").unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "mode is {mode:o}");
        assert_eq!(fs::read_to_string(&path).unwrap(), "api_key: secret\n");
        // An existing file, which could have a wider mode, is never reused.
        assert!(write_private_file(&path, "other").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "api_key: secret\n");
    }

    #[test]
    fn materialize_rejects_missing_or_malformed_values_without_echoing_them() {
        for (case, value) in [
            ("missing", None),
            ("empty", Some("")),
            ("line break", Some("secret\nvalue")),
        ] {
            let config = validate(VALID_CONFIG, &[]).unwrap();
            let result = materialize_config(config, |network| match network {
                Network::Testnet => value.map(str::to_owned),
                Network::Mainnet => Some("mainnet-secret".to_owned()),
            });
            let error = format!("{:#}", result.expect_err(case));
            assert!(error.contains("TESTNET_API_KEY_VALUE"), "{case}: {error}");
            assert!(!error.contains("secret"), "{case}: {error}");
        }
    }
}
