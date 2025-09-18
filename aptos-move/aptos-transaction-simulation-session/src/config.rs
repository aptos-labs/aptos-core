// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use serde::{Deserialize, Serialize};
use url::Url;

// TODO: Config versioning?

/// Represents the (optional) base state of a session.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum BaseState {
    /// No base state; the session is entirely local (e.g., for integration tests or synthetic simulations).
    Empty,
    /// The session starts from a remote network state (a "forked state").
    Remote {
        node_url: Url,
        network_version: u64,
        api_key: Option<String>,
    },
}

/// The configuration for a session, stored to a file in the session directory
/// to allow the session to be restored.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Config {
    /// The base state of the session.
    pub base: BaseState,

    /// The number of operations the session has performed.
    pub ops: u64,
}

impl Config {
    /// Creates a new empty configuration.
    pub fn new() -> Self {
        Self {
            base: BaseState::Empty,
            ops: 0,
        }
    }

    /// Creates a configuration with remote base state.
    pub fn with_remote(node_url: Url, network_version: u64, api_key: Option<String>) -> Self {
        Self {
            base: BaseState::Remote {
                node_url,
                network_version,
                api_key,
            },
            ops: 0,
        }
    }

    /// Saves the configuration to a file.
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Loads the configuration from a file.
    pub fn load_from_file(path: &std::path::Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&json)?;
        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

#[test]
fn test_config_roundtrip_empty_base() -> Result<()> {
    let config = Config::new();
    let temp_file = tempfile::NamedTempFile::new()?;

    config.save_to_file(temp_file.path())?;
    let config_loaded = Config::load_from_file(temp_file.path())?;

    assert_eq!(config, config_loaded);

    Ok(())
}

#[test]
fn test_config_roundtrip_remote_base() -> Result<()> {
    let config = Config::with_remote(
        Url::parse("https://fullnode.testnet.aptoslabs.com")?,
        1,
        Some("some_api_key".to_string()),
    );
    let temp_file = tempfile::NamedTempFile::new()?;

    config.save_to_file(temp_file.path())?;
    let config_loaded = Config::load_from_file(temp_file.path())?;

    assert_eq!(config, config_loaded);

    Ok(())
}
