// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    diff::{TransactionDiff, TransactionDiffBuilder},
    execution::execute_workload,
    overrides::OverrideConfig,
    state_view::{ReadSet, ReadSetCapturingStateView},
    workload::{TransactionBlock, Workload},
};
use aptos_logger::error;
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_types::transaction::Version;
use aptos_vm::{aptos_vm::AptosVMBlockExecutor, move_vm_ext::flush_warm_vm_cache, VMBlockExecutor};
use std::{
    collections::HashMap,
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
    /// Generates a sequence of inputs (pre-block states) for benchmarking. Also, returns a vector
    /// of output diffs when transactions are executed with these inputs (recall that inputs may be
    /// different from on-chain due to overrides).
    pub(crate) async fn generate(
        debugger: AptosDebugger,
        override_config: OverrideConfig,
        txn_blocks: Vec<TransactionBlock>,
    ) -> anyhow::Result<(Vec<ReadSet>, Vec<Vec<TransactionDiff>>)> {
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
                    let (inputs, diffs) = generator.generate_inputs_with_diffs(txn_block);
                    let time = start_time.elapsed().as_secs();
                    println!(
                        "Generated inputs and computed diffs for block {}/{} in {}s",
                        num_generated.fetch_add(1, Ordering::SeqCst) + 1,
                        num_blocks,
                        time
                    );
                    (inputs, diffs)
                }
            });
            tasks.push(task);
        }

        let mut all_inputs = Vec::with_capacity(tasks.len());
        let mut all_diffs = Vec::with_capacity(tasks.len());
        for task in tasks {
            let (inputs, diffs) = task.await?;
            all_inputs.push(inputs);
            all_diffs.push(diffs)
        }

        Ok((all_inputs, all_diffs))
    }

    /// Generates a pre-block for a single block of transactions.
    ///
    /// Transactions are first executed on top of an actual state. Then, transactions are executed
    /// in top of an overridden state. The outputs of two executions are compared, and the diffs
    /// for each transaction are returned together with the inputs.
    ///
    /// Note: transaction execution is sequential, so that multiple inputs can be constructed in
    /// parallel.
    fn generate_inputs_with_diffs(
        &self,
        txn_block: TransactionBlock,
    ) -> (ReadSet, Vec<TransactionDiff>) {
        let state_view = self.debugger.state_view_at_version(txn_block.begin_version);
        let state_override = self.override_config.get_state_override(&state_view);
        let workload = Workload::from(txn_block);

        // First, we execute transactions without overrides. Flush warm VM cache to ensure read-set
        // contains modules used to warming up the VM.
        flush_warm_vm_cache();
        let state_view_without_override =
            ReadSetCapturingStateView::new(&state_view, HashMap::new());
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
            .transaction_slice_metadata
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

        // Execute transactions with an override. Again, flush the warm VM cache to capture all
        // reads.
        flush_warm_vm_cache();
        let state_view_with_override = ReadSetCapturingStateView::new(&state_view, state_override);
        let outputs = execute_workload(
            &AptosVMBlockExecutor::new(),
            &workload,
            &state_view_with_override,
            1,
        );
        let inputs = state_view_with_override.into_read_set();

        // Compute the differences between outputs.
        let diff_builder = TransactionDiffBuilder::new();
        let diffs = onchain_outputs
            .into_iter()
            .zip(outputs)
            .map(|(onchain_output, output)| diff_builder.build_from_outputs(onchain_output, output))
            .collect();

        (inputs, diffs)
    }
}
