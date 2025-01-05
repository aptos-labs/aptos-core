// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::BlockGasLimitType;
use serde::{Deserialize, Serialize};

/// Local, per-node configurations for module cache. While caches can be persisted across multiple
/// block executions, these configurations allow to specify cache sizes, etc.
#[derive(Clone, Debug)]
pub struct BlockExecutorModuleCacheLocalConfig {
    /// If true, when global caches are empty, Aptos framework is prefetched into module cache.
    pub prefetch_framework_code: bool,
    /// The maximum size of module cache (the sum of serialized sizes of all cached modules in
    /// bytes).
    pub max_module_cache_size_in_bytes: usize,
    /// The maximum size (in terms of entries) of struct name re-indexing map stored in the runtime
    /// environment.
    pub max_struct_name_index_map_num_entries: usize,
}

impl Default for BlockExecutorModuleCacheLocalConfig {
    fn default() -> Self {
        Self {
            prefetch_framework_code: true,
            // Use 1Gb for now, should be large enough to cache all mainnet modules (at the time
            // of writing this comment, 13.11.24).
            max_module_cache_size_in_bytes: 1024 * 1024 * 1024,
            max_struct_name_index_map_num_entries: 1_000_000,
        }
    }
}

/// Local, per-node configuration.
#[derive(Clone, Debug)]
pub struct BlockExecutorLocalConfig {
    pub concurrency_level: usize,
    // If specified, parallel execution fallbacks to sequential, if issue occurs.
    // Otherwise, if there is an error in either of the execution, we will panic.
    pub allow_fallback: bool,
    // If true, we will discard the failed blocks and continue with the next block.
    // (allow_fallback needs to be set)
    pub discard_failed_blocks: bool,
    pub module_cache_config: BlockExecutorModuleCacheLocalConfig,
}

impl BlockExecutorLocalConfig {
    /// Returns a new config with specified concurrency level and:
    ///   - Allowed fallback to sequential execution from parallel.
    ///   - Not allowed discards of failed blocks.
    ///   - Default module cache configs.
    pub fn default_with_concurrency_level(concurrency_level: usize) -> Self {
        Self {
            concurrency_level,
            allow_fallback: true,
            discard_failed_blocks: false,
            module_cache_config: BlockExecutorModuleCacheLocalConfig::default(),
        }
    }
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
                // present, so code is exercised, but large to not limit blocks
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
            local: BlockExecutorLocalConfig::default_with_concurrency_level(concurrency_level),
            onchain: BlockExecutorConfigFromOnchain::new_no_block_limit(),
        }
    }

    pub fn new_maybe_block_limit(
        concurrency_level: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> Self {
        Self {
            local: BlockExecutorLocalConfig::default_with_concurrency_level(concurrency_level),
            onchain: BlockExecutorConfigFromOnchain::new_maybe_block_limit(maybe_block_gas_limit),
        }
    }
}
