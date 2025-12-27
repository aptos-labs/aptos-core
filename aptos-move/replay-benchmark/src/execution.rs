// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::workload::Workload;
use aptos_types::{
    block_executor::config::{
        BlockExecutorConfig, BlockExecutorConfigFromOnchain, BlockExecutorLocalConfig,
        BlockExecutorModuleCacheLocalConfig,
    },
    state_store::StateView,
    transaction::TransactionOutput,
};
use aptos_vm::aptos_vm::AptosVMBlockExecutor;

/// Runs a block of transactions from the workload on top of the specified state (sequentially or
/// in parallel). Block execution should never fail.
pub(crate) fn execute_workload(
    executor: &AptosVMBlockExecutor,
    workload: &Workload,
    state_view: &(impl StateView + Sync),
    concurrency_level: usize,
) -> Vec<TransactionOutput> {
    let config = BlockExecutorConfig {
        local: BlockExecutorLocalConfig {
            blockstm_v2: false,
            concurrency_level,
            allow_fallback: true,
            discard_failed_blocks: false,
            module_cache_config: BlockExecutorModuleCacheLocalConfig::default(),
            enable_pre_write: true,
        },
        // For replay, there is no block limit.
        onchain: BlockExecutorConfigFromOnchain::on_but_large_for_test(),
    };

    executor
        .execute_block_with_config(
            &workload.txn_provider,
            state_view,
            config,
            workload.transaction_slice_metadata,
        )
        .unwrap_or_else(|err| {
            panic!(
                "Block execution should not fail, but returned an error: {:?}",
                err
            )
        })
        .into_transaction_outputs_forced()
}
