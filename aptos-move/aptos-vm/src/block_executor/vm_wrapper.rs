// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptos_vm::AptosVM, block_executor::AptosTransactionOutput, data_cache::AsMoveResolver,
};
use aptos_aggregator::types::code_invariant_error;
use aptos_block_executor::task::{ExecutionStatus, ExecutorTask};
use aptos_logger::{enabled, Level};
use aptos_mvhashmap::types::TxnIndex;
use aptos_state_view::StateView;
use aptos_types::{
    account_config::{BlockLimitReachedEvent, BLOCK_LIMIT_REACHED_EVENT_TYPE_NAME},
    aggregator::PanicError,
    contract_event::ContractEvent,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, Transaction,
        TransactionOutput, TransactionStatus, WriteSetPayload,
    },
    write_set::WriteSet,
};
use aptos_vm_logging::{log_schema::AdapterLogSchema, prelude::*};
use aptos_vm_types::resolver::{ExecutorView, ResourceGroupView};
use move_core_types::{
    language_storage::TypeTag,
    vm_status::{StatusCode, VMStatus},
};
use std::str::FromStr;

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
        let vm = AptosVM::new(&argument.as_move_resolver());

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
        if (executor_with_group_view.is_delayed_field_optimization_capable()
            || executor_with_group_view.is_resource_group_split_in_change_set_capable())
            && !Self::is_transaction_dynamic_change_set_capable(txn)
        {
            return ExecutionStatus::DirectWriteSetTransactionNotCapableError;
        }

        let log_context = AdapterLogSchema::new(self.base_view.id(), txn_idx as usize);
        let resolver = self
            .vm
            .as_move_resolver_with_group_view(executor_with_group_view);
        match self
            .vm
            .execute_single_transaction(txn, &resolver, &log_context)
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
                } else if vm_status.status_code() == StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR
                {
                    ExecutionStatus::DelayedFieldsCodeInvariantError(
                        vm_status.message().cloned().unwrap_or_default(),
                    )
                } else if AptosVM::should_restart_execution(&vm_output) {
                    speculative_info!(
                        &log_context,
                        "Reconfiguration occurred: restart required".into()
                    );
                    ExecutionStatus::SkipRest(AptosTransactionOutput::new(vm_output))
                } else {
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
                } else if err.status_code() == StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR {
                    ExecutionStatus::DelayedFieldsCodeInvariantError(
                        err.message().cloned().unwrap_or_default(),
                    )
                } else {
                    ExecutionStatus::Abort(err)
                }
            },
        }
    }

    fn execute_skipped_checkpoint(
        txn: &Self::Txn,
        output: &mut Self::Output,
        block_limit_reached_event: Option<BlockLimitReachedEvent>,
    ) -> Result<(), PanicError> {
        if txn.is_valid() {
            match txn.expect_valid() {
                Transaction::StateCheckpoint(_) => {
                    aptos_block_executor::task::TransactionOutput::set_txn_output_for_non_dynamic_change_set(output);
                    let committed_output = output.committed_output();
                    if !committed_output.status().is_retry() {
                        return Err(code_invariant_error(format!(
                            "Block limit cannot be reached on StateCheckpoint transaction, as it is free and empty. {:?}",
                            committed_output,
                        )));
                    }

                    let status = TransactionStatus::Keep(aptos_types::transaction::ExecutionStatus::Success);
                    let events = block_limit_reached_event
                        .map(|event| {
                            Ok(vec![ContractEvent::new_v2(
                                TypeTag::from_str(BLOCK_LIMIT_REACHED_EVENT_TYPE_NAME).map_err(|e| code_invariant_error(format!("Failed to parse type tag: {:?}", e)))?,
                                bcs::to_bytes(&event).map_err(|e| code_invariant_error(format!("Failed to serialize event: {:?}", e)))?,
                            )])
                        })
                        .transpose()?
                        .unwrap_or_default();
                    *output = AptosTransactionOutput::new_from_committed_output(TransactionOutput::new(WriteSet::default(), events, 0, status));
                    Ok(())
                },
                Transaction::GenesisTransaction(_) => {
                    // Genesis transaction is allowed to be last/only transaction in a block.
                    Ok(())
                },
                valid_txn => {
                    Err(code_invariant_error(format!(
                        "Last transaction in a block where limit is reached is not StateCheckpoint, but: {}",
                        valid_txn.type_name()
                    )))
                },
            }
        } else {
            Err(code_invariant_error("When block limit is used, last transaction (should be StateCheckpoint or genesis) must not be invalid"))
        }
    }

    fn is_transaction_dynamic_change_set_capable(txn: &Self::Txn) -> bool {
        if txn.is_valid() {
            if let Transaction::GenesisTransaction(WriteSetPayload::Direct(_)) = txn.expect_valid()
            {
                // WriteSetPayload::Direct cannot be handled in mode where delayed_field_optimization or
                // resource_group_split_in_write_set is enabled.
                return false;
            }
        }
        true
    }
}
