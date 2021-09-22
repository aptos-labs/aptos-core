// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    adapter_common::{PreprocessedTransaction, VMAdapter},
    diem_vm::DiemVM,
    logging::AdapterLogSchema,
    parallel_executor::{storage_wrapper::VersionedView, DiemTransactionOutput},
};
use diem_logger::prelude::*;
use diem_parallel_executor::{
    executor::MVHashMapView,
    task::{ExecutionStatus, ExecutorTask},
};
use diem_state_view::StateView;
use diem_types::{access_path::AccessPath, write_set::WriteOp};
use move_core_types::vm_status::VMStatus;

pub(crate) struct DiemVMWrapper<'a, S> {
    vm: DiemVM,
    base_view: &'a S,
}

impl<'a, S: 'a + StateView> ExecutorTask for DiemVMWrapper<'a, S> {
    type T = PreprocessedTransaction;
    type Output = DiemTransactionOutput;
    type Error = VMStatus;
    type Argument = &'a S;

    fn init(argument: &'a S) -> Self {
        Self {
            vm: DiemVM::new(argument),
            base_view: argument,
        }
    }

    fn execute_transaction(
        &self,
        view: &MVHashMapView<AccessPath, WriteOp>,
        txn: &PreprocessedTransaction,
    ) -> ExecutionStatus<DiemTransactionOutput, VMStatus> {
        let log_context = AdapterLogSchema::new(self.base_view.id(), view.version());
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
                if DiemVM::should_restart_execution(&output) {
                    ExecutionStatus::SkipRest(DiemTransactionOutput::new(output))
                } else {
                    ExecutionStatus::Success(DiemTransactionOutput::new(output))
                }
            }
            Err(err) => ExecutionStatus::Abort(err),
        }
    }
}
