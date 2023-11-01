// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod vm_wrapper;

use crate::{
    block_executor::vm_wrapper::AptosExecutorTask,
    counters::{BLOCK_EXECUTOR_CONCURRENCY, BLOCK_EXECUTOR_EXECUTE_BLOCK_SECONDS},
};
use aptos_aggregator::{
    delayed_change::DelayedChange, delta_change_set::DeltaOp, types::DelayedFieldID,
};
use aptos_block_executor::{
    errors::Error, executor::BlockExecutor,
    task::TransactionOutput as BlockExecutorTransactionOutput,
    txn_commit_hook::TransactionCommitHook,
};
use aptos_infallible::Mutex;
use aptos_state_view::{StateView, StateViewId};
use aptos_types::{
    contract_event::ContractEvent,
    executable::ExecutableTestType,
    fee_statement::FeeStatement,
    state_store::state_key::StateKey,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, BlockExecutableTransaction,
        TransactionOutput, TransactionStatus,
    },
    write_set::WriteOp,
};
use aptos_vm_logging::{flush_speculative_logs, init_speculative_logs};
use aptos_vm_types::output::VMOutput;
use move_core_types::{language_storage::StructTag, value::MoveTypeLayout, vm_status::VMStatus};
use once_cell::sync::OnceCell;
use rayon::ThreadPool;
use std::{collections::BTreeMap, sync::Arc};

/// Output type wrapper used by block executor. VM output is stored first, then
/// transformed into TransactionOutput type that is returned.
#[derive(Debug)]
pub struct AptosTransactionOutput {
    // Note: should these mutexes be changed to ExplicitSyncSwapper?
    vm_output: Mutex<Option<VMOutput>>,
    committed_output: OnceCell<TransactionOutput>,
}

impl AptosTransactionOutput {
    pub(crate) fn new(output: VMOutput) -> Self {
        Self {
            vm_output: Mutex::new(Some(output)),
            committed_output: OnceCell::new(),
        }
    }

    pub(crate) fn committed_output(&self) -> &TransactionOutput {
        self.committed_output.get().unwrap()
    }

    fn take_output(mut self) -> TransactionOutput {
        match self.committed_output.take() {
            Some(output) => output,
            // TODO: revisit whether we should always get it via committed, or o.w. create a
            // dedicated API without creating empty data structures.
            None => self
                .vm_output
                .lock()
                .take()
                .expect("Output must be set")
                .into_transaction_output_with_materialized_write_set(
                    vec![],
                    BTreeMap::new(),
                    vec![],
                    vec![],
                ),
        }
    }
}

impl BlockExecutorTransactionOutput for AptosTransactionOutput {
    type Txn = SignatureVerifiedTransaction;

    /// Execution output for transactions that comes after SkipRest signal or when there was a
    /// problem creating the output (e.g. group serialization issue).
    fn skip_output() -> Self {
        Self::new(VMOutput::empty_with_status(TransactionStatus::Retry))
    }

    // TODO: get rid of the cloning data-structures in the following APIs.

