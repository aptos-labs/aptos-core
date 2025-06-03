// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{
        config_sanitizer::ConfigSanitizer, gas_estimation_config::GasEstimationConfig,
        node_config_loader::NodeType, Error, NodeConfig, MAX_RECEIVING_BLOCK_TXNS,
    },
    utils,
};
use aptos_transactions_filter::transaction_matcher::Filter;
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ApiConfig {
    /// Enables the REST API endpoint
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Address for the REST API to listen on. Set to 0.0.0.0:port to allow all inbound connections.
    pub address: SocketAddr,
    /// Path to a local TLS certificate to enable HTTPS
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_cert_path: Option<String>,
    /// Path to a local TLS key to enable HTTPS
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_key_path: Option<String>,
    /// A maximum limit to the body of a POST request in bytes
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_length_limit: Option<u64>,
    /// Enables failpoints for error testing
    #[serde(default = "default_disabled")]
    pub failpoints_enabled: bool,
    /// Enables JSON output of APIs that support it
    #[serde(default = "default_enabled")]
    pub json_output_enabled: bool,
    /// Enables BCS output of APIs that support it
    #[serde(default = "default_enabled")]
    pub bcs_output_enabled: bool,
    /// Enables compression middleware for API responses
    #[serde(default = "default_enabled")]
    pub compression_enabled: bool,
    /// Enables encode submission API
    #[serde(default = "default_enabled")]
    pub encode_submission_enabled: bool,
    /// Enables transaction submission APIs
    #[serde(default = "default_enabled")]
    pub transaction_submission_enabled: bool,
    /// Enables transaction simulation
    #[serde(default = "default_enabled")]
    pub transaction_simulation_enabled: bool,
    /// Maximum number of transactions that can be sent with the Batch submit API
    pub max_submit_transaction_batch_size: usize,
    /// Maximum page size for transaction paginated APIs
    pub max_transactions_page_size: u16,
    /// Maximum page size for block transaction APIs
    pub max_block_transactions_page_size: u16,
    /// Maximum page size for event paginated APIs
    pub max_events_page_size: u16,
    /// Maximum page size for resource paginated APIs
    pub max_account_resources_page_size: u16,
    /// Maximum page size for module paginated APIs
    pub max_account_modules_page_size: u16,
    /// Maximum gas unit limit for view functions
    ///
    /// This limits the execution length of a view function to the given gas used.
    pub max_gas_view_function: u64,
    /// Optional: Maximum number of worker threads for the API.
    ///
    /// If not set, `runtime_worker_multiplier` will multiply times the number of CPU cores on the machine
    pub max_runtime_workers: Option<usize>,
    /// Multiplier for number of worker threads with number of CPU cores
    ///
    /// If `max_runtime_workers` is set, this is ignored
    pub runtime_worker_multiplier: usize,
    /// Configs for computing unit gas price estimation
    pub gas_estimation: GasEstimationConfig,
    /// Periodically call gas estimation
    pub periodic_gas_estimation_ms: Option<u64>,
    /// Configuration to filter simulation requests.
    pub simulation_filter: Filter,
    /// Configuration to filter view function requests.
    pub view_filter: ViewFilter,
    /// Periodically log stats for view function and simulate transaction usage
    pub periodic_function_stats_sec: Option<u64>,
    /// The time wait_by_hash will wait before returning 404.
    pub wait_by_hash_timeout_ms: u64,
    /// The interval at which wait_by_hash will poll the storage for the transaction.
    pub wait_by_hash_poll_interval_ms: u64,
    /// The number of active wait_by_hash requests that can be active at any given time.
    pub wait_by_hash_max_active_connections: usize,
}

const DEFAULT_ADDRESS: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8080;
const DEFAULT_REQUEST_CONTENT_LENGTH_LIMIT: u64 = 8 * 1024 * 1024; // 8 MB
pub const DEFAULT_MAX_SUBMIT_TRANSACTION_BATCH_SIZE: usize = 10;
pub const DEFAULT_MAX_PAGE_SIZE: u16 = 100;
const DEFAULT_MAX_ACCOUNT_RESOURCES_PAGE_SIZE: u16 = 9999;
const DEFAULT_MAX_ACCOUNT_MODULES_PAGE_SIZE: u16 = 9999;
const DEFAULT_MAX_VIEW_GAS: u64 = 2_000_000; // We keep this value the same as the max number of gas allowed for one single transaction defined in aptos-gas.

fn default_enabled() -> bool {
    true
}

fn default_disabled() -> bool {
    false
}

