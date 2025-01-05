// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    diff::TransactionDiff,
    state_view::{ReadSet, ReadSetCapturingStateView},
    workload::Workload,
};
use aptos_logger::error;
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

/// Block execution config used for replay benchmarking.
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
        // Execute transactions without overrides.
        let state_view_without_override =
            ReadSetCapturingStateView::new(state_view, HashMap::new());
        let onchain_outputs = execute_workload(
            &AptosVMBlockExecutor::new(),
            &workload,
            &state_view_without_override,
            1,
        );
        let _onchain_inputs = state_view_without_override.into_read_set();

        // Check on-chain outputs do not modify the state we override. If so, benchmarking results
        // may not be correct.
        let begin = workload
            .transaction_slice_metadata()
            .begin_version()
            .expect("Transaction metadata must be a chunk");
        for (idx, on_chain_output) in onchain_outputs.iter().enumerate() {
            for (state_key, _) in on_chain_output.write_set() {
                if state_override.contains_key(state_key) {
                    error!(
                        "Transaction {} writes to overridden state value for {:?}",
                        begin + idx as Version,
                        state_key
                    );
                }
            }
        }

        // Execute transactions with an override.
        let state_view_with_override = ReadSetCapturingStateView::new(state_view, state_override);
        let outputs = execute_workload(
            &AptosVMBlockExecutor::new(),
            &workload,
            &state_view_with_override,
            1,
        );
        let inputs = state_view_with_override.into_read_set();

        // Compute the differences between outputs.
        // TODO: We can also compute the differences between the read sets. Maybe we should add it?
        let diffs = onchain_outputs
            .into_iter()
            .zip(outputs)
            .map(|(onchain_output, output)| TransactionDiff::from_outputs(onchain_output, output))
            .collect();

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
    pub(crate) fn run(&self, executor: &AptosVMBlockExecutor, concurrency_level: usize) {
        execute_workload(executor, &self.workload, &self.inputs, concurrency_level);
    }
}

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