    /// Should never be called after incorporate_additional_writes, as it
    /// will consume vm_output to prepare an output with deltas.
    fn resource_group_write_set(&self) -> Vec<(StateKey, WriteOp, BTreeMap<StructTag, WriteOp>)> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output must be set to get resource group writes")
            .change_set()
            .resource_group_write_set()
            .iter()
            .map(|(group_key, group_write)| {
                (
                    group_key.clone(),
                    group_write.metadata_op().clone(),
                    // TODO: propagate layouts.
                    group_write
                        .inner_ops()
                        .iter()
                        .map(|(tag, (op, _maybe_layout))| (tag.clone(), op.clone()))
                        .collect(),
                )
            })
            .collect()
    }

    /// More efficient implementation to avoid unnecessarily cloning inner_ops.
    fn resource_group_metadata_ops(&self) -> Vec<(StateKey, WriteOp)> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output must be set to get metadata ops")
            .change_set()
            .resource_group_write_set()
            .iter()
            .map(|(group_key, group_write)| (group_key.clone(), group_write.metadata_op().clone()))
            .collect()
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn resource_write_set(&self) -> BTreeMap<StateKey, (WriteOp, Option<Arc<MoveTypeLayout>>)> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output must be set to get resource writes")
            .change_set()
            .resource_write_set()
            .clone()
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn module_write_set(&self) -> BTreeMap<StateKey, WriteOp> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output must be set to get module writes")
            .change_set()
            .module_write_set()
            .clone()
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn aggregator_v1_write_set(&self) -> BTreeMap<StateKey, WriteOp> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output must be set to get aggregator V1 writes")
            .change_set()
            .aggregator_v1_write_set()
            .clone()
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn aggregator_v1_delta_set(&self) -> BTreeMap<StateKey, DeltaOp> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output must be set to get deltas")
            .change_set()
            .aggregator_v1_delta_set()
            .clone()
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn delayed_field_change_set(&self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output must be set to get aggregator change set")
            .change_set()
            .delayed_field_change_set()
            .clone()
    }

    fn reads_needing_delayed_field_exchange(
        &self,
    ) -> BTreeMap<
        <Self::Txn as BlockExecutableTransaction>::Key,
        (
            <Self::Txn as BlockExecutableTransaction>::Value,
            Arc<MoveTypeLayout>,
        ),
    > {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output to be set to get reads")
            .change_set()
            .reads_needing_delayed_field_exchange()
            .clone()
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn get_events(&self) -> Vec<(ContractEvent, Option<MoveTypeLayout>)> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output must be set to get events")
            .change_set()
            .events()
            .to_vec()
    }

    fn incorporate_materialized_txn_output(
        &self,
        aggregator_v1_writes: Vec<(<Self::Txn as BlockExecutableTransaction>::Key, WriteOp)>,
        patched_resource_write_set: BTreeMap<
            <Self::Txn as BlockExecutableTransaction>::Key,
            <Self::Txn as BlockExecutableTransaction>::Value,
        >,
        patched_events: Vec<<Self::Txn as BlockExecutableTransaction>::Event>,
        combined_groups: Vec<(
            <Self::Txn as BlockExecutableTransaction>::Key,
            <Self::Txn as BlockExecutableTransaction>::Value,
        )>,
    ) {
        assert!(
            self.committed_output
                .set(
                    self.vm_output
                        .lock()
                        .take()
                        .expect("Output must be set to incorporate materialized data")
                        .into_transaction_output_with_materialized_write_set(
                            aggregator_v1_writes,
                            patched_resource_write_set,
                            patched_events,
                            combined_groups,
                        ),
                )
                .is_ok(),
            "Could not combine VMOutput with the patched resource and event data"
        );
    }

    /// Return the fee statement of the transaction.
    /// Should never be called after vm_output is consumed.
    fn fee_statement(&self) -> FeeStatement {
        *self
            .vm_output
            .lock()
            .as_ref()
            .expect("Output to be set to get fee statement")
            .fee_statement()
    }
}

pub struct BlockAptosVM();

impl BlockAptosVM {
    pub fn execute_block<
        S: StateView + Sync,
        L: TransactionCommitHook<Output = AptosTransactionOutput>,
    >(
        executor_thread_pool: Arc<ThreadPool>,
        signature_verified_block: &[SignatureVerifiedTransaction],
        state_view: &S,
        concurrency_level: usize,
        maybe_block_gas_limit: Option<u64>,
        transaction_commit_listener: Option<L>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        let _timer = BLOCK_EXECUTOR_EXECUTE_BLOCK_SECONDS.start_timer();
        let num_txns = signature_verified_block.len();
        if state_view.id() != StateViewId::Miscellaneous {
            // Speculation is disabled in Miscellaneous context, which is used by testing and
            // can even lead to concurrent execute_block invocations, leading to errors on flush.
            init_speculative_logs(num_txns);
        }

        BLOCK_EXECUTOR_CONCURRENCY.set(concurrency_level as i64);
        let executor = BlockExecutor::<
            SignatureVerifiedTransaction,
            AptosExecutorTask<S>,
            S,
            L,
            ExecutableTestType,
        >::new(
            concurrency_level,
            executor_thread_pool,
            maybe_block_gas_limit,
            transaction_commit_listener,
        );

        let ret = executor.execute_block(state_view, signature_verified_block, state_view);
        match ret {
            Ok(outputs) => {
                let output_vec: Vec<TransactionOutput> = outputs
                    .into_iter()
                    .map(|output| output.take_output())
                    .collect();

                // Flush the speculative logs of the committed transactions.
                let pos = output_vec.partition_point(|o| !o.status().is_retry());

                if state_view.id() != StateViewId::Miscellaneous {
                    // Speculation is disabled in Miscellaneous context, which is used by testing and
                    // can even lead to concurrent execute_block invocations, leading to errors on flush.
                    flush_speculative_logs(pos);
                }

                Ok(output_vec)
            },
            Err(Error::FallbackToSequential(e)) => {
                unreachable!(
                    "[Execution]: Must be handled by sequential fallback: {:?}",
                    e
                )
            },
            Err(Error::UserError(err)) => Err(err),
        }
    }
}
