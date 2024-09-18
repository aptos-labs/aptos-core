// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::config::{
    config_sanitizer::ConfigSanitizer, node_config_loader::NodeType, Error, NodeConfig,
};
use aptos_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GasEstimationStaticOverride {
    pub low: u64,
    pub market: u64,
    pub aggressive: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum GasEstimationMode {
    OnChainEstimation(FromOnChainGasEstimationMode),
    LocalHistory(FromLocalHistoryGasEstimationMode),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct FromOnChainGasEstimationMode {
    /// Number of transactions for blocks to be classified as full for gas estimation
    pub full_block_txns: usize,
    /// Maximum number of blocks read for low gas estimation
    pub low_block_history: usize,
    /// Maximum number of blocks read for market gas estimation
    pub market_block_history: usize,
    /// Maximum number of blocks read for aggressive gas estimation
    pub aggressive_block_history: usize,
}

impl Default for FromOnChainGasEstimationMode {
    fn default() -> Self {
        Self {
            full_block_txns: 250,
            low_block_history: 10,
            market_block_history: 30,
            aggressive_block_history: 120,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct FromLocalHistoryGasEstimationMode {
    pub target_samples: usize,
    pub min_samples_needed: usize,

    pub target_inclusion_latency_s: f64,
    pub prioritized_target_inclusion_latency_s: f64,
}

impl Default for FromLocalHistoryGasEstimationMode {
    fn default() -> Self {
        Self {
            target_samples: 100,
            min_samples_needed: 10,
            target_inclusion_latency_s: 1.0,
            prioritized_target_inclusion_latency_s: 0.7,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct GasEstimationConfig {
    /// A gate for computing GasEstimation. If false, just returns the default.
    pub enabled: bool,
    /// Static values to override. If set, use these values instead of computing a GasEstimation.
    pub static_override: Option<GasEstimationStaticOverride>,

    pub mode: GasEstimationMode,

    /// Time after write when previous value is returned without recomputing
    pub cache_expiration_ms: u64,
}

impl Default for GasEstimationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            static_override: None,
            mode: GasEstimationMode::OnChainEstimation(FromOnChainGasEstimationMode::default()),
            cache_expiration_ms: 500,
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

        match &gas_estimation_config.mode {
            GasEstimationMode::OnChainEstimation(from_onchain) => {
                // Validate aggressive price takes the most history
                if from_onchain.low_block_history > from_onchain.aggressive_block_history
                    || from_onchain.market_block_history > from_onchain.aggressive_block_history
                {
                    return Err(Error::ConfigSanitizerFailed(
                        sanitizer_name,
                        format!(
                            "aggressive block history {} must be > low {}, market {}",
                            from_onchain.aggressive_block_history,
                            from_onchain.low_block_history,
                            from_onchain.market_block_history
                        ),
                    ));
                }

                if from_onchain.low_block_history == 0
                    || from_onchain.market_block_history == 0
                    || from_onchain.aggressive_block_history == 0
                {
                    return Err(Error::ConfigSanitizerFailed(
                        sanitizer_name,
                        format!(
                            "low {}, market {}, aggressive {} block history must be > 0",
                            from_onchain.low_block_history,
                            from_onchain.market_block_history,
                            from_onchain.aggressive_block_history
                        ),
                    ));
                }
            },
            GasEstimationMode::LocalHistory(_from_local_history) => {
                // TODO add sanitize
            },
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
                    mode: GasEstimationMode::OnChainEstimation(FromOnChainGasEstimationMode {
                        low_block_history: 11,
                        aggressive_block_history: 10,
                        ..Default::default()
                    }),
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
                    mode: GasEstimationMode::OnChainEstimation(FromOnChainGasEstimationMode {
                        market_block_history: 31,
                        aggressive_block_history: 30,
                        ..Default::default()
                    }),
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
                    mode: GasEstimationMode::OnChainEstimation(FromOnChainGasEstimationMode {
                        low_block_history: 0,
                        ..Default::default()
                    }),
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
                    mode: GasEstimationMode::OnChainEstimation(FromOnChainGasEstimationMode {
                        market_block_history: 0,
                        ..Default::default()
                    }),
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
                    mode: GasEstimationMode::OnChainEstimation(FromOnChainGasEstimationMode {
                        aggressive_block_history: 0,
                        ..Default::default()
                    }),
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
