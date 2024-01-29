// Copyright © Aptos Foundation

use crate::on_chain_config::BlockGasLimitType;
use serde::{Deserialize, Serialize};

/// Local, per-node configuration.
#[derive(Clone, Debug)]
pub struct BlockExecutorLocalConfig {
    pub concurrency_level: usize,
}

/// Configuration from on-chain configuration, that is
/// required to be the same across all nodes.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BlockExecutorConfigFromOnchain {
    pub block_gas_limit_type: BlockGasLimitType,
}

impl BlockExecutorConfigFromOnchain {
    pub fn new_no_block_limit() -> Self {
        Self {
            block_gas_limit_type: BlockGasLimitType::NoLimit,
        }
    }

    pub fn new_maybe_block_limit(maybe_block_gas_limit: Option<u64>) -> Self {
        Self {
            block_gas_limit_type: maybe_block_gas_limit
                .map_or(BlockGasLimitType::NoLimit, BlockGasLimitType::Limit),
        }
    }

    pub const fn on_but_large_for_test() -> Self {
        Self {
            block_gas_limit_type:
                // present, so code is excercised, but large to not limit blocks
                BlockGasLimitType::ComplexLimitV1 {
                    effective_block_gas_limit: 1_000_000_000,
                    execution_gas_effective_multiplier: 1,
                    io_gas_effective_multiplier: 1,
                    block_output_limit: Some(1_000_000_000_000),
                    conflict_penalty_window: 8,
                    use_module_publishing_block_conflict: true,
                    include_user_txn_size_in_block_output: true,
                    add_block_limit_outcome_onchain: false,
                    use_granular_resource_group_conflicts: false,
                },
        }
    }
}

/// Configuration for the BlockExecutor.
#[derive(Clone, Debug)]
pub struct BlockExecutorConfig {
    /// Local, per-node configuration.
    pub local: BlockExecutorLocalConfig,
    /// Configuration from on-chain configuration, that is
    /// required to be the same across all nodes.
    pub onchain: BlockExecutorConfigFromOnchain,
}

impl BlockExecutorConfig {
    pub fn new_no_block_limit(concurrency_level: usize) -> Self {
        Self {
            local: BlockExecutorLocalConfig { concurrency_level },
            onchain: BlockExecutorConfigFromOnchain::new_no_block_limit(),
        }
    }

    pub fn new_maybe_block_limit(
        concurrency_level: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> Self {
        Self {
            local: BlockExecutorLocalConfig { concurrency_level },
            onchain: BlockExecutorConfigFromOnchain::new_maybe_block_limit(maybe_block_gas_limit),
        }
    }
}
