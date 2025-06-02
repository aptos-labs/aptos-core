// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    execution::execute_workload,
    overrides::OverrideConfig,
    state_view::{ReadSet, ReadSetCapturingStateView},
    workload::{TransactionBlock, Workload},
};
use aptos_logger::error;
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_types::transaction::Version;
use aptos_vm::{aptos_vm::AptosVMBlockExecutor, VMBlockExecutor};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Instant,
};

pub struct InputOutputDiffGenerator {
    debugger: AptosDebugger,
    override_config: OverrideConfig,
}

impl InputOutputDiffGenerator {
    /// Generates a sequence of inputs (pre-block states) for benchmarking or comparison.
    pub(crate) async fn generate(
        debugger: AptosDebugger,
        override_config: OverrideConfig,
        txn_blocks: Vec<TransactionBlock>,
    ) -> anyhow::Result<Vec<ReadSet>> {
        let generator = Arc::new(Self {
            debugger,
            override_config,
        });

        let num_generated = Arc::new(AtomicU64::new(0));
        let num_blocks = txn_blocks.len();

        let mut tasks = Vec::with_capacity(num_blocks);
        for txn_block in txn_blocks {
            let task = tokio::task::spawn_blocking({
                let generator = generator.clone();
                let num_generated = num_generated.clone();
                move || {
                    let start_time = Instant::now();
                    let inputs = generator.generate_inputs(txn_block);
                    let time = start_time.elapsed().as_secs();
                    println!(
                        "Generated inputs for block {}/{} in {}s",
                        num_generated.fetch_add(1, Ordering::SeqCst) + 1,
                        num_blocks,
                        time
                    );
                    inputs
                }
            });
            tasks.push(task);
        }

        let mut all_inputs = Vec::with_capacity(tasks.len());
        for task in tasks {
            all_inputs.push(task.await?);
        }

        Ok(all_inputs)
    }

    /// Generates a pre-block state for a single block of transactions. Transactions are executed
    /// on top of an overridden state.
    ///
    /// Note: transaction execution is sequential, so that multiple inputs can be constructed in
    /// parallel.
    fn generate_inputs(&self, txn_block: TransactionBlock) -> ReadSet {
        let state_view = self.debugger.state_view_at_version(txn_block.begin_version);
        let state_override = self.override_config.get_state_override(&state_view);
        let workload = Workload::from(txn_block);

        // First, we execute transactions without overrides.
        let onchain_outputs =
            execute_workload(&AptosVMBlockExecutor::new(), &workload, &state_view, 1);

        // Check on-chain outputs do not modify the state we override. If so, benchmarking results
        // may not be correct.
        let begin = workload
            .transaction_slice_metadata
            .begin_version()
            .expect("Transaction metadata must be a chunk");
        for (idx, on_chain_output) in onchain_outputs.iter().enumerate() {
            for (state_key, _) in on_chain_output.write_set().write_op_iter() {
                if state_override.contains_key(state_key) {
                    error!(
                        "Transaction {} writes to overridden state value for {:?}",
                        begin + idx as Version,
                        state_key
                    );
                }
            }
        }

        let state_view_with_override = ReadSetCapturingStateView::new(&state_view, state_override);

        // Execute transactions with an override.
        execute_workload(
            &AptosVMBlockExecutor::new(),
            &workload,
            &state_view_with_override,
            1,
        );
        state_view_with_override.into_read_set()
    }
}
