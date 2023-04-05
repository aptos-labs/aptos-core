// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::OnChainConfig;
use anyhow::{format_err, Result};
use serde::{Deserialize, Serialize};

/// The on-chain per-block gas limit config, in order to be able to add fields, we use enum to wrap the actual struct.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum OnChainBlockGasLimitConfig {
    V1(BlockGasLimitConfigV1),
}

/// The public interface that exposes all values with safe fallback.
impl OnChainBlockGasLimitConfig {
    pub fn block_gas_limit_type(&self) -> BlockGasLimitType {
        match &self {
            OnChainBlockGasLimitConfig::V1(config) => config.block_gas_limit_type.clone(),
        }
    }
}

/// This is used when on-chain config is not initialized.
impl Default for OnChainBlockGasLimitConfig {
    fn default() -> Self {
        OnChainBlockGasLimitConfig::V1(BlockGasLimitConfigV1::default())
    }
}

impl OnChainConfig for OnChainBlockGasLimitConfig {
    const MODULE_IDENTIFIER: &'static str = "block_gas_limit_config";
    const TYPE_IDENTIFIER: &'static str = "BlockGasLimitConfig";

    /// The Move resource is
    /// ```ignore
    /// struct AptosBlockGasLimitConfig has copy, drop, store {
    ///    config: vector<u8>,
    /// }
    /// ```
    /// so we need two rounds of bcs deserilization to turn it back to OnChainBlockGasLimitConfig
    fn deserialize_into_config(bytes: &[u8]) -> Result<Self> {
        let raw_bytes: Vec<u8> = bcs::from_bytes(bytes)?;
        bcs::from_bytes(&raw_bytes)
            .map_err(|e| format_err!("[on-chain config] Failed to deserialize into config: {}", e))
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct BlockGasLimitConfigV1 {
    pub block_gas_limit_type: BlockGasLimitType,
}

impl Default for BlockGasLimitConfigV1 {
    fn default() -> Self {
        Self {
            block_gas_limit_type: BlockGasLimitType::NoLimit,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")] // cannot use tag = "type" as nested enums cannot work, and bcs doesn't support it
pub enum BlockGasLimitType {
    NoLimit,
    BlockGasLimitV1(u64),
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::on_chain_config::OnChainConfigPayload;
    use std::{collections::HashMap, sync::Arc};

    #[test]
    fn test_config_yaml_serialization() {
        let config = OnChainBlockGasLimitConfig::default();
        let s = serde_yaml::to_string(&config).unwrap();

        serde_yaml::from_str::<OnChainBlockGasLimitConfig>(&s).unwrap();
    }

    #[test]
    fn test_config_bcs_serialization() {
        let config = OnChainBlockGasLimitConfig::default();
        let s = bcs::to_bytes(&config).unwrap();

        bcs::from_bytes::<OnChainBlockGasLimitConfig>(&s).unwrap();
    }

    #[test]
    fn test_config_serialization() {
        let config = OnChainBlockGasLimitConfig::V1(BlockGasLimitConfigV1 {
            block_gas_limit_type: BlockGasLimitType::BlockGasLimitV1(1000000),
        });

        let s = serde_yaml::to_string(&config).unwrap();
        let result = serde_yaml::from_str::<OnChainBlockGasLimitConfig>(&s).unwrap();
        assert!(matches!(
            result.block_gas_limit_type(),
            BlockGasLimitType::BlockGasLimitV1(1000000)
        ));
    }

    #[test]
    fn test_config_onchain_payload() {
        let block_gas_limit_config = OnChainBlockGasLimitConfig::V1(BlockGasLimitConfigV1 {
            block_gas_limit_type: BlockGasLimitType::BlockGasLimitV1(1000000)
        });

        let mut configs = HashMap::new();
        configs.insert(
            OnChainBlockGasLimitConfig::CONFIG_ID,
            // Requires double serialization, check deserialize_into_config for more details
            bcs::to_bytes(&bcs::to_bytes(&block_gas_limit_config).unwrap()).unwrap(),
        );

        let payload = OnChainConfigPayload::new(1, Arc::new(configs));

        let result: OnChainBlockGasLimitConfig = payload.get().unwrap();
        assert!(matches!(
            result.block_gas_limit_type(),
            BlockGasLimitType::BlockGasLimitV1(1000000)
        ));
    }
}
