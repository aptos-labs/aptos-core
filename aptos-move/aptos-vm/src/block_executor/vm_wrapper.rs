// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptos_vm::AptosVM, block_executor::AptosTransactionOutput, data_cache::AsMoveResolver,
};
use aptos_block_executor::task::{ExecutionStatus, ExecutorTask};
use aptos_logger::{enabled, Level};
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    access_path::AccessPath,
    state_store::{state_key::StateKey, StateView},
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, Transaction, WriteSetPayload,
    },
};
use aptos_vm_logging::{log_schema::AdapterLogSchema, prelude::*};
use aptos_vm_types::resolver::{AptosExecutable, ExecutorView, ResourceGroupView};
use fail::fail_point;
use move_core_types::{
    language_storage::ModuleId,
    vm_status::{StatusCode, VMStatus},
};
use move_vm_runtime::{Module, ModuleStorage};
use std::sync::Arc;

pub(crate) struct AptosExecutorTask<'a, S> {
    vm: AptosVM,
    base_view: &'a S,
}

struct AptosModuleStorage<'a> {
    view: &'a dyn ExecutorView,
}

impl ModuleStorage for AptosModuleStorage<'_> {
    fn store_module(&self, module_id: &ModuleId, executable: Module) -> Arc<Module> {
        let state_key = StateKey::access_path(AccessPath::from(module_id));

        let executable_module = Arc::new(executable);
        let ret = executable_module.clone();

        self.view
            .store_executable(&state_key, AptosExecutable { executable_module });

        ret
    }

    fn fetch_module(&self, module_id: &ModuleId) -> Option<Arc<Module>> {
        let state_key = StateKey::access_path(AccessPath::from(module_id));
        self.view
            .fetch_executable(&state_key)
            .map(|AptosExecutable { executable_module }| executable_module)
    }
}

impl<'a, S: 'a + StateView + Sync> ExecutorTask for AptosExecutorTask<'a, S> {
    type Argument = &'a S;
    type Error = VMStatus;
    type Executable = AptosExecutable;
    type Output = AptosTransactionOutput;
    type Txn = SignatureVerifiedTransaction;

    fn init(argument: &'a S) -> Self {
        // AptosVM has to be initialized using configs from storage.
        let vm = AptosVM::new(
            &argument.as_move_resolver(),
            /*override_is_delayed_field_optimization_capable=*/ Some(true),
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
        executor_with_group_view: &(impl ExecutorView + ResourceGroupView),
        txn: &SignatureVerifiedTransaction,
        txn_idx: TxnIndex,
    ) -> ExecutionStatus<AptosTransactionOutput, VMStatus> {
        fail_point!("aptos_vm::vm_wrapper::execute_transaction", |_| {
            ExecutionStatus::DelayedFieldsCodeInvariantError("fail points error".into())
        });

        let log_context = AdapterLogSchema::new(self.base_view.id(), txn_idx as usize);
        let resolver = self
            .vm
            .as_move_resolver_with_group_view(executor_with_group_view);
        let _module_storage = AptosModuleStorage {
            view: executor_with_group_view,
        };
        let maybe_module_storage = None;
        // TODO: enable with:
        // let maybe_module_storage = Some(module_storage);
        match self
            .vm
            .execute_single_transaction(txn, &resolver, maybe_module_storage, &log_context)
        {
            Ok((vm_status, vm_output, sender)) => {
                if vm_output.status().is_discarded() {
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
                if vm_status.status_code() == StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR {
                    ExecutionStatus::SpeculativeExecutionAbortError(
                        vm_status.message().cloned().unwrap_or_default(),
                    )
                } else if vm_status.status_code()
                    == StatusCode::DELAYED_MATERIALIZATION_CODE_INVARIANT_ERROR
                {
                    ExecutionStatus::DelayedFieldsCodeInvariantError(
                        vm_status.message().cloned().unwrap_or_default(),
                    )
                } else if AptosVM::should_restart_execution(vm_output.change_set()) {
                    speculative_info!(
                        &log_context,
                        "Reconfiguration occurred: restart required".into()
                    );
                    ExecutionStatus::SkipRest(AptosTransactionOutput::new(vm_output))
                } else {
                    assert!(
                        Self::is_transaction_dynamic_change_set_capable(txn),
                        "DirectWriteSet should always create SkipRest transaction, validate_waypoint_change_set provides this guarantee"
                    );
                    ExecutionStatus::Success(AptosTransactionOutput::new(vm_output))
                }
            },
            // execute_single_transaction only returns an error when transactions that should never fail
            // (BlockMetadataTransaction and GenesisTransaction) return an error themselves.
            Err(err) => {
                if err.status_code() == StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR {
                    ExecutionStatus::SpeculativeExecutionAbortError(
                        err.message().cloned().unwrap_or_default(),
                    )
                } else if err.status_code()
                    == StatusCode::DELAYED_MATERIALIZATION_CODE_INVARIANT_ERROR
                {
                    ExecutionStatus::DelayedFieldsCodeInvariantError(
                        err.message().cloned().unwrap_or_default(),
                    )
                } else {
                    ExecutionStatus::Abort(err)
                }
            },
        }
    }

    fn is_transaction_dynamic_change_set_capable(txn: &Self::Txn) -> bool {
        if txn.is_valid() {
            if let Transaction::GenesisTransaction(WriteSetPayload::Direct(_)) = txn.expect_valid()
            {
                // WriteSetPayload::Direct cannot be handled in mode where delayed_field_optimization or
                // resource_groups_split_in_change_set is enabled.
                return false;
            }
        }
        true
    }
}
