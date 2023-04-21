// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{config_sanitizer::ConfigSanitizer, Error, NodeConfig, RoleType},
    utils,
};
use aptos_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ApiConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub address: SocketAddr,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_cert_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_key_path: Option<String>,
    // optional for compatible with old configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_length_limit: Option<u64>,
    #[serde(default = "default_disabled")]
    pub failpoints_enabled: bool,
    #[serde(default = "default_enabled")]
    pub json_output_enabled: bool,
    #[serde(default = "default_enabled")]
    pub bcs_output_enabled: bool,
    #[serde(default = "default_enabled")]
    pub encode_submission_enabled: bool,
    #[serde(default = "default_enabled")]
    pub transaction_submission_enabled: bool,
    #[serde(default = "default_enabled")]
    pub transaction_simulation_enabled: bool,

    pub max_submit_transaction_batch_size: usize,

    // Maximum page size for paginated APIs
    pub max_transactions_page_size: u16,
    pub max_events_page_size: u16,
    pub max_account_resources_page_size: u16,
    pub max_account_modules_page_size: u16,

    /// Max gas unit for view function.
    pub max_gas_view_function: u64,

    // Performance functionality
    pub max_runtime_workers: Option<usize>, // The maximum number of workers to use for the API runtime
    pub runtime_worker_multiplier: usize, // If max_runtime_workers is None, use runtime_worker_multiplier * num CPU cores
}

pub const DEFAULT_ADDRESS: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 8080;
pub const DEFAULT_REQUEST_CONTENT_LENGTH_LIMIT: u64 = 8 * 1024 * 1024; // 8 MB
pub const DEFAULT_MAX_SUBMIT_TRANSACTION_BATCH_SIZE: usize = 10;
pub const DEFAULT_MAX_PAGE_SIZE: u16 = 100;
pub const DEFAULT_MAX_ACCOUNT_RESOURCES_PAGE_SIZE: u16 = 9999;
pub const DEFAULT_MAX_ACCOUNT_MODULES_PAGE_SIZE: u16 = 9999;
pub const DEFAULT_MAX_VIEW_GAS: u64 = 2_000_000; // We keep this value the same as the max number of gas allowed for one single transaction defined in aptos-gas.

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
            encode_submission_enabled: default_enabled(),
            transaction_submission_enabled: default_enabled(),
            transaction_simulation_enabled: default_enabled(),
            max_submit_transaction_batch_size: DEFAULT_MAX_SUBMIT_TRANSACTION_BATCH_SIZE,
            max_transactions_page_size: DEFAULT_MAX_PAGE_SIZE,
            max_events_page_size: DEFAULT_MAX_PAGE_SIZE,
            max_account_resources_page_size: DEFAULT_MAX_ACCOUNT_RESOURCES_PAGE_SIZE,
            max_account_modules_page_size: DEFAULT_MAX_ACCOUNT_MODULES_PAGE_SIZE,
            max_gas_view_function: DEFAULT_MAX_VIEW_GAS,
            max_runtime_workers: None,
            runtime_worker_multiplier: 2,
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
        node_config: &mut NodeConfig,
        _node_role: RoleType,
        chain_id: ChainId,
    ) -> Result<(), Error> {
        let sanitizer_name = Self::get_sanitizer_name();
        let api_config = &node_config.api;

        // If the API is disabled, we don't need to do anything
        if !api_config.enabled {
            return Ok(());
        }

        // Verify that failpoints are not enabled in mainnet
        if chain_id.is_mainnet() && api_config.failpoints_enabled {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "Failpoints are not supported on mainnet nodes!".into(),
            ));
        }

        // Validate basic runtime properties
        if api_config.max_runtime_workers.is_none() && api_config.runtime_worker_multiplier == 0 {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "runtime_worker_multiplier must be greater than 0!".into(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_disabled_api() {
        // Create a node config with the API disabled
        let mut node_config = NodeConfig {
            api: ApiConfig {
                enabled: false,
                failpoints_enabled: true,
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it succeeds
        ApiConfig::sanitize(&mut node_config, RoleType::Validator, ChainId::mainnet()).unwrap();
    }

    #[test]
    fn test_sanitize_failpoints_on_mainnet() {
        // Create a node config with failpoints enabled
        let mut node_config = NodeConfig {
            api: ApiConfig {
                enabled: true,
                failpoints_enabled: true,
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails because
        // failpoints are not supported on mainnet.
        let error = ApiConfig::sanitize(&mut node_config, RoleType::Validator, ChainId::mainnet())
            .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_sanitize_invalid_workers() {
        // Create a node config with failpoints enabled
        let mut node_config = NodeConfig {
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
        let error = ApiConfig::sanitize(&mut node_config, RoleType::Validator, ChainId::mainnet())
            .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }
}
