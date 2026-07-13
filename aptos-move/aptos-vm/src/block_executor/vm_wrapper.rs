// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{aptos_vm::AptosVM, block_executor::AptosTransactionOutput};
use aptos_block_executor::task::{ExecutionStatus, ExecutorTask};
use aptos_logger::{enabled, Level};
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    block_executor::value::ValueWithLayout,
    state_store::{state_key::StateKey, StateView, StateViewId},
    timestamp::TimestampResource,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, AuxiliaryInfo, Transaction,
        WriteSetPayload,
    },
    write_set::WriteOp,
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::{log_schema::AdapterLogSchema, prelude::*};
use aptos_vm_types::{
    module_and_script_storage::{
        code_storage::AptosCodeStorage, read_recording::ReadRecordingCodeStorage,
    },
    output::UnorderedReadSet,
    resolver::{BlockSynchronizationKillSwitch, ExecutorView, ResourceGroupView},
};
use fail::fail_point;
use move_core_types::{
    account_address::AccountAddress,
    vm_status::{StatusCode, VMStatus},
};

pub struct AptosExecutorTask {
    vm: AptosVM,
    id: StateViewId,
}

impl ExecutorTask for AptosExecutorTask {
    type AuxiliaryInfo = AuxiliaryInfo;
    type Error = VMStatus;
    type Output = AptosTransactionOutput;
    type Txn = SignatureVerifiedTransaction;

    fn init(
        environment: &AptosEnvironment,
        state_view: &impl StateView,
        async_runtime_checks_enabled: bool,
    ) -> Self {
        let vm = AptosVM::new_for_block_executor(environment, async_runtime_checks_enabled);
        let id = state_view.id();
        Self { vm, id }
    }

