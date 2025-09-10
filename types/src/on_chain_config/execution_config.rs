// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_executor::config::BlockExecutorConfigFromOnchain, on_chain_config::OnChainConfig,
};
use anyhow::{format_err, Result};
use serde::{Deserialize, Serialize};

/// The on-chain execution config, in order to be able to add fields, we use enum to wrap the actual struct.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum OnChainExecutionConfig {
    V1(ExecutionConfigV1),
    V2(ExecutionConfigV2),
    V3(ExecutionConfigV3),
    /// To maintain backwards compatibility on replay, we must ensure that any new features resolve
    /// to previous behavior (before OnChainExecutionConfig was registered) in case of Missing.
    Missing,
    // Reminder: Add V4 and future versions here, after Missing (order matters for enums).
    V4(ExecutionConfigV4),
    V5(ExecutionConfigV5),
    V6(ExecutionConfigV6),
    V7(ExecutionConfigV7),
}

/// The public interface that exposes all values with safe fallback.
impl OnChainExecutionConfig {
    /// The type of the transaction shuffler being used.
    pub fn transaction_shuffler_type(&self) -> TransactionShufflerType {
        match &self {
            OnChainExecutionConfig::Missing => TransactionShufflerType::NoShuffling,
            OnChainExecutionConfig::V1(config) => config.transaction_shuffler_type.clone(),
            OnChainExecutionConfig::V2(config) => config.transaction_shuffler_type.clone(),
            OnChainExecutionConfig::V3(config) => config.transaction_shuffler_type.clone(),
            OnChainExecutionConfig::V4(config) => config.transaction_shuffler_type.clone(),
            OnChainExecutionConfig::V5(config) => config.transaction_shuffler_type.clone(),
            OnChainExecutionConfig::V6(config) => config.transaction_shuffler_type.clone(),
            OnChainExecutionConfig::V7(config) => config.transaction_shuffler_type.clone(),
        }
    }

    /// The per-block gas limit being used.
    pub fn block_gas_limit_type(&self) -> BlockGasLimitType {
        match &self {
            OnChainExecutionConfig::Missing => BlockGasLimitType::NoLimit,
            OnChainExecutionConfig::V1(_config) => BlockGasLimitType::NoLimit,
            OnChainExecutionConfig::V2(config) => config
                .block_gas_limit
                .map_or(BlockGasLimitType::NoLimit, BlockGasLimitType::Limit),
            OnChainExecutionConfig::V3(config) => config
                .block_gas_limit
                .map_or(BlockGasLimitType::NoLimit, BlockGasLimitType::Limit),
            OnChainExecutionConfig::V4(config) => config.block_gas_limit_type.clone(),
            OnChainExecutionConfig::V5(config) => config.block_gas_limit_type.clone(),
            OnChainExecutionConfig::V6(config) => config.block_gas_limit_type.clone(),
            OnChainExecutionConfig::V7(config) => config.block_gas_limit_type.clone(),
        }
    }

    pub fn enable_per_block_gas_limit(&self) -> bool {
        match &self {
            OnChainExecutionConfig::Missing
            | OnChainExecutionConfig::V1(_)
            | OnChainExecutionConfig::V2(_)
            | OnChainExecutionConfig::V3(_)
            | OnChainExecutionConfig::V4(_) => false,
            OnChainExecutionConfig::V5(config) => config.enable_per_block_gas_limit,
            OnChainExecutionConfig::V6(config) => config.enable_per_block_gas_limit,
            OnChainExecutionConfig::V7(config) => config.enable_per_block_gas_limit,
        }
    }

    pub fn gas_price_to_burn(&self) -> Option<u64> {
        match self {
            OnChainExecutionConfig::Missing
            | OnChainExecutionConfig::V1(_)
            | OnChainExecutionConfig::V2(_)
            | OnChainExecutionConfig::V3(_)
            | OnChainExecutionConfig::V4(_)
            | OnChainExecutionConfig::V5(_) => None,
            OnChainExecutionConfig::V6(config) => Some(config.gas_price_to_burn),
            OnChainExecutionConfig::V7(config) => Some(config.gas_price_to_burn),
        }
    }

