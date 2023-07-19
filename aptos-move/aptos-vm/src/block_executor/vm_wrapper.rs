// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    adapter_common::{PreprocessedTransaction, VMAdapter},
    aptos_vm::AptosVM,
    block_executor::AptosTransactionOutput,
};
use aptos_aggregator::delta_change_set::is_aggregator_error;
use aptos_block_executor::task::{ExecutionStatus, ExecutorTask};
use aptos_logger::{enabled, Level};
use aptos_mvhashmap::types::TxnIndex;
use aptos_state_view::StateView;
use aptos_vm_logging::{log_schema::AdapterLogSchema, prelude::*};
use aptos_vm_types::output::VMOutput;
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, CORE_CODE_ADDRESS},
    vm_status::VMStatus,
};

pub(crate) struct AptosExecutorTask<'a, S> {
    vm: AptosVM,
    base_view: &'a S,
}

impl<'a, S: 'a + StateView + Sync> ExecutorTask for AptosExecutorTask<'a, S> {
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
            &vm.as_move_resolver(argument),
        );

        Self {
            vm,
            base_view: argument,
        }
    }

    // This function is called by the BlockExecutor for each transaction it intends
    // to execute (via the ExecutorTask trait).
    fn execute_transaction(
        &self,
        view: &impl StateView,
        txn: &PreprocessedTransaction,
        txn_idx: TxnIndex,
        parallel_execution: bool,
        aggregator_enabled: bool,
    ) -> ExecutionStatus<AptosTransactionOutput, VMStatus> {
        let log_context = AdapterLogSchema::new(self.base_view.id(), txn_idx as usize);
        match self
            .vm
            .execute_single_transaction(txn, &view, &log_context, aggregator_enabled)
        {
            Ok((vm_status, vm_output, sender)) => {
                if !parallel_execution {
                    match vm_output.try_materialize(view) {
                        Ok(materialized_output) => {
                            process_vm_output(vm_status, materialized_output, sender, log_context)
                        },
                        Err(vm_status) => {
                            if !is_aggregator_error(&vm_status) {
                                ExecutionStatus::AggregatorError
                            } else {
                                ExecutionStatus::Abort(vm_status)
                            }
                        },
                    }
                } else {
                    process_vm_output(vm_status, vm_output, sender, log_context)
                }
            },
            Err(vm_status) => {
                if !is_aggregator_error(&vm_status) {
                    ExecutionStatus::AggregatorError
                } else {
                    ExecutionStatus::Abort(vm_status)
                }
            },
        }
    }
}

fn process_vm_output(
    vm_status: VMStatus,
    vm_output: VMOutput,
    sender: Option<String>,
    log_context: AdapterLogSchema,
) -> ExecutionStatus<AptosTransactionOutput, VMStatus> {
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
    if AptosVM::should_restart_execution(&vm_output) {
        speculative_info!(
            &log_context,
            "Reconfiguration occurred: restart required".into()
        );
        ExecutionStatus::SkipRest(AptosTransactionOutput::new(vm_output))
    } else {
        ExecutionStatus::Success(AptosTransactionOutput::new(vm_output))
    }
}
