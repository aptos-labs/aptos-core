// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    adapter_common::{PreprocessedTransaction, VMAdapter},
    aptos_vm::AptosVM,
    data_cache::RemoteStorage,
    logging::AdapterLogSchema,
    parallel_executor::{storage_wrapper::VersionedView, DiemTransactionOutput},
};
use aptos_logger::prelude::*;
use aptos_parallel_executor::{
    executor::MVHashMapView,
    task::{ExecutionStatus, ExecutorTask},
};
use aptos_state_view::StateView;
use aptos_types::{
    access_path::AccessPath, account_config::DIEM_ACCOUNT_MODULE, write_set::WriteOp,
};
use move_core_types::vm_status::VMStatus;

pub(crate) struct DiemVMWrapper<'a, S> {
    vm: AptosVM,
    base_view: &'a S,
}

impl<'a, S: 'a + StateView> ExecutorTask for DiemVMWrapper<'a, S> {
    type T = PreprocessedTransaction;
    type Output = DiemTransactionOutput;
    type Error = VMStatus;
    type Argument = &'a S;

    fn init(argument: &'a S) -> Self {
        let vm = AptosVM::new(argument);

        // Loading `0x1::DiemAccount` and its transitive dependency into the code cache.
        //
        // This should give us a warm VM to avoid the overhead of VM cold start.
        // Result of this load could be omitted as this is a best effort approach and won't hurt if that fails.
        //
        // Loading up `0x1::DiemAccount` should be sufficient as this is the most common module
        // used for prologue, epilogue and transfer functionality.

        let _ = vm.load_module(&DIEM_ACCOUNT_MODULE, &RemoteStorage::new(argument));

        Self {
            vm,
            base_view: argument,
        }
    }

    fn execute_transaction(
        &self,
        view: &MVHashMapView<AccessPath, WriteOp>,
        txn: &PreprocessedTransaction,
    ) -> ExecutionStatus<DiemTransactionOutput, VMStatus> {
        let log_context = AdapterLogSchema::new(self.base_view.id(), view.txn_idx());
        let versioned_view = VersionedView::new_view(self.base_view, view);

        match self
            .vm
            .execute_single_transaction(txn, &versioned_view, &log_context)
        {
            Ok((vm_status, output, sender)) => {
                if output.status().is_discarded() {
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
                if AptosVM::should_restart_execution(&output) {
                    ExecutionStatus::SkipRest(DiemTransactionOutput::new(output))
                } else {
                    ExecutionStatus::Success(DiemTransactionOutput::new(output))
                }
            }
            Err(err) => ExecutionStatus::Abort(err),
        }
    }
}