    pub fn persisted_auxiliary_info_version(&self) -> u8 {
        match self {
            OnChainExecutionConfig::Missing
            | OnChainExecutionConfig::V1(_)
            | OnChainExecutionConfig::V2(_)
            | OnChainExecutionConfig::V3(_)
            | OnChainExecutionConfig::V4(_)
            | OnChainExecutionConfig::V5(_)
            | OnChainExecutionConfig::V6(_) => 0,
            OnChainExecutionConfig::V7(config) => config.persisted_auxiliary_info_version,
        }
    }

    pub fn block_executor_onchain_config(&self) -> BlockExecutorConfigFromOnchain {
        BlockExecutorConfigFromOnchain::new(
            self.block_gas_limit_type(),
            self.enable_per_block_gas_limit(),
            self.gas_price_to_burn(),
        )
    }

    /// The type of the transaction deduper being used.
    pub fn transaction_deduper_type(&self) -> TransactionDeduperType {
        match &self {
            // Note, this behavior was enabled before OnChainExecutionConfig was registered.
            OnChainExecutionConfig::Missing => TransactionDeduperType::TxnHashAndAuthenticatorV1,
            OnChainExecutionConfig::V1(_config) => TransactionDeduperType::NoDedup,
            OnChainExecutionConfig::V2(_config) => TransactionDeduperType::NoDedup,
            OnChainExecutionConfig::V3(config) => config.transaction_deduper_type.clone(),
            OnChainExecutionConfig::V4(config) => config.transaction_deduper_type.clone(),
            OnChainExecutionConfig::V5(config) => config.transaction_deduper_type.clone(),
            OnChainExecutionConfig::V6(config) => config.transaction_deduper_type.clone(),
            OnChainExecutionConfig::V7(config) => config.transaction_deduper_type.clone(),
        }
    }

    /// The default values to use for new networks, e.g., devnet, forge.
    /// Features that are ready for deployment can be enabled here.
    pub fn default_for_genesis() -> Self {
        OnChainExecutionConfig::V7(ExecutionConfigV7 {
            transaction_shuffler_type: TransactionShufflerType::default_for_genesis(),
            block_gas_limit_type: BlockGasLimitType::default_for_genesis(),
            enable_per_block_gas_limit: false,
            transaction_deduper_type: TransactionDeduperType::TxnHashAndAuthenticatorV1,
            gas_price_to_burn: 90,
            persisted_auxiliary_info_version: 1,
        })
    }

    /// The default values to use when on-chain config is not initialized.
    /// This value should not be changed, for replay purposes.
    pub fn default_if_missing() -> Self {
        OnChainExecutionConfig::Missing
    }
}

