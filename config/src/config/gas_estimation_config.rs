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
        }
    }
}

impl ConfigSanitizer for GasEstimationConfig {
    fn sanitize(
        node_config: &mut NodeConfig,
        _node_type: NodeType,
        _chain_id: ChainId,
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
