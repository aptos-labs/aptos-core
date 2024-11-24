// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    state_view::{ReadSet, ReadSetCapturingStateView},
    workload::Workload,
};
use anyhow::bail;
use aptos_types::{
    block_executor::config::{
        BlockExecutorConfig, BlockExecutorConfigFromOnchain, BlockExecutorLocalConfig,
        BlockExecutorModuleCacheLocalConfig,
    },
    state_store::{state_key::StateKey, state_value::StateValue, StateView},
    transaction::{TransactionOutput, Version},
};
use aptos_vm::{aptos_vm::AptosVMBlockExecutor, VMBlockExecutor};
use std::collections::HashMap;

/// Config used by benchmarking blocks.
fn block_execution_config(concurrency_level: usize) -> BlockExecutorConfig {
    BlockExecutorConfig {
        local: BlockExecutorLocalConfig {
            concurrency_level,
            allow_fallback: true,
            discard_failed_blocks: false,
            module_cache_config: BlockExecutorModuleCacheLocalConfig::default(),
        },
        // For replay, there is no block limit.
        onchain: BlockExecutorConfigFromOnchain::new_no_block_limit(),
    }
}

pub struct Block {
    inputs: ReadSet,
    workload: Workload,
}

impl Block {
    pub(crate) fn new(
        workload: Workload,
        state_view: &(impl StateView + Sync),
        state_override: HashMap<StateKey, StateValue>,
        concurrency_level: usize,
    ) -> anyhow::Result<Self> {
        // Execute transactions with on-chain configs.
        let onchain_outputs = execute_workload(
            &AptosVMBlockExecutor::new(),
            &workload,
            state_view,
            concurrency_level,
        );

        // Check on-chain outputs do not modify state we override. If so, benchmarking results may
        // not be correct.
        let begin = workload.first_version();
        for (idx, output) in onchain_outputs.iter().enumerate() {
            for (state_key, _) in output.write_set() {
                if state_override.contains_key(state_key) {
                    bail!(
                        "Transaction {} writes to overridden state value for {:?}",
                        begin + idx as Version,
                        state_key
                    );
                }
            }
        }

        // Execute transactions, recording all reads.
        let state_view = ReadSetCapturingStateView::new(state_view, state_override);
        let _outputs = execute_workload(
            &AptosVMBlockExecutor::new(),
            &workload,
            &state_view,
            concurrency_level,
        );
        let inputs = state_view.into_read_set();

        // Check on-chain outputs against new outputs. We want to ensure that changes are minimal
        // so that overrides do not change execution flow too much.
        // Run analysis.
        // TODO

        Ok(Self { inputs, workload })
    }

    /// Executes the workload for benchmarking.
    #[inline(always)]
    pub(crate) fn run(&self, executor: &AptosVMBlockExecutor, concurrency_level: usize) {
        execute_workload(executor, &self.workload, &self.inputs, concurrency_level);
    }
}

#[inline(always)]
fn execute_workload(
    executor: &AptosVMBlockExecutor,
    workload: &Workload,
    state_view: &(impl StateView + Sync),
    concurrency_level: usize,
) -> Vec<TransactionOutput> {
    executor
        .execute_block_with_config(
            workload.txn_provider(),
            state_view,
            block_execution_config(concurrency_level),
            workload.transaction_slice_metadata(),
        )
        .unwrap_or_else(|err| {
            panic!(
                "Block execution should not fail, but returned an error: {:?}",
                err
            )
        })
        .into_transaction_outputs_forced()
}
