// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Trusted CI checks for PR-controlled indexer transaction fixtures.
//!
//! A pull request controls `imported_transactions.yaml`, `move_fixtures/`, and the
//! checked-in JSON baseline. CI builds this module from the trusted base revision.
//! It parses the PR files with the generator's own config types, so CI checks exactly
//! what the generator loads.
//!
//! `validate` checks the PR-controlled configuration. `materialize` writes the
//! protected API keys into a private copy. `compare` compares generated
//! transactions with the checked-in baseline.

mod compare;
mod materialize;
mod validate;

pub use compare::{compare, Comparison};
pub use materialize::{materialize_config, write_private_file};
pub use validate::validate_config_file;

/// A network that CI imports transactions from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Network {
    Testnet,
    Mainnet,
}

impl Network {
    pub const ALL: [Network; 2] = [Network::Testnet, Network::Mainnet];

    pub fn name(self) -> &'static str {
        match self {
            Network::Testnet => "testnet",
            Network::Mainnet => "mainnet",
        }
    }

    /// The value that the checked-in configuration must hold in `api_key`.
    pub fn placeholder(self) -> &'static str {
        match self {
            Network::Testnet => "TESTNET_API_KEY",
            Network::Mainnet => "MAINNET_API_KEY",
        }
    }

    /// The environment variable that carries the protected API key.
    pub fn secret_env(self) -> &'static str {
        match self {
            Network::Testnet => "TESTNET_API_KEY_VALUE",
            Network::Mainnet => "MAINNET_API_KEY_VALUE",
        }
    }

    /// The only host that may receive this network's API key.
    pub fn endpoint_host(self) -> &'static str {
        match self {
            Network::Testnet => "grpc.testnet.aptoslabs.com",
            Network::Mainnet => "grpc.mainnet.aptoslabs.com",
        }
    }
}

/// Fixtures shared by the submodule tests.
#[cfg(test)]
mod test_utils {
    use super::validate_config_file;
    use crate::config::TransactionImporterConfig;
    use anyhow::Result;
    use std::{
        fs,
        path::{Path, PathBuf},
    };
    use tempfile::TempDir;

    /// Relative path and content of each fixture file.
    pub(super) type Files<'a> = &'a [(&'a str, &'a str)];

    pub(super) const VALID_CONFIG: &str = "\
testnet:
  transaction_stream_endpoint: https://grpc.testnet.aptoslabs.com:443
  api_key: TESTNET_API_KEY
  versions_to_import:
    1: testnet_one
mainnet:
  transaction_stream_endpoint: https://grpc.mainnet.aptoslabs.com:443
  api_key: MAINNET_API_KEY
  versions_to_import:
    2: mainnet_two
";

    pub(super) fn write(root: &Path, relative: &str, content: &str) -> PathBuf {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        path
    }

    pub(super) fn validate(config: &str, fixtures: Files) -> Result<TransactionImporterConfig> {
        let root = TempDir::new().unwrap();
        let path = write(root.path(), "imported_transactions.yaml", config);
        for (name, content) in fixtures {
            write(root.path(), &format!("move_fixtures/{name}"), content);
        }
        validate_config_file(&path)
    }

    pub(super) fn assert_error<T: std::fmt::Debug>(result: Result<T>, expected: &str, case: &str) {
        let error = format!("{:#}", result.expect_err(case));
        assert!(error.contains(expected), "{case}: {error}");
    }
}
