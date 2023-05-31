// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::OnChainConfig;
use anyhow::{format_err, Result};
use serde::{Deserialize, Serialize};

/// The on-chain execution config, in order to be able to add fields, we use enum to wrap the actual struct.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum OnChainExecutionConfig {
    V1(ExecutionConfigV1),
    V2(ExecutionConfigV2),
    V3(ExecutionConfigV3),
}

/// The public interface that exposes all values with safe fallback.
impl OnChainExecutionConfig {
    /// The type of the transaction shuffler being used.
    pub fn transaction_shuffler_type(&self) -> TransactionShufflerType {
        match &self {
            OnChainExecutionConfig::V1(config) => config.transaction_shuffler_type.clone(),
            OnChainExecutionConfig::V2(config) => config.transaction_shuffler_type.clone(),
            OnChainExecutionConfig::V3(config) => config.transaction_shuffler_type.clone(),
        }
    }

    /// The per-block gas limit being used.
    pub fn block_gas_limit(&self) -> Option<u64> {
        match &self {
            OnChainExecutionConfig::V1(_config) => None,
            OnChainExecutionConfig::V2(config) => config.block_gas_limit,
            OnChainExecutionConfig::V3(config) => config.block_gas_limit,
        }
    }

    /// The type of the transaction deduper being used.
    pub fn transaction_deduper_type(&self) -> TransactionDeduperType {
        match &self {
            OnChainExecutionConfig::V1(_config) => TransactionDeduperType::NoDedup,
            OnChainExecutionConfig::V2(_config) => TransactionDeduperType::NoDedup,
            OnChainExecutionConfig::V3(config) => config.transaction_deduper_type.clone(),
        }
    }
}

/// This is used when on-chain config is not initialized.
impl Default for OnChainExecutionConfig {
    fn default() -> Self {
        OnChainExecutionConfig::V1(ExecutionConfigV1::default())
    }
}

impl OnChainConfig for OnChainExecutionConfig {
    const MODULE_IDENTIFIER: &'static str = "execution_config";
    const TYPE_IDENTIFIER: &'static str = "ExecutionConfig";

