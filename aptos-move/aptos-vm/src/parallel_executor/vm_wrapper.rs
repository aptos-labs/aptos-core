// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    adapter_common::{PreprocessedTransaction, VMAdapter},
    aptos_vm::AptosVM,
    data_cache::StorageAdapter,
    logging::AdapterLogSchema,
    parallel_executor::{storage_wrapper::VersionedView, AptosTransactionOutput},
};
use aptos_logger::prelude::*;
use aptos_parallel_executor::{
    executor::MVHashMapView,
    task::{ExecutionStatus, ExecutorTask},
};
use aptos_state_view::StateView;
use aptos_types::{state_store::state_key::StateKey, write_set::WriteOp};
use move_deps::move_core_types::{
    ident_str,
    language_storage::{ModuleId, CORE_CODE_ADDRESS},
    vm_status::VMStatus,
};

pub(crate) struct AptosVMWrapper<'a, S> {
    vm: AptosVM,
    base_view: &'a S,
}

impl<'a, S: 'a + StateView> ExecutorTask for AptosVMWrapper<'a, S> {
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

    fn execute_transaction(
        &self,
        view: &MVHashMapView<StateKey, WriteOp>,
        txn: &PreprocessedTransaction,
    ) -> ExecutionStatus<AptosTransactionOutput, VMStatus> {
        let log_context = AdapterLogSchema::new(self.base_view.id(), view.txn_idx());
        let versioned_view = VersionedView::new_view(self.base_view, view);

        match self
            .vm
            .execute_single_transaction(txn, &versioned_view, &log_context)
        {
            Ok((vm_status, output_ext, sender)) => {
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
                    ExecutionStatus::SkipRest(AptosTransactionOutput::new(output_ext))
                } else {
                    ExecutionStatus::Success(AptosTransactionOutput::new(output_ext))
                }
            }
            Err(err) => ExecutionStatus::Abort(err),
        }
    }
}
