// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    adapter_common::{PreprocessedTransaction, VMAdapter},
    aptos_vm::AptosVM,
    block_executor::AptosTransactionOutput,
    data_cache::{AsMoveResolver, StorageAdapter},
    logging::AdapterLogSchema,
};
use aptos_aggregator::{delta_change_set::DeltaChangeSet, transaction::TransactionOutputExt};
use aptos_block_executor::task::{ExecutionStatus, ExecutorTask};
use aptos_logger::prelude::*;
use aptos_state_view::StateView;
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, CORE_CODE_ADDRESS},
    vm_status::VMStatus,
};

use std::cell::RefCell;

thread_local!(static CACHE_VM: RefCell<Option<AptosVM>> = RefCell::new(None));

pub(crate) struct AptosExecutorTask<'a, S> {
    base_view: &'a S,
}

impl<'a, S: 'a + StateView> ExecutorTask for AptosExecutorTask<'a, S> {
    type Txn = PreprocessedTransaction;
    type Output = AptosTransactionOutput;
    type Error = VMStatus;
    type Argument = &'a S;

    fn init(argument: &'a S) -> Self {
        CACHE_VM.with(|cell| {
            let borrow = cell.replace(None);
            let vm = if let Some(vm) = borrow {
                AptosVM::new_with_existing_vm(vm, argument)
            } else {
                AptosVM::new(argument)
            };
            cell.replace(Some(vm.clone()));
        });
        Self {
            base_view: argument,
        }
    }

    // This function is called by the BlockExecutor for each transaction is intends
    // to execute (via the ExecutorTask trait). It can be as a part of sequential
    // execution, or speculatively as a part of a parallel execution.
    fn execute_transaction(
        &self,
        view: &impl StateView,
        txn: &PreprocessedTransaction,
        txn_idx: usize,
        materialize_deltas: bool,
    ) -> ExecutionStatus<AptosTransactionOutput, VMStatus> {
        let log_context = AdapterLogSchema::new(self.base_view.id(), txn_idx);
        let vm = CACHE_VM.with(|cell| cell.borrow().as_ref().cloned().unwrap());

        match vm.execute_single_transaction(txn, &view.as_move_resolver(), &log_context) {
            Ok((vm_status, mut output_ext, sender)) => {
                if materialize_deltas {
                    // Keep TransactionOutputExt type for wrapper.
                    output_ext = TransactionOutputExt::new(
                        DeltaChangeSet::empty(),                  // Cleared deltas.
                        output_ext.into_transaction_output(view), // Materialize.
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
}
