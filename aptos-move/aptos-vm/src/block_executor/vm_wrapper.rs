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
    state_store::StateView,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, Transaction, WriteSetPayload,
    },
};
use aptos_vm_logging::{log_schema::AdapterLogSchema, prelude::*};
use aptos_vm_types::resolver::{ExecutorView, ResourceGroupView};
use fail::fail_point;
use move_core_types::vm_status::{StatusCode, VMStatus};

use std::time::{Duration, Instant};

fn spin_wait(time: usize)
{
    let start = Instant::now();
    let duration = Duration::from_micros(time as u64);

    loop {
        if start.elapsed() >= duration {
            break;
        }
    }
}
pub(crate) struct AptosExecutorTask<'a, S> {
    vm: AptosVM,
    base_view: &'a S,
}

impl<'a, S: 'a + StateView + Sync> ExecutorTask for AptosExecutorTask<'a, S> {
    type Argument = &'a S;
    type Error = VMStatus;
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
        let res = match self
            .vm
            .execute_single_transaction(txn, &resolver, &log_context)
        {
            Ok((vm_status, vm_output)) => {
                if vm_output.status().is_discarded() {
                    speculative_trace!(
                        &log_context,
                        format!("Transaction discarded, status: {:?}", vm_status),
                    );
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
        };
        spin_wait(1000);
        res
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
