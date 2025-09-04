// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::config::{
    config_sanitizer::ConfigSanitizer, node_config_loader::NodeType, Error, NodeConfig,
};
use velor_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetbenchConfig {
    pub enabled: bool,
    pub max_network_channel_size: u64, // Max num of pending network messages
    pub netbench_service_threads: Option<usize>, // Number of kernel threads for tokio runtime. None default for num-cores.

    pub enable_direct_send_testing: bool, // Whether or not to enable direct send test mode
    pub direct_send_data_size: usize,     // The amount of data to send in each request
    pub direct_send_per_second: u64,      // The interval (microseconds) between requests

    pub enable_rpc_testing: bool,
    pub rpc_data_size: usize,
    pub rpc_per_second: u64,
    pub rpc_in_flight: usize,
}

impl Default for NetbenchConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_network_channel_size: 1000,
            netbench_service_threads: Some(2),

            enable_direct_send_testing: false,
            direct_send_data_size: 100 * 1024, // 100 KB
            direct_send_per_second: 1_000,

            enable_rpc_testing: false,
            rpc_data_size: 100 * 1024, // 100 KB
            rpc_per_second: 1_000,
            rpc_in_flight: 8,
        }
    }
}

impl ConfigSanitizer for NetbenchConfig {
    fn sanitize(
        node_config: &NodeConfig,
        _node_type: NodeType,
        chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        let sanitizer_name = Self::get_sanitizer_name();

        // If no netbench config is specified, there's nothing to do
        if node_config.netbench.is_none() {
            return Ok(());
        }

        // If netbench is disabled, there's nothing to do
        let netbench_config = node_config.netbench.unwrap();
        if !netbench_config.enabled {
            return Ok(());
        }

        // Otherwise, verify that netbench is not enabled in testnet or mainnet
        if let Some(chain_id) = chain_id {
            if chain_id.is_testnet() || chain_id.is_mainnet() {
                return Err(Error::ConfigSanitizerFailed(
                    sanitizer_name,
                    "The netbench application should not be enabled in testnet or mainnet!"
                        .to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sanitize_config() {
        // Create a netbench config with the application enabled
        let node_config = NodeConfig {
            netbench: Some(NetbenchConfig {
                enabled: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        // Verify that the config fails sanitization (for testnet)
        let error =
            NetbenchConfig::sanitize(&node_config, NodeType::Validator, Some(ChainId::testnet()))
                .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));

        // Verify that the config fails sanitization (for mainnet)
        let error =
            NetbenchConfig::sanitize(&node_config, NodeType::Validator, Some(ChainId::mainnet()))
                .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));

        // Verify that the config passes sanitization (for an unknown network)
        NetbenchConfig::sanitize(&node_config, NodeType::Validator, None).unwrap();
    }
}