impl BlockGasLimitType {
    pub fn default_for_genesis() -> Self {
        BlockGasLimitType::ComplexLimitV1 {
            effective_block_gas_limit: 20000,
            execution_gas_effective_multiplier: 1,
            io_gas_effective_multiplier: 1,
            conflict_penalty_window: 9,
            use_granular_resource_group_conflicts: false,
            use_module_publishing_block_conflict: true,
            block_output_limit: Some(4 * 1024 * 1024),
            include_user_txn_size_in_block_output: true,
            add_block_limit_outcome_onchain: true,
        }
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

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ExecutionConfigV2 {
    pub transaction_shuffler_type: TransactionShufflerType,
    pub block_gas_limit: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ExecutionConfigV3 {
    pub transaction_shuffler_type: TransactionShufflerType,
    pub block_gas_limit: Option<u64>,
    pub transaction_deduper_type: TransactionDeduperType,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ExecutionConfigV4 {
    pub transaction_shuffler_type: TransactionShufflerType,
    pub block_gas_limit_type: BlockGasLimitType,
    pub transaction_deduper_type: TransactionDeduperType,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ExecutionConfigV5 {
    pub transaction_shuffler_type: TransactionShufflerType,
    pub block_gas_limit_type: BlockGasLimitType,
    pub enable_per_block_gas_limit: bool,
    pub transaction_deduper_type: TransactionDeduperType,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ExecutionConfigV6 {
    pub transaction_shuffler_type: TransactionShufflerType,
    pub block_gas_limit_type: BlockGasLimitType,
    pub enable_per_block_gas_limit: bool,
    pub transaction_deduper_type: TransactionDeduperType,
    pub gas_price_to_burn: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ExecutionConfigV7 {
    pub transaction_shuffler_type: TransactionShufflerType,
    pub block_gas_limit_type: BlockGasLimitType,
    pub enable_per_block_gas_limit: bool,
    pub transaction_deduper_type: TransactionDeduperType,
    pub gas_price_to_burn: u64,
    pub persisted_auxiliary_info_version: u8,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")] // cannot use tag = "type" as nested enums cannot work, and bcs doesn't support it
pub enum TransactionShufflerType {
    NoShuffling,
    DeprecatedSenderAwareV1(u32),
    SenderAwareV2(u32),
    DeprecatedFairness,
    UseCaseAware {
        sender_spread_factor: usize,
        platform_use_case_spread_factor: usize,
        user_use_case_spread_factor: usize,
    },
}

impl TransactionShufflerType {
    pub fn default_for_genesis() -> Self {
        TransactionShufflerType::UseCaseAware {
            sender_spread_factor: 32,
            platform_use_case_spread_factor: 0,
            user_use_case_spread_factor: 4,
        }
    }

    pub fn user_use_case_spread_factor(&self) -> Option<usize> {
        match self {
            TransactionShufflerType::NoShuffling
            | TransactionShufflerType::DeprecatedSenderAwareV1(_)
            | TransactionShufflerType::SenderAwareV2(_)
            | TransactionShufflerType::DeprecatedFairness => None,
            TransactionShufflerType::UseCaseAware {
                user_use_case_spread_factor,
                ..
            } => Some(*user_use_case_spread_factor),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")] // cannot use tag = "type" as nested enums cannot work, and bcs doesn't support it
pub enum TransactionDeduperType {
    NoDedup,
    TxnHashAndAuthenticatorV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")] // cannot use tag = "type" as nested enums cannot work, and bcs doesn't support it
pub enum BlockGasLimitType {
    NoLimit,
    Limit(u64),
    /// Provides two separate block limits:
    /// 1. effective_block_gas_limit
    /// 2. block_output_limit
    ComplexLimitV1 {
        /// Formula for effective block gas limit:
        /// effective_block_gas_limit <
        /// (execution_gas_effective_multiplier * execution_gas_used +
        ///  io_gas_effective_multiplier * io_gas_used
        /// ) * (1 + num conflicts in conflict_penalty_window)
        effective_block_gas_limit: u64,
        execution_gas_effective_multiplier: u64,
        io_gas_effective_multiplier: u64,
        conflict_penalty_window: u32,

        /// If true we look at granular resource group conflicts (i.e. if same Tag
        /// within a resource group has a conflict)
        /// If false, we treat any conclicts inside of resource groups (even across
        /// non-overlapping tags) as conflicts).
        use_granular_resource_group_conflicts: bool,
        /// Module publishing today fallbacks to sequential execution,
        /// even though there is no read-write conflict.
        /// When enabled, this flag allows us to account for that conflict.
        /// NOTE: Currently not supported.
        use_module_publishing_block_conflict: bool,

        /// Block limit on the total (approximate) txn output size in bytes.
        block_output_limit: Option<u64>,
        /// When set, we include the user txn size in the approximate computation
        /// of block output size, which is compared against the block_output_limit above.
        include_user_txn_size_in_block_output: bool,

        /// When set, we create BlockEpilogue (instead of StateCheckpint) transaction,
        /// which contains BlockEndInfo
        /// NOTE: Currently not supported.
        add_block_limit_outcome_onchain: bool,
    },
}

impl BlockGasLimitType {
    pub fn block_gas_limit(&self) -> Option<u64> {
        match self {
            BlockGasLimitType::NoLimit => None,
            BlockGasLimitType::Limit(limit) => Some(*limit),
            BlockGasLimitType::ComplexLimitV1 {
                effective_block_gas_limit,
                ..
            } => Some(*effective_block_gas_limit),
        }
    }

    pub fn execution_gas_effective_multiplier(&self) -> u64 {
        match self {
            BlockGasLimitType::NoLimit => 1,
            BlockGasLimitType::Limit(_) => 1,
            BlockGasLimitType::ComplexLimitV1 {
                execution_gas_effective_multiplier,
                ..
            } => *execution_gas_effective_multiplier,
        }
    }

    pub fn io_gas_effective_multiplier(&self) -> u64 {
        match self {
            BlockGasLimitType::NoLimit => 1,
            BlockGasLimitType::Limit(_) => 1,
            BlockGasLimitType::ComplexLimitV1 {
                io_gas_effective_multiplier,
                ..
            } => *io_gas_effective_multiplier,
        }
    }

    pub fn block_output_limit(&self) -> Option<u64> {
        match self {
            BlockGasLimitType::NoLimit => None,
            BlockGasLimitType::Limit(_) => None,
            BlockGasLimitType::ComplexLimitV1 {
                block_output_limit, ..
            } => *block_output_limit,
        }
    }

    pub fn conflict_penalty_window(&self) -> Option<u32> {
        match self {
            BlockGasLimitType::NoLimit => None,
            BlockGasLimitType::Limit(_) => None,
            BlockGasLimitType::ComplexLimitV1 {
                conflict_penalty_window,
                ..
            } => {
                if *conflict_penalty_window > 1 {
                    Some(*conflict_penalty_window)
                } else {
                    None
                }
            },
        }
    }

    pub fn use_module_publishing_block_conflict(&self) -> bool {
        match self {
            BlockGasLimitType::NoLimit => false,
            BlockGasLimitType::Limit(_) => false,
            BlockGasLimitType::ComplexLimitV1 {
                use_module_publishing_block_conflict,
                ..
            } => *use_module_publishing_block_conflict,
        }
    }

    pub fn include_user_txn_size_in_block_output(&self) -> bool {
        match self {
            BlockGasLimitType::NoLimit => false,
            BlockGasLimitType::Limit(_) => false,
            BlockGasLimitType::ComplexLimitV1 {
                include_user_txn_size_in_block_output,
                ..
            } => *include_user_txn_size_in_block_output,
        }
    }

    pub fn add_block_limit_outcome_onchain(&self) -> bool {
        match self {
            BlockGasLimitType::NoLimit => false,
            BlockGasLimitType::Limit(_) => false,
            BlockGasLimitType::ComplexLimitV1 {
                add_block_limit_outcome_onchain,
                ..
            } => *add_block_limit_outcome_onchain,
        }
    }

    pub fn use_granular_resource_group_conflicts(&self) -> bool {
        match self {
            BlockGasLimitType::NoLimit => false,
            BlockGasLimitType::Limit(_) => false,
            BlockGasLimitType::ComplexLimitV1 {
                use_granular_resource_group_conflicts,
                ..
            } => *use_granular_resource_group_conflicts,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::on_chain_config::{InMemoryOnChainConfig, OnChainConfigPayload};
    use rand::Rng;
    use std::collections::HashMap;

    #[test]
    fn test_config_yaml_serialization() {
        let config = OnChainExecutionConfig::default_for_genesis();
        let s = serde_yaml::to_string(&config).unwrap();

        serde_yaml::from_str::<OnChainExecutionConfig>(&s).unwrap();
    }

    #[test]
    fn test_config_bcs_serialization() {
        let config = OnChainExecutionConfig::default_for_genesis();
        let s = bcs::to_bytes(&config).unwrap();

        bcs::from_bytes::<OnChainExecutionConfig>(&s).unwrap();
    }

    #[test]
    fn test_config_serialization() {
        let config = OnChainExecutionConfig::V1(ExecutionConfigV1 {
            transaction_shuffler_type: TransactionShufflerType::default_for_genesis(),
        });

        let s = serde_yaml::to_string(&config).unwrap();
        let result = serde_yaml::from_str::<OnChainExecutionConfig>(&s).unwrap();
        assert_eq!(
            result.transaction_shuffler_type(),
            TransactionShufflerType::default_for_genesis(),
        );

        // V2 test with random per-block gas limit
        let rand_gas_limit = rand::thread_rng().gen_range(0, 1000000) as u64;
        let config = OnChainExecutionConfig::V2(ExecutionConfigV2 {
            transaction_shuffler_type: TransactionShufflerType::default_for_genesis(),
            block_gas_limit: Some(rand_gas_limit),
        });

        let s = serde_yaml::to_string(&config).unwrap();
        let result = serde_yaml::from_str::<OnChainExecutionConfig>(&s).unwrap();
        assert_eq!(
            result.transaction_shuffler_type(),
            TransactionShufflerType::default_for_genesis(),
        );
        assert_eq!(
            result.block_gas_limit_type(),
            BlockGasLimitType::Limit(rand_gas_limit)
        );

        // V2 test with no per-block gas limit
        let config = OnChainExecutionConfig::V2(ExecutionConfigV2 {
            transaction_shuffler_type: TransactionShufflerType::default_for_genesis(),
            block_gas_limit: None,
        });

        let s = serde_yaml::to_string(&config).unwrap();
        let result = serde_yaml::from_str::<OnChainExecutionConfig>(&s).unwrap();
        assert_eq!(
            result.transaction_shuffler_type(),
            TransactionShufflerType::default_for_genesis(),
        );
        assert_eq!(result.block_gas_limit_type(), BlockGasLimitType::NoLimit);
    }

    #[test]
    fn test_config_onchain_payload() {
        let execution_config = OnChainExecutionConfig::V1(ExecutionConfigV1 {
            transaction_shuffler_type: TransactionShufflerType::UseCaseAware {
                sender_spread_factor: 32,
                platform_use_case_spread_factor: 0,
                user_use_case_spread_factor: 0,
            },
        });

        let mut configs = HashMap::new();
        configs.insert(
            OnChainExecutionConfig::CONFIG_ID,
            // Requires double serialization, check deserialize_into_config for more details
            bcs::to_bytes(&bcs::to_bytes(&execution_config).unwrap()).unwrap(),
        );

        let payload = OnChainConfigPayload::new(1, InMemoryOnChainConfig::new(configs));

        let result: OnChainExecutionConfig = payload.get().unwrap();
        assert!(matches!(
            result.transaction_shuffler_type(),
            TransactionShufflerType::UseCaseAware {
                sender_spread_factor: 32,
                ..
            }
        ));

        // V2 test with random per-block gas limit
        let rand_gas_limit = rand::thread_rng().gen_range(0, 1000000) as u64;
        let execution_config = OnChainExecutionConfig::V2(ExecutionConfigV2 {
            transaction_shuffler_type: TransactionShufflerType::UseCaseAware {
                sender_spread_factor: 32,
                platform_use_case_spread_factor: 0,
                user_use_case_spread_factor: 0,
            },
            block_gas_limit: Some(rand_gas_limit),
        });

        let mut configs = HashMap::new();
        configs.insert(
            OnChainExecutionConfig::CONFIG_ID,
            // Requires double serialization, check deserialize_into_config for more details
            bcs::to_bytes(&bcs::to_bytes(&execution_config).unwrap()).unwrap(),
        );

        let payload = OnChainConfigPayload::new(1, InMemoryOnChainConfig::new(configs));

        let result: OnChainExecutionConfig = payload.get().unwrap();
        assert!(matches!(
            result.transaction_shuffler_type(),
            TransactionShufflerType::UseCaseAware {
                sender_spread_factor: 32,
                ..
            }
        ));
        assert_eq!(
            result.block_gas_limit_type(),
            BlockGasLimitType::Limit(rand_gas_limit)
        );

        // V2 test with no per-block gas limit
        let execution_config = OnChainExecutionConfig::V2(ExecutionConfigV2 {
            transaction_shuffler_type: TransactionShufflerType::UseCaseAware {
                sender_spread_factor: 32,
                platform_use_case_spread_factor: 0,
                user_use_case_spread_factor: 0,
            },
            block_gas_limit: None,
        });

        let mut configs = HashMap::new();
        configs.insert(
            OnChainExecutionConfig::CONFIG_ID,
            // Requires double serialization, check deserialize_into_config for more details
            bcs::to_bytes(&bcs::to_bytes(&execution_config).unwrap()).unwrap(),
        );

        let payload = OnChainConfigPayload::new(1, InMemoryOnChainConfig::new(configs));

        let result: OnChainExecutionConfig = payload.get().unwrap();
        assert!(matches!(
            result.transaction_shuffler_type(),
            TransactionShufflerType::UseCaseAware {
                sender_spread_factor: 32,
                ..
            }
        ));
        assert_eq!(result.block_gas_limit_type(), BlockGasLimitType::NoLimit);
    }
}