    /// The Move resource is
    /// ```ignore
    /// struct AptosExecutionConfig has copy, drop, store {
    ///    config: vector<u8>,
    /// }
    /// ```
    /// so we need two rounds of bcs deserilization to turn it back to OnChainExecutionConfig
    fn deserialize_into_config(bytes: &[u8]) -> Result<Self> {
        let raw_bytes: Vec<u8> = bcs::from_bytes(bytes)?;
        bcs::from_bytes(&raw_bytes)
            .map_err(|e| format_err!("[on-chain config] Failed to deserialize into config: {}", e))
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ExecutionConfigV1 {
    pub transaction_shuffler_type: TransactionShufflerType,
}

impl Default for ExecutionConfigV1 {
    fn default() -> Self {
        Self {
            transaction_shuffler_type: TransactionShufflerType::NoShuffling,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ExecutionConfigV2 {
    pub transaction_shuffler_type: TransactionShufflerType,
    pub block_gas_limit: Option<u64>,
}

impl Default for ExecutionConfigV2 {
    fn default() -> Self {
        Self {
            transaction_shuffler_type: TransactionShufflerType::NoShuffling,
            block_gas_limit: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ExecutionConfigV3 {
    pub transaction_shuffler_type: TransactionShufflerType,
    pub block_gas_limit: Option<u64>,
    pub transaction_deduper_type: TransactionDeduperType,
}

impl Default for ExecutionConfigV3 {
    fn default() -> Self {
        Self {
            transaction_shuffler_type: TransactionShufflerType::NoShuffling,
            block_gas_limit: None,
            transaction_deduper_type: TransactionDeduperType::NoDedup,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")] // cannot use tag = "type" as nested enums cannot work, and bcs doesn't support it
pub enum TransactionShufflerType {
    NoShuffling,
    SenderAwareV1(u32),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")] // cannot use tag = "type" as nested enums cannot work, and bcs doesn't support it
pub enum TransactionDeduperType {
    NoDedup,
    TxnHashAndAuthenticatorV1,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::on_chain_config::OnChainConfigPayload;
    use rand::Rng;
    use std::{collections::HashMap, sync::Arc};

    #[test]
    fn test_config_yaml_serialization() {
        let config = OnChainExecutionConfig::default();
        let s = serde_yaml::to_string(&config).unwrap();

        serde_yaml::from_str::<OnChainExecutionConfig>(&s).unwrap();
    }

    #[test]
    fn test_config_bcs_serialization() {
        let config = OnChainExecutionConfig::default();
        let s = bcs::to_bytes(&config).unwrap();

        bcs::from_bytes::<OnChainExecutionConfig>(&s).unwrap();
    }

    #[test]
    fn test_config_serialization() {
        let config = OnChainExecutionConfig::V1(ExecutionConfigV1 {
            transaction_shuffler_type: TransactionShufflerType::SenderAwareV1(32),
        });

        let s = serde_yaml::to_string(&config).unwrap();
        let result = serde_yaml::from_str::<OnChainExecutionConfig>(&s).unwrap();
        assert!(matches!(
            result.transaction_shuffler_type(),
            TransactionShufflerType::SenderAwareV1(32)
        ));

        // V2 test with random per-block gas limit
        let rand_gas_limit = rand::thread_rng().gen_range(0, 1000000) as u64;
        let config = OnChainExecutionConfig::V2(ExecutionConfigV2 {
            transaction_shuffler_type: TransactionShufflerType::SenderAwareV1(32),
            block_gas_limit: Some(rand_gas_limit),
        });

        let s = serde_yaml::to_string(&config).unwrap();
        let result = serde_yaml::from_str::<OnChainExecutionConfig>(&s).unwrap();
        assert!(matches!(
            result.transaction_shuffler_type(),
            TransactionShufflerType::SenderAwareV1(32)
        ));
        assert!(result.block_gas_limit() == Some(rand_gas_limit));

        // V2 test with no per-block gas limit
        let config = OnChainExecutionConfig::V2(ExecutionConfigV2 {
            transaction_shuffler_type: TransactionShufflerType::SenderAwareV1(32),
            block_gas_limit: None,
        });

        let s = serde_yaml::to_string(&config).unwrap();
        let result = serde_yaml::from_str::<OnChainExecutionConfig>(&s).unwrap();
        assert!(matches!(
            result.transaction_shuffler_type(),
            TransactionShufflerType::SenderAwareV1(32)
        ));
        assert!(matches!(result.block_gas_limit(), None));
    }

    #[test]
    fn test_config_onchain_payload() {
        let execution_config = OnChainExecutionConfig::V1(ExecutionConfigV1 {
            transaction_shuffler_type: TransactionShufflerType::SenderAwareV1(32),
        });

        let mut configs = HashMap::new();
        configs.insert(
            OnChainExecutionConfig::CONFIG_ID,
            // Requires double serialization, check deserialize_into_config for more details
            bcs::to_bytes(&bcs::to_bytes(&execution_config).unwrap()).unwrap(),
        );

        let payload = OnChainConfigPayload::new(1, Arc::new(configs));

        let result: OnChainExecutionConfig = payload.get().unwrap();
        assert!(matches!(
            result.transaction_shuffler_type(),
            TransactionShufflerType::SenderAwareV1(32)
        ));

        // V2 test with random per-block gas limit
        let rand_gas_limit = rand::thread_rng().gen_range(0, 1000000) as u64;
        let execution_config = OnChainExecutionConfig::V2(ExecutionConfigV2 {
            transaction_shuffler_type: TransactionShufflerType::SenderAwareV1(32),
            block_gas_limit: Some(rand_gas_limit),
        });

        let mut configs = HashMap::new();
        configs.insert(
            OnChainExecutionConfig::CONFIG_ID,
            // Requires double serialization, check deserialize_into_config for more details
            bcs::to_bytes(&bcs::to_bytes(&execution_config).unwrap()).unwrap(),
        );

        let payload = OnChainConfigPayload::new(1, Arc::new(configs));

        let result: OnChainExecutionConfig = payload.get().unwrap();
        assert!(matches!(
            result.transaction_shuffler_type(),
            TransactionShufflerType::SenderAwareV1(32)
        ));
        assert!(result.block_gas_limit() == Some(rand_gas_limit));

        // V2 test with no per-block gas limit
        let execution_config = OnChainExecutionConfig::V2(ExecutionConfigV2 {
            transaction_shuffler_type: TransactionShufflerType::SenderAwareV1(32),
            block_gas_limit: None,
        });

        let mut configs = HashMap::new();
        configs.insert(
            OnChainExecutionConfig::CONFIG_ID,
            // Requires double serialization, check deserialize_into_config for more details
            bcs::to_bytes(&bcs::to_bytes(&execution_config).unwrap()).unwrap(),
        );

        let payload = OnChainConfigPayload::new(1, Arc::new(configs));

        let result: OnChainExecutionConfig = payload.get().unwrap();
        assert!(matches!(
            result.transaction_shuffler_type(),
            TransactionShufflerType::SenderAwareV1(32)
        ));
        assert!(matches!(result.block_gas_limit(), None));
    }
}
