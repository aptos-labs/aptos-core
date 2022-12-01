// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    adapter_common::{PreprocessedTransaction, VMAdapter},
    aptos_vm::AptosVM,
    block_executor::{storage_wrapper::VersionedView, AptosTransactionOutput},
    data_cache::{AsMoveResolver, StateViewCache, StorageAdapter},
    logging::AdapterLogSchema,
    move_vm_ext::MoveResolverExt,
};
use aptos_aggregator::{delta_change_set::DeltaChangeSet, transaction::TransactionOutputExt};
use aptos_block_executor::{
    executor::MVHashMapView,
    task::{ExecutionStatus, ExecutorTask},
};
use aptos_logger::prelude::*;
use aptos_state_view::StateView;
use aptos_types::{state_store::state_key::StateKey, write_set::WriteOp};
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, CORE_CODE_ADDRESS},
    vm_status::VMStatus,
};
use std::collections::btree_map::BTreeMap;

pub(crate) struct AptosExecutorTask<'a, S> {
    vm: AptosVM,
    base_view: &'a S,
}

// This function is called by the BlockExecutor for each transaction is intends
// to execute (via the ExecutorTask trait). It can be as a part of sequential
// execution, or speculatively as a part of a parallel execution.
fn execute_transaction<S: MoveResolverExt + StateView>(
    vm: &AptosVM,
    txn: &PreprocessedTransaction,
    view: S,
    log_context: AdapterLogSchema,
    materialize_deltas: bool,
) -> ExecutionStatus<AptosTransactionOutput, VMStatus> {
    match vm.execute_single_transaction(txn, &view, &log_context) {
        Ok((vm_status, mut output_ext, sender)) => {
            if materialize_deltas {
                // Keep TransactionOutputExt type for wrapper.
                output_ext = TransactionOutputExt::new(
                    DeltaChangeSet::empty(),                   // Cleared deltas.
                    output_ext.into_transaction_output(&view), // Materialize.
                );
            }

            if output_ext.txn_output().status().is_discarded() {
                match sender {
                    Some(s) => trace!(
                        log_context,
                        "Transaction discarded, sender: {}, error: {:?}",
                        s,
                        vm_status,
                    ),
                    None => {
                        trace!(log_context, "Transaction malformed, error: {:?}", vm_status,)
                    }
                };
            }
            if AptosVM::should_restart_execution(output_ext.txn_output()) {
                info!(log_context, "Reconfiguration occurred: restart required",);
                ExecutionStatus::SkipRest(AptosTransactionOutput::new(output_ext))
            } else {
                ExecutionStatus::Success(AptosTransactionOutput::new(output_ext))
            }
        }
        Err(err) => ExecutionStatus::Abort(err),
    }
}

impl<'a, S: 'a + StateView> ExecutorTask for AptosExecutorTask<'a, S> {
    type T = PreprocessedTransaction;
    type Output = AptosTransactionOutput;
    type Error = VMStatus;
    type Argument = &'a S;

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

    fn execute_transaction_btree_view(
        &self,
        view: &BTreeMap<StateKey, WriteOp>,
        txn: &PreprocessedTransaction,
        txn_idx: usize,
    ) -> ExecutionStatus<AptosTransactionOutput, VMStatus> {
        let state_cache_view = StateViewCache::from_map_ref(self.base_view, view);
        execute_transaction(
            &self.vm,
            txn,
            state_cache_view.as_move_resolver(),
            AdapterLogSchema::new(self.base_view.id(), txn_idx),
            true,
        )
    }

    fn execute_transaction_mvhashmap_view(
        &self,
        view: &MVHashMapView<StateKey, WriteOp>,
        txn: &PreprocessedTransaction,
    ) -> ExecutionStatus<AptosTransactionOutput, VMStatus> {
        execute_transaction(
            &self.vm,
            txn,
            VersionedView::new_view(self.base_view, view),
            AdapterLogSchema::new(self.base_view.id(), view.txn_idx()),
            false,
        )
    }
}
