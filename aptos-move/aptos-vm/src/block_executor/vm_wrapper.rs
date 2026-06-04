// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    aptos_vm::AptosVM,
    block_executor::{hotness_recorder::HotnessReadRecorder, AptosTransactionOutput},
};
use aptos_block_executor::task::{ExecutionStatus, ExecutorTask};
use aptos_logger::{enabled, Level};
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    state_store::{state_key::StateKey, StateView, StateViewId},
    transaction::{
        block_epilogue::BlockEpiloguePayload,
        signature_verified_transaction::SignatureVerifiedTransaction, AuxiliaryInfo, Transaction,
        WriteSetPayload,
    },
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::{log_schema::AdapterLogSchema, prelude::*};
use aptos_vm_types::{
    module_and_script_storage::code_storage::AptosCodeStorage,
    output::VMOutput,
    resolver::{BlockSynchronizationKillSwitch, ExecutorView, ResourceGroupView},
};
use fail::fail_point;
use move_core_types::vm_status::{StatusCode, VMStatus};
use std::collections::BTreeSet;

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

        // Hot-state observation is gated on the same feature that decides whether hotness is
        // persisted in the block epilogue. When enabled, wrap the view in a recorder so all
        // resolver-boundary reads (value/metadata/exists/size/group/aggregator-v1) are captured
        // deterministically; the original `view` is still passed for code storage / kill switch.
        let recording = self.vm.features().is_hotness_in_epilogue_enabled();
        let (execution_result, hotness_reads) = if recording {
            let recorder = HotnessReadRecorder::new(view);
            let resolver = self.vm.as_move_resolver_with_group_view(&recorder);
            let result = self.vm.execute_single_transaction(
                txn,
                &resolver,
                view,
                &log_context,
                auxiliary_info,
            );
            let reads = recorder.take_reads();
            (result, reads)
        } else {
            let resolver = self.vm.as_move_resolver_with_group_view(view);
            let result = self.vm.execute_single_transaction(
                txn,
                &resolver,
                view,
                &log_context,
                auxiliary_info,
            );
            (result, BTreeSet::new())
        };

        match execution_result {
            Ok((vm_status, mut vm_output)) => {
                if recording {
                    attach_hotness(&mut vm_output, txn, hotness_reads);
                }
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

/// Attaches VM-boundary hotness data to a successful output.
///
/// `set_hotness_reads` records the resolver-boundary reads used to feed the block hot-state
/// accumulator (these are not persisted in the write set for user transactions). For a
/// `BlockEpiloguePayload::V2` epilogue we additionally compute the VM-owned promotion set persisted
/// in the output's write set: the payload's `to_make_hot` unioned with the epilogue's own reads
/// minus its writes. V0 carries no hotness and V1 hotness is ephemeral (not serialized), so neither
/// adds the epilogue's own reads — replay parity for the persisted set is a V2-only guarantee.
fn attach_hotness(
    vm_output: &mut VMOutput,
    txn: &SignatureVerifiedTransaction,
    hotness_reads: BTreeSet<StateKey>,
) {
    if txn.is_valid() {
        if let Transaction::BlockEpilogue(payload) = txn.expect_valid() {
            let mut promotion = payload
                .try_get_keys_to_make_hot()
                .cloned()
                .unwrap_or_default();
            if matches!(payload, BlockEpiloguePayload::V2 { .. }) {
                // Add the epilogue's own reads (V2 is the replay-parity-guaranteed format).
                promotion.extend(hotness_reads.iter().cloned());
            }
            // Keep the epilogue's own writes out of the promotion set, matching the normal block
            // policy: a written key is already made hot by the write itself, so a separate MakeHot
            // op would be redundant (and would collide with the value write in the write set).
            for (key, _) in vm_output.concrete_write_set_iter() {
                promotion.remove(key);
            }
            vm_output.set_hotness_promotion(promotion);
        }
    }
    vm_output.set_hotness_reads(hotness_reads);
}
