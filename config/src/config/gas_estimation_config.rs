// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::config::{
    config_sanitizer::ConfigSanitizer, node_config_loader::NodeType, Error, NodeConfig,
};
use aptos_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct GasEstimationStaticOverride {
    pub low: u64,
    pub market: u64,
    pub aggressive: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct GasEstimationConfig {
    /// A gate for computing GasEstimation. If false, just returns the default.
    pub enabled: bool,
    /// Static values to override. If set, use these values instead of computing a GasEstimation.
    pub static_override: Option<GasEstimationStaticOverride>,
    /// Number of transactions for blocks to be classified as full for gas estimation
    pub full_block_txns: usize,
    /// Maximum number of blocks read for low gas estimation
    pub low_block_history: usize,
    /// Maximum number of blocks read for market gas estimation
    pub market_block_history: usize,
    /// Maximum number of blocks read for aggressive gas estimation
    pub aggressive_block_history: usize,
    /// Time after write when previous value is returned without recomputing
    pub cache_expiration_ms: u64,
    /// Whether to account which TransactionShufflerType is used onchain, and how it affects gas estimation
    pub incorporate_reordering_effects: bool,
}

impl Default for GasEstimationConfig {
    fn default() -> GasEstimationConfig {
        GasEstimationConfig {
            enabled: true,
            static_override: None,
            full_block_txns: 250,
            low_block_history: 10,
            market_block_history: 30,
            aggressive_block_history: 120,
            cache_expiration_ms: 500,
            incorporate_reordering_effects: true,
        }
    }
}

impl ConfigSanitizer for GasEstimationConfig {
    fn sanitize(
        node_config: &NodeConfig,
        _node_type: NodeType,
        _chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        let sanitizer_name = Self::get_sanitizer_name();
        let gas_estimation_config = &node_config.api.gas_estimation;

        // Validate aggressive price takes the most history
        if gas_estimation_config.low_block_history > gas_estimation_config.aggressive_block_history
            || gas_estimation_config.market_block_history
                > gas_estimation_config.aggressive_block_history
        {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                format!(
                    "aggressive block history {} must be > low {}, market {}",
                    gas_estimation_config.aggressive_block_history,
                    gas_estimation_config.low_block_history,
                    gas_estimation_config.market_block_history
                ),
            ));
        }

        if gas_estimation_config.low_block_history == 0
            || gas_estimation_config.market_block_history == 0
            || gas_estimation_config.aggressive_block_history == 0
        {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                format!(
                    "low {}, market {}, aggressive {} block history must be > 0",
                    gas_estimation_config.low_block_history,
                    gas_estimation_config.market_block_history,
                    gas_estimation_config.aggressive_block_history
                ),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiConfig;

    #[test]
    fn test_sanitize_invalid_aggressive_low_block_history() {
        // Create a node config with an aggressive block history that is too low
        let node_config = NodeConfig {
            api: ApiConfig {
                gas_estimation: GasEstimationConfig {
                    low_block_history: 11,
                    aggressive_block_history: 10,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error = GasEstimationConfig::sanitize(
            &node_config,
            NodeType::Validator,
            Some(ChainId::mainnet()),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_sanitize_invalid_aggressive_market_block_history() {
        // Create a node config with an aggressive block history that is too low
        let node_config = NodeConfig {
            api: ApiConfig {
                gas_estimation: GasEstimationConfig {
                    market_block_history: 31,
                    aggressive_block_history: 30,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error = GasEstimationConfig::sanitize(
            &node_config,
            NodeType::Validator,
            Some(ChainId::testnet()),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_sanitize_invalid_zero_low_block_history() {
        // Create a node config with a zero low block history
        let node_config = NodeConfig {
            api: ApiConfig {
                gas_estimation: GasEstimationConfig {
                    low_block_history: 0,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error = GasEstimationConfig::sanitize(
            &node_config,
            NodeType::Validator,
            Some(ChainId::mainnet()),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_sanitize_invalid_zero_market_block_history() {
        // Create a node config with a zero market block history
        let node_config = NodeConfig {
            api: ApiConfig {
                gas_estimation: GasEstimationConfig {
                    market_block_history: 0,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error = GasEstimationConfig::sanitize(
            &node_config,
            NodeType::Validator,
            Some(ChainId::testnet()),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_sanitize_invalid_zero_aggressive_block_history() {
        // Create a node config with a zero aggressive block history
        let node_config = NodeConfig {
            api: ApiConfig {
                gas_estimation: GasEstimationConfig {
                    aggressive_block_history: 0,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error =
            GasEstimationConfig::sanitize(&node_config, NodeType::Validator, None).unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }
}