    // This function is called by the BlockExecutor for each transaction it intends
    // to execute (via the ExecutorTask trait). It can be as a part of sequential
    // execution, or speculatively as a part of a parallel execution.
    fn execute_transaction(
        &self,
        view: &(impl ExecutorView
              + ResourceGroupView
              + AptosCodeStorage
              + BlockSynchronizationKillSwitch),
        txn: &SignatureVerifiedTransaction,
        auxiliary_info: &Self::AuxiliaryInfo,
        txn_idx: TxnIndex,
    ) -> ExecutionStatus<AptosTransactionOutput, VMStatus> {
        fail_point!("aptos_vm::vm_wrapper::execute_transaction", |_| {
            ExecutionStatus::DelayedFieldsCodeInvariantError("fail points error".into())
        });

        let log_context = AdapterLogSchema::new(self.id, txn_idx as usize);
        let resolver = self.vm.as_move_resolver_with_group_view(view);
        let code_storage = ReadRecordingCodeStorage::new(view);
        match self.vm.execute_single_transaction(
            txn,
            &resolver,
            &code_storage,
            &log_context,
            auxiliary_info,
        ) {
            Ok((vm_status, vm_output)) => {
                // Discarded transactions commit no state changes, so their reads must not feed
                // hot-state promotion. Only carry the read set for outputs that can commit.
                let read_set = if vm_output.status().is_discarded() {
                    speculative_trace!(
                        &log_context,
                        format!("Transaction discarded, status: {:?}", vm_status),
                    );
                    UnorderedReadSet::default()
                } else {
                    UnorderedReadSet::new(
                        resolver.take_recorded_reads(),
                        code_storage.into_recorded_reads(),
                    )
                };
                if vm_status.status_code() == StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR {
                    ExecutionStatus::SpeculativeExecutionAbortError(
                        vm_status.message().cloned().unwrap_or_default(),
                    )
                } else if vm_status.status_code()
                    == StatusCode::DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR
                {
                    ExecutionStatus::DelayedFieldsCodeInvariantError(
                        vm_status.message().cloned().unwrap_or_default(),
                    )
                } else if AptosVM::should_restart_execution(vm_output.events()) {
                    speculative_info!(
                        &log_context,
                        "Reconfiguration occurred: restart required".into()
                    );
                    ExecutionStatus::SkipRest(AptosTransactionOutput::new_with_read_set(
                        vm_output, read_set,
                    ))
                } else {
                    assert!(
                        Self::is_transaction_dynamic_change_set_capable(txn),
                        "DirectWriteSet should always create SkipRest transaction, validate_waypoint_change_set provides this guarantee"
                    );
                    ExecutionStatus::Success(AptosTransactionOutput::new_with_read_set(
                        vm_output, read_set,
                    ))
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
                    == StatusCode::DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR
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

    fn pre_write_values(txn: &Self::Txn) -> Vec<(StateKey, ValueWithLayout<WriteOp>)> {
        let timestamp = match txn {
            SignatureVerifiedTransaction::Valid(Transaction::BlockMetadataExt(metadata_txn)) => {
                Some(metadata_txn.timestamp_usecs())
            },
            SignatureVerifiedTransaction::Valid(Transaction::BlockMetadata(metadata_txn)) => {
                Some(metadata_txn.timestamp_usecs())
            },
            _ => None,
        };

        match timestamp {
            Some(ts) => {
                // Use typed StateKey creation to avoid string parsing.
                // These unwraps are safe: TimestampResource is a valid MoveResource type,
                // and u64 serialization via BCS cannot fail.
                let state_key = StateKey::resource_typed::<TimestampResource>(&AccountAddress::ONE)
                    .expect("TimestampResource is a valid MoveResource");
                let value = WriteOp::legacy_modification(
                    bcs::to_bytes(&ts)
                        .expect("u64 BCS serialization cannot fail")
                        .into(),
                );
                // The timestamp resource has no delayed fields, so the pre-written value
                // is already in its exchanged form with no layout.
                vec![(
                    state_key,
                    ValueWithLayout::Exchanged(triomphe::Arc::new(value), None),
                )]
            },
            None => vec![],
        }
    }
}

impl AptosExecutorTask {
    fn is_transaction_dynamic_change_set_capable(txn: &SignatureVerifiedTransaction) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::HashValue;
    use aptos_types::block_metadata::BlockMetadata;

    #[test]
    fn test_pre_write_values_for_block_metadata() {
        let timestamp_usecs = 1234567890u64;
        let block_metadata = BlockMetadata::new(
            HashValue::zero(),
            1, // epoch
            1, // round
            AccountAddress::ONE,
            vec![], // previous_block_votes_bitvec
            vec![], // failed_proposer_indices
            timestamp_usecs,
        );

        let txn = SignatureVerifiedTransaction::Valid(Transaction::BlockMetadata(block_metadata));
        let pre_write_values = AptosExecutorTask::pre_write_values(&txn);

        // Should return exactly one pre-write entry for the timestamp
        assert_eq!(pre_write_values.len(), 1);

        let (state_key, value) = &pre_write_values[0];

        // Verify the state key is for the timestamp resource
        let expected_state_key =
            StateKey::resource_typed::<TimestampResource>(&AccountAddress::ONE)
                .expect("TimestampResource is a valid MoveResource");
        assert_eq!(state_key, &expected_state_key);

        // Verify the value is the serialized timestamp, in the exchanged form
        // with no layout.
        let expected_value = bcs::to_bytes(&timestamp_usecs).unwrap();
        assert!(matches!(value, ValueWithLayout::Exchanged(_, None)));
        assert_eq!(value.extract_value().bytes(), Some(&expected_value.into()));
    }

    #[test]
    fn test_pre_write_values_for_user_transaction_returns_empty() {
        // For non-block-metadata transactions, pre_write_values should return empty
        let state_checkpoint_txn =
            SignatureVerifiedTransaction::Valid(Transaction::StateCheckpoint(HashValue::zero()));
        assert!(AptosExecutorTask::pre_write_values(&state_checkpoint_txn).is_empty());
    }
}
