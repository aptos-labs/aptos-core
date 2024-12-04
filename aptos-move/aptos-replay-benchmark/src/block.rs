// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    diff::TransactionDiff,
    state_view::{ReadSet, ReadSetCapturingStateView},
    workload::Workload,
};
use aptos_logger::warn;
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

/// Represents a single benchmarking unit: a block of transactions with the input pre-block state.
/// Also stores the comparison of outputs based on the input state to on-chain outputs (recall that
/// input state may contain an override and differ from on-chain pre-block state).
pub struct Block {
    /// Stores all data needed to execute this block.
    inputs: ReadSet,
    /// Stores transactions to execute and benchmark.
    workload: Workload,
    /// Stores diff results for each transaction output. The number of diffs is always equal to the
    /// number of transactions, but they may or may not be empty.
    diffs: Vec<TransactionDiff>,
}

impl Block {
    /// Creates a new block for benchmarking by executing transactions on top of an overridden
    /// state. If there are any state overrides, transactions are first executed based on the
    /// on-chain state for later comparison (otherwise, if there are no overrides diffs are empty).
    ///
    /// Note: transaction execution is sequential, so that multiple blocks can be constructed in
    /// parallel.
    pub(crate) fn new(
        workload: Workload,
        state_view: &(impl StateView + Sync),
        state_override: HashMap<StateKey, StateValue>,
    ) -> Self {
        // Execute transactions, recording all reads.
        let capturing_state_view =
            ReadSetCapturingStateView::new(state_view, state_override.clone());
        let outputs = execute_workload(
            &AptosVMBlockExecutor::new(),
            &workload,
            &capturing_state_view,
            1,
        );
        let inputs = capturing_state_view.into_read_set();

        let diffs = if state_override.is_empty() {
            // No overrides, skip executing on top of the on-chain state, use empty diffs.
            (0..outputs.len())
                .map(|_| TransactionDiff::empty())
                .collect()
        } else {
            // Execute transactions with on-chain configs.
            let onchain_outputs =
                execute_workload(&AptosVMBlockExecutor::new(), &workload, state_view, 1);

            // Construct diffs and check on-chain outputs do not modify state we override. If so,
            // benchmarking results may not be correct.
            let begin = workload
                .transaction_slice_metadata()
                .begin_version()
                .expect("Transaction metadata must be a chunk");
            let mut diffs = Vec::with_capacity(onchain_outputs.len());
            for (idx, (on_chain_output, new_output)) in
                onchain_outputs.into_iter().zip(outputs).enumerate()
            {
                for (state_key, _) in on_chain_output.write_set() {
                    if state_override.contains_key(state_key) {
                        warn!(
                            "Transaction {} writes to overridden state value for {:?}",
                            begin + idx as Version,
                            state_key
                        );
                    }
                }
                diffs.push(TransactionDiff::from_outputs(on_chain_output, new_output));
            }
            diffs
        };

        Self {
            inputs,
            workload,
            diffs,
        }
    }

    /// Prints the difference in transaction outputs when running with overrides.
    pub fn print_diffs(&self) {
        let begin = self
            .workload
            .transaction_slice_metadata()
            .begin_version()
            .expect("Transaction metadata is a chunk");
        for (idx, diff) in self.diffs.iter().enumerate() {
            if !diff.is_empty() {
                println!("Transaction {} diff:\n {}\n", begin + idx as Version, diff);
            }
        }
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