impl Default for ApiConfig {
    fn default() -> ApiConfig {
        ApiConfig {
            enabled: default_enabled(),
            address: format!("{}:{}", DEFAULT_ADDRESS, DEFAULT_PORT)
                .parse()
                .unwrap(),
            tls_cert_path: None,
            tls_key_path: None,
            content_length_limit: None,
            failpoints_enabled: default_disabled(),
            bcs_output_enabled: default_enabled(),
            json_output_enabled: default_enabled(),
            compression_enabled: default_enabled(),
            encode_submission_enabled: default_enabled(),
            transaction_submission_enabled: default_enabled(),
            transaction_simulation_enabled: default_enabled(),
            max_submit_transaction_batch_size: DEFAULT_MAX_SUBMIT_TRANSACTION_BATCH_SIZE,
            max_block_transactions_page_size: *MAX_RECEIVING_BLOCK_TXNS as u16,
            max_transactions_page_size: DEFAULT_MAX_PAGE_SIZE,
            max_events_page_size: DEFAULT_MAX_PAGE_SIZE,
            max_account_resources_page_size: DEFAULT_MAX_ACCOUNT_RESOURCES_PAGE_SIZE,
            max_account_modules_page_size: DEFAULT_MAX_ACCOUNT_MODULES_PAGE_SIZE,
            max_gas_view_function: DEFAULT_MAX_VIEW_GAS,
            max_runtime_workers: None,
            runtime_worker_multiplier: 2,
            gas_estimation: GasEstimationConfig::default(),
            periodic_gas_estimation_ms: Some(30_000),
            simulation_filter: Filter::default(),
            view_filter: ViewFilter::default(),
            periodic_function_stats_sec: Some(60),
            wait_by_hash_timeout_ms: 1_000,
            wait_by_hash_poll_interval_ms: 20,
            wait_by_hash_max_active_connections: 100,
        }
    }
}

impl ApiConfig {
    pub fn randomize_ports(&mut self) {
        self.address.set_port(utils::get_available_port());
    }

    pub fn content_length_limit(&self) -> u64 {
        match self.content_length_limit {
            Some(v) => v,
            None => DEFAULT_REQUEST_CONTENT_LENGTH_LIMIT,
        }
    }
}

impl ConfigSanitizer for ApiConfig {
    fn sanitize(
        node_config: &NodeConfig,
        node_type: NodeType,
        chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        let sanitizer_name = Self::get_sanitizer_name();
        let api_config = &node_config.api;

        // If the API is disabled, we don't need to do anything
        if !api_config.enabled {
            return Ok(());
        }

        // Verify that failpoints are not enabled in mainnet
        if let Some(chain_id) = chain_id {
            if chain_id.is_mainnet() && api_config.failpoints_enabled {
                return Err(Error::ConfigSanitizerFailed(
                    sanitizer_name,
                    "Failpoints are not supported on mainnet nodes!".into(),
                ));
            }
        }

        // Validate basic runtime properties
        if api_config.max_runtime_workers.is_none() && api_config.runtime_worker_multiplier == 0 {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "runtime_worker_multiplier must be greater than 0!".into(),
            ));
        }

        // Sanitize the gas estimation config
        GasEstimationConfig::sanitize(node_config, node_type, chain_id)?;

        Ok(())
    }
}

// This is necessary because we can't import the EntryFunctionId type from the API types.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ViewFunctionId {
    pub address: AccountAddress,
    pub module: String,
    pub function_name: String,
}

// We just accept Strings here because we can't import EntryFunctionId. We sanitize
// the values later.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewFilter {
    /// Allowlist of functions. If a function is not found here, the API will refuse to
    /// service the view / simulation request.
    Allowlist(Vec<ViewFunctionId>),
    /// Blocklist of functions. If a function is found here, the API will refuse to
    /// service the view / simulation request.
    Blocklist(Vec<ViewFunctionId>),
}

impl Default for ViewFilter {
    fn default() -> Self {
        ViewFilter::Blocklist(vec![])
    }
}

impl ViewFilter {
    /// Returns true if the given function is allowed by the filter.
    pub fn allows(&self, address: &AccountAddress, module: &str, function: &str) -> bool {
        match self {
            ViewFilter::Allowlist(ids) => ids.iter().any(|id| {
                &id.address == address && id.module == module && id.function_name == function
            }),
            ViewFilter::Blocklist(ids) => !ids.iter().any(|id| {
                &id.address == address && id.module == module && id.function_name == function
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_disabled_api() {
        // Create a node config with the API disabled
        let node_config = NodeConfig {
            api: ApiConfig {
                enabled: false,
                failpoints_enabled: true,
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it succeeds
        ApiConfig::sanitize(&node_config, NodeType::Validator, Some(ChainId::mainnet())).unwrap();
    }

    #[test]
    fn test_sanitize_failpoints_on_mainnet() {
        // Create a node config with failpoints enabled
        let node_config = NodeConfig {
            api: ApiConfig {
                enabled: true,
                failpoints_enabled: true,
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails because
        // failpoints are not supported on mainnet.
        let error =
            ApiConfig::sanitize(&node_config, NodeType::Validator, Some(ChainId::mainnet()))
                .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));

        // Sanitize the config for a different network and verify that it succeeds
        ApiConfig::sanitize(&node_config, NodeType::Validator, Some(ChainId::testnet())).unwrap();

        // Sanitize the config for an unknown network and verify that it succeeds
        ApiConfig::sanitize(&node_config, NodeType::Validator, None).unwrap();
    }

    #[test]
    fn test_sanitize_invalid_workers() {
        // Create a node config with failpoints enabled
        let node_config = NodeConfig {
            api: ApiConfig {
                enabled: true,
                max_runtime_workers: None,
                runtime_worker_multiplier: 0,
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails because
        // the runtime worker multiplier is invalid.
        let error =
            ApiConfig::sanitize(&node_config, NodeType::Validator, Some(ChainId::mainnet()))
                .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }
}
