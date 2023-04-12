// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    adapter_common::{PreprocessedTransaction, VMAdapter},
    aptos_vm::AptosVM,
    block_executor::AptosTransactionOutput,
    data_cache::{AsMoveResolver, StorageAdapter},
};
use aptos_block_executor::task::{ExecutionStatus, ExecutorTask};
use aptos_logger::{enabled, Level};
use aptos_mvhashmap::types::TxnIndex;
use aptos_vm_logging::{log_schema::AdapterLogSchema, prelude::*};
use aptos_vm_types::{
    change_set::DeltaChangeSet, remote_cache::StateViewWithRemoteCache,
    transaction_output::TransactionOutput,
};
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, CORE_CODE_ADDRESS},
    vm_status::VMStatus,
};

pub(crate) struct AptosExecutorTask<'a, S> {
    vm: AptosVM,
    base_view: &'a S,
}

impl<'a, S: 'a + StateViewWithRemoteCache + Sync> ExecutorTask for AptosExecutorTask<'a, S> {
    type Argument = &'a S;
    type Error = VMStatus;
    type Output = AptosTransactionOutput;
    type Txn = PreprocessedTransaction;

    fn init(argument: &'a S) -> Self {
        let vm = AptosVM::new(argument);

        // Loading `0x1::account` and its transitive dependency into the code cache.
        //
        // This should give us a warm VM to avoid the overhead of VM cold start.
        // Result of this load could be omitted as this is a best effort approach and won't hurt if that fails.
        //
        // Loading up `0x1::account` should be sufficient as this is the most common module
        // used for prologue, epilogue and transfer functionality.

        let _ = vm.load_module(
            &ModuleId::new(CORE_CODE_ADDRESS, ident_str!("account").to_owned()),
            &StorageAdapter::new(argument),
        );

        Self {
            vm,
            base_view: argument,
        }
    }

    // This function is called by the BlockExecutor for each transaction is intends
    // to execute (via the ExecutorTask trait). It can be as a part of sequential
    // execution, or speculatively as a part of a parallel execution.
    fn execute_transaction(
        &self,
        view: &impl StateViewWithRemoteCache,
        txn: &PreprocessedTransaction,
        txn_idx: TxnIndex,
        materialize_deltas: bool,
    ) -> ExecutionStatus<AptosTransactionOutput, VMStatus> {
        let log_context = AdapterLogSchema::new(self.base_view.id(), txn_idx as usize);

        match self
            .vm
            .execute_single_transaction(txn, &view.as_move_resolver(), &log_context)
        {
            Ok((vm_status, mut output, sender)) => {
                if materialize_deltas {
                    // TODO: Here we assume materialization does not fail. If it does we have to rerun
                    // the transaction. For now, this is not critical because deltas are used in the
                    // cases when they never fail.
                    let (mut writes, deltas, events, gas_used, status) = output.unpack();
                    let materialized_writes = deltas
                        .try_materialize(view)
                        .expect("failed to apply aggregator delta outputs");

                    // This is guaranteed to succeed because state keys for deltas and writes will be
                    // different, and so merging is equivalent to simply adding more elements to the
                    // change set.
                    writes
                        .merge_writes(materialized_writes)
                        .expect("failed to merge materialized aggregator deltas");

                    // Reconstruct the output but without deltas.
                    output = TransactionOutput::new(
                        writes,
                        DeltaChangeSet::empty(),
                        events,
                        gas_used,
                        status,
                    );
                }

                if output.status().is_discarded() {
                    match sender {
                        Some(s) => speculative_trace!(
                            &log_context,
                            format!(
                                "Transaction discarded, sender: {}, error: {:?}",
                                s, vm_status
                            ),
                        ),
                        None => {
                            speculative_trace!(
                                &log_context,
                                format!("Transaction malformed, error: {:?}", vm_status),
                            )
                        },
                    };
                }
                if AptosVM::should_restart_execution(&output) {
                    speculative_info!(
                        &log_context,
                        "Reconfiguration occurred: restart required".into()
                    );
                    ExecutionStatus::SkipRest(AptosTransactionOutput::new(output))
                } else {
                    ExecutionStatus::Success(AptosTransactionOutput::new(output))
                }
            },
            Err(err) => ExecutionStatus::Abort(err),
        }
    }
}
