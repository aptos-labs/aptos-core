// Copyright © Aptos Foundation

use crate::on_chain_config::BlockGasLimitType;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct BlockExecutorLocalConfig {
    pub concurrency_level: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BlockExecutorOnchainConfig {
    pub block_gas_limit_type: BlockGasLimitType,
}

impl BlockExecutorOnchainConfig {
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

    pub fn has_any_block_gas_limit(&self) -> bool {
        self.block_gas_limit_type.block_gas_limit().is_some()
    }
}

#[derive(Clone, Debug)]
pub struct BlockExecutorConfig {
    pub local: BlockExecutorLocalConfig,
    pub onchain: BlockExecutorOnchainConfig,
}

impl BlockExecutorConfig {
    pub fn new_no_block_limit(concurrency_level: usize) -> Self {
        Self {
            local: BlockExecutorLocalConfig { concurrency_level },
            onchain: BlockExecutorOnchainConfig::new_no_block_limit(),
        }
    }

    pub fn new_maybe_block_limit(
        concurrency_level: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> Self {
        Self {
            local: BlockExecutorLocalConfig { concurrency_level },
            onchain: BlockExecutorOnchainConfig::new_maybe_block_limit(maybe_block_gas_limit),
        }
    }
}
