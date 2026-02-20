// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! REST client and debugger initialization utilities.
//!
//! This module provides convenient functions for creating Aptos REST clients
//! and debuggers with common network configurations.

use anyhow::Result;
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_rest_client::{AptosBaseUrl, Client};
use url::Url;

/// Common network configurations for Aptos clients.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub endpoint: String,
    pub api_key: Option<String>,
}

impl ClientConfig {
    /// Create a new client configuration.
    pub fn new(endpoint: String, api_key: Option<String>) -> Self {
        Self { endpoint, api_key }
    }

    /// Configuration for mainnet.
    pub fn mainnet() -> Self {
        Self {
            endpoint: "https://api.mainnet.aptoslabs.com/v1".to_string(),
            api_key: None,
        }
    }

    /// Configuration for testnet.
    pub fn testnet() -> Self {
        Self {
            endpoint: "https://api.testnet.aptoslabs.com/v1".to_string(),
            api_key: None,
        }
    }

    /// Configuration for devnet.
    pub fn devnet() -> Self {
        Self {
            endpoint: "https://api.devnet.aptoslabs.com/v1".to_string(),
            api_key: None,
        }
    }

    /// Configuration for local node.
    pub fn local() -> Self {
        Self {
            endpoint: "http://localhost:8080/v1".to_string(),
            api_key: None,
        }
    }

    /// Set the API key for this configuration.
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }
}

/// Creates an Aptos REST client from an endpoint URL.
///
/// # Arguments
/// * `endpoint` - The REST API endpoint URL
///
/// # Returns
/// `Ok(Client)` on success, or an error if the URL is invalid
///
/// # Example
/// ```no_run
/// use aptos_move_testing_utils::create_rest_client;
///
/// let client = create_rest_client("https://api.mainnet.aptoslabs.com/v1", None).unwrap();
/// ```
pub fn create_rest_client(endpoint: &str, api_key: Option<String>) -> Result<Client> {
    let url = Url::parse(endpoint)?;
    let builder = Client::builder(AptosBaseUrl::Custom(url));
    let client = if let Some(api_key) = api_key {
        builder.api_key(&api_key)?.build()
    } else {
        builder.build()
    };
    Ok(client)
}

/// Creates an AptosDebugger from a REST endpoint.
///
/// # Arguments
/// * `endpoint` - The REST API endpoint URL
/// * `api_key` - Optional API key for increased rate limits
///
/// # Returns
/// `Ok(AptosDebugger)` on success, or an error if initialization fails
///
/// # Example
/// ```no_run
/// use aptos_move_testing_utils::create_debugger;
///
/// let debugger = create_debugger("https://api.mainnet.aptoslabs.com/v1", None).unwrap();
/// ```
pub fn create_debugger(endpoint: &str, api_key: Option<String>) -> Result<AptosDebugger> {
    let client = create_rest_client(endpoint, api_key)?;
    AptosDebugger::rest_client(client)
}

/// Creates both a REST client and debugger from the same configuration.
///
/// # Arguments
/// * `config` - Client configuration with endpoint and optional API key
///
/// # Returns
/// `Ok((Client, AptosDebugger))` on success, or an error if initialization fails
///
/// # Example
/// ```no_run
/// use aptos_move_testing_utils::{create_client_and_debugger, ClientConfig};
///
/// let config = ClientConfig::mainnet();
/// let (client, debugger) = create_client_and_debugger(&config).unwrap();
/// ```
pub fn create_client_and_debugger(config: &ClientConfig) -> Result<(Client, AptosDebugger)> {
    let client = create_rest_client(&config.endpoint, config.api_key.clone())?;
    let debugger = create_debugger(&config.endpoint, config.api_key.clone())?;
    Ok((client, debugger))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_presets() {
        let mainnet = ClientConfig::mainnet();
        assert_eq!(mainnet.endpoint, "https://api.mainnet.aptoslabs.com/v1");
        assert!(mainnet.api_key.is_none());

        let testnet = ClientConfig::testnet();
        assert_eq!(testnet.endpoint, "https://api.testnet.aptoslabs.com/v1");

        let devnet = ClientConfig::devnet();
        assert_eq!(devnet.endpoint, "https://api.devnet.aptoslabs.com/v1");

        let local = ClientConfig::local();
        assert_eq!(local.endpoint, "http://localhost:8080/v1");
    }

    #[test]
    fn test_client_config_with_api_key() {
        let config = ClientConfig::mainnet().with_api_key("test-key".to_string());
        assert_eq!(config.api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_create_rest_client_valid_url() {
        let result = create_rest_client("https://api.mainnet.aptoslabs.com/v1", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_rest_client_invalid_url() {
        let result = create_rest_client("not-a-valid-url", None);
        assert!(result.is_err());
    }
}
