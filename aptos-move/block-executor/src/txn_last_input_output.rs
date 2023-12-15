// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    captured_reads::CapturedReads,
    errors::{Error, IntentionalFallbackToSequential},
    explicit_sync_wrapper::ExplicitSyncWrapper,
    task::{ExecutionStatus, TransactionOutput},
    types::{InputOutputKey, ReadWriteSummary},
};
use aptos_aggregator::types::PanicOr;
use aptos_mvhashmap::types::{TxnIndex, ValueWithLayout};
use aptos_types::{
    fee_statement::FeeStatement, transaction::BlockExecutableTransaction as Transaction,
    write_set::WriteOp,
};
use arc_swap::ArcSwapOption;
use crossbeam::utils::CachePadded;
use dashmap::DashSet;
use move_core_types::value::MoveTypeLayout;
use std::{
    collections::{BTreeMap, HashSet},
    fmt::Debug,
    iter::{empty, Iterator},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

type TxnInput<T> = CapturedReads<T>;

// When a transaction is committed, the output delta writes must be populated by
// the WriteOps corresponding to the deltas in the corresponding outputs.
#[derive(Debug)]
pub(crate) struct TxnOutput<O: TransactionOutput, E: Debug> {
    output_status: ExecutionStatus<O, Error<E>>,
}

pub(crate) enum KeyKind {
    Resource,
    Module,
    Group,
}

impl<O: TransactionOutput, E: Debug> TxnOutput<O, E> {
    pub fn from_output_status(output_status: ExecutionStatus<O, Error<E>>) -> Self {
        Self { output_status }
    }

    pub fn output_status(&self) -> &ExecutionStatus<O, Error<E>> {
        &self.output_status
    }
}

pub struct TxnLastInputOutput<T: Transaction, O: TransactionOutput<Txn = T>, E: Debug> {
    inputs: Vec<CachePadded<ArcSwapOption<TxnInput<T>>>>, // txn_idx -> input.
    // Set once when the group outputs are committed sequentially, to be processed later by
    // concurrent materialization / output preparation.
    finalized_groups: Vec<
        CachePadded<
            ExplicitSyncWrapper<Vec<(T::Key, T::Value, Vec<(T::Tag, ValueWithLayout<T::Value>)>)>>,
        >,
    >,

    outputs: Vec<CachePadded<ArcSwapOption<TxnOutput<O, E>>>>, // txn_idx -> output.

    // Record all writes and reads to access paths corresponding to modules (code) in any
    // (speculative) executions. Used to avoid a potential race with module publishing and
    // Move-VM loader cache - see 'record' function comment for more information.
    module_writes: DashSet<T::Key>,
    module_reads: DashSet<T::Key>,

    module_read_write_intersection: AtomicBool,
}

impl<T: Transaction, O: TransactionOutput<Txn = T>, E: Debug + Send + Clone>
    TxnLastInputOutput<T, O, E>
{
    pub fn new(num_txns: TxnIndex) -> Self {
        Self {
            inputs: (0..num_txns)
                .map(|_| CachePadded::new(ArcSwapOption::empty()))
                .collect(),
            outputs: (0..num_txns)
                .map(|_| CachePadded::new(ArcSwapOption::empty()))
                .collect(),
            finalized_groups: (0..num_txns)
                .map(|_| CachePadded::new(ExplicitSyncWrapper::<Vec<_>>::new(vec![])))
                .collect(),
            module_writes: DashSet::new(),
            module_reads: DashSet::new(),
            module_read_write_intersection: AtomicBool::new(false),
        }
    }

    fn append_and_check<'a>(
        paths: impl Iterator<Item = &'a T::Key>,
        set_to_append: &DashSet<T::Key>,
        set_to_check: &DashSet<T::Key>,
    ) -> bool {
        for path in paths {
            // Standard flags, first show, then look.
            set_to_append.insert(path.clone());

            if set_to_check.contains(path) {
                return true;
            }
        }
        false
    }

    /// Returns false on an error - if a module path that was read was previously written to, and vice versa.
    /// Since parallel executor is instantiated per block, any module that is in the Move-VM loader
    /// cache must previously be read and would be recorded in the 'module_reads' set. Any module
    /// that is written (published or re-published) goes through transaction output write-set and
    /// gets recorded in the 'module_writes' set. If these sets have an intersection, it is currently
    /// possible that Move-VM loader cache loads a module and incorrectly uses it for another
    /// transaction (e.g. a smaller transaction, or if the speculative execution of the publishing
    /// transaction later aborts). The intersection is guaranteed to be found because we first
    /// record the paths then check the other set (flags principle), and in this case we return an
    /// error that ensures a fallback to a correct sequential execution.
    /// When the sets do not have an intersection, it is impossible for the race to occur as any
    /// module in the loader cache may not be published by a transaction in the ongoing block.
    pub(crate) fn record(
        &self,
        txn_idx: TxnIndex,
        input: CapturedReads<T>,
        output: ExecutionStatus<O, Error<E>>,
    ) -> bool {
        let written_modules = match &output {
            ExecutionStatus::Success(output) | ExecutionStatus::SkipRest(output) => {
                output.module_write_set()
            },
            ExecutionStatus::Abort(_)
            | ExecutionStatus::DirectWriteSetTransactionNotCapableError
            | ExecutionStatus::SpeculativeExecutionAbortError(_)
            | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => BTreeMap::new(),
        };

        if !self.module_read_write_intersection.load(Ordering::Relaxed) {
            // Check if adding new read & write modules leads to intersections.
            if Self::append_and_check(
                input.module_reads.iter(),
                &self.module_reads,
                &self.module_writes,
            ) || Self::append_and_check(
                written_modules.keys(),
                &self.module_writes,
                &self.module_reads,
            ) {
                self.module_read_write_intersection
                    .store(true, Ordering::Release);
                return false;
            }
        }

        self.inputs[txn_idx as usize].store(Some(Arc::new(input)));
        self.outputs[txn_idx as usize].store(Some(Arc::new(TxnOutput::from_output_status(output))));

        true
    }

    pub(crate) fn read_set(&self, txn_idx: TxnIndex) -> Option<Arc<CapturedReads<T>>> {
        self.inputs[txn_idx as usize].load_full()
    }

    /// Returns the total gas, execution gas, io gas and storage gas of the transaction.
    pub(crate) fn fee_statement(&self, txn_idx: TxnIndex) -> Option<FeeStatement> {
        match &self.outputs[txn_idx as usize]
            .load_full()
            .expect("[BlockSTM]: Execution output must be recorded after execution")
            .output_status
        {
            ExecutionStatus::Success(output) | ExecutionStatus::SkipRest(output) => {
                Some(output.fee_statement())
            },
            _ => None,
        }
    }

    pub(crate) fn output_approx_size(&self, txn_idx: TxnIndex) -> Option<u64> {
        match &self.outputs[txn_idx as usize]
            .load_full()
            .expect("[BlockSTM]: Execution output must be recorded after execution")
            .output_status
        {
            ExecutionStatus::Success(output) | ExecutionStatus::SkipRest(output) => {
                Some(output.output_approx_size())
            },
            _ => None,
        }
    }

    /// Does a transaction at txn_idx have SkipRest or Abort status.
    pub(crate) fn block_skips_rest_at_idx(&self, txn_idx: TxnIndex) -> bool {
        matches!(
            &self.outputs[txn_idx as usize]
                .load_full()
                .expect("[BlockSTM]: Execution output must be recorded after execution")
                .output_status,
            ExecutionStatus::SkipRest(_)
        )
    }

    pub(crate) fn maybe_execution_error(&self, txn_idx: TxnIndex) -> Option<Error<E>> {
        if self.module_read_write_intersection.load(Ordering::Acquire) {
            return Some(Error::FallbackToSequential(PanicOr::Or(
                IntentionalFallbackToSequential::ModulePathReadWrite,
            )));
        }

        if let ExecutionStatus::Abort(err) = &self.outputs[txn_idx as usize]
            .load_full()
            .expect("[BlockSTM]: Execution output must be recorded after execution")
            .output_status
        {
            return Some(err.clone());
        }
        None
    }

    pub(crate) fn update_to_skip_rest(&self, txn_idx: TxnIndex) {
        if let ExecutionStatus::Success(output) = self.take_output(txn_idx) {
            self.outputs[txn_idx as usize].store(Some(Arc::new(TxnOutput {
                output_status: ExecutionStatus::SkipRest(output),
            })));
        } else {
            unreachable!();
        }
    }

    pub(crate) fn txn_output(&self, txn_idx: TxnIndex) -> Option<Arc<TxnOutput<O, E>>> {
        self.outputs[txn_idx as usize].load_full()
    }

    // Extracts a set of paths (keys) written or updated during execution from transaction
    // output, .1 for each item is false for non-module paths and true for module paths.
    pub(crate) fn modified_keys(
        &self,
        txn_idx: TxnIndex,
    ) -> Option<impl Iterator<Item = (T::Key, KeyKind)>> {
        self.outputs[txn_idx as usize]
            .load_full()
            .and_then(|txn_output| match &txn_output.output_status {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => Some(
                    t.resource_write_set()
                        .into_iter()
                        .map(|(k, _)| k)
                        .chain(t.aggregator_v1_write_set().into_keys())
                        .chain(t.aggregator_v1_delta_set().into_keys())
                        .map(|k| (k, KeyKind::Resource))
                        .chain(
                            t.module_write_set()
                                .into_keys()
                                .map(|k| (k, KeyKind::Module)),
                        )
                        .chain(
                            t.resource_group_metadata_ops()
                                .into_iter()
                                .map(|(k, _)| (k, KeyKind::Group)),
                        ),
                ),
                ExecutionStatus::Abort(_)
                | ExecutionStatus::DirectWriteSetTransactionNotCapableError
                | ExecutionStatus::SpeculativeExecutionAbortError(_)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => None,
            })
    }

    pub(crate) fn resource_write_set(
        &self,
        txn_idx: TxnIndex,
    ) -> Option<Vec<(T::Key, (T::Value, Option<Arc<MoveTypeLayout>>))>> {
        self.outputs[txn_idx as usize]
            .load_full()
            .and_then(|txn_output| match &txn_output.output_status {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => {
                    Some(t.resource_write_set())
                },
                ExecutionStatus::Abort(_)
                | ExecutionStatus::DirectWriteSetTransactionNotCapableError
                | ExecutionStatus::SpeculativeExecutionAbortError(_)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => None,
            })
    }

    pub(crate) fn delayed_field_keys(
        &self,
        txn_idx: TxnIndex,
    ) -> Option<impl Iterator<Item = T::Identifier>> {
        self.outputs[txn_idx as usize]
            .load()
            .as_ref()
            .and_then(|txn_output| match &txn_output.output_status {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => {
                    Some(t.delayed_field_change_set().into_keys())
                },
                ExecutionStatus::Abort(_)
                | ExecutionStatus::DirectWriteSetTransactionNotCapableError
                | ExecutionStatus::SpeculativeExecutionAbortError(_)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => None,
            })
    }

    pub(crate) fn reads_needing_delayed_field_exchange(
        &self,
        txn_idx: TxnIndex,
    ) -> Option<Vec<(T::Key, Arc<MoveTypeLayout>)>> {
        self.outputs[txn_idx as usize]
            .load()
            .as_ref()
            .and_then(|txn_output| match &txn_output.output_status {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => {
                    Some(t.reads_needing_delayed_field_exchange())
                },
                ExecutionStatus::Abort(_)
                | ExecutionStatus::DirectWriteSetTransactionNotCapableError
                | ExecutionStatus::SpeculativeExecutionAbortError(_)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => None,
            })
    }

    pub(crate) fn group_reads_needing_delayed_field_exchange(
        &self,
        txn_idx: TxnIndex,
    ) -> Option<Vec<(T::Key, T::Value)>> {
        self.outputs[txn_idx as usize]
            .load()
            .as_ref()
            .and_then(|txn_output| match &txn_output.output_status {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => {
                    Some(t.group_reads_needing_delayed_field_exchange())
                },
                ExecutionStatus::Abort(_)
                | ExecutionStatus::DirectWriteSetTransactionNotCapableError
                | ExecutionStatus::SpeculativeExecutionAbortError(_)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => None,
            })
    }

    pub(crate) fn aggregator_v1_delta_keys(&self, txn_idx: TxnIndex) -> Vec<T::Key> {
        self.outputs[txn_idx as usize].load().as_ref().map_or(
            vec![],
            |txn_output| match &txn_output.output_status {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => {
                    t.aggregator_v1_delta_set().into_keys().collect()
                },
                ExecutionStatus::Abort(_)
                | ExecutionStatus::DirectWriteSetTransactionNotCapableError
                | ExecutionStatus::SpeculativeExecutionAbortError(_)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => vec![],
            },
        )
    }

    pub(crate) fn group_metadata_ops(&self, txn_idx: TxnIndex) -> Vec<(T::Key, T::Value)> {
        self.outputs[txn_idx as usize].load().as_ref().map_or(
            vec![],
            |txn_output| match &txn_output.output_status {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => {
                    t.resource_group_metadata_ops()
                },
                ExecutionStatus::Abort(_)
                | ExecutionStatus::DirectWriteSetTransactionNotCapableError
                | ExecutionStatus::SpeculativeExecutionAbortError(_)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => vec![],
            },
        )
    }

    pub(crate) fn events(
        &self,
        txn_idx: TxnIndex,
    ) -> Box<dyn Iterator<Item = (T::Event, Option<MoveTypeLayout>)>> {
        self.outputs[txn_idx as usize].load().as_ref().map_or(
            Box::new(empty::<(T::Event, Option<MoveTypeLayout>)>()),
            |txn_output| match &txn_output.output_status {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => {
                    let events = t.get_events();
                    Box::new(events.into_iter())
                },
                ExecutionStatus::Abort(_)
                | ExecutionStatus::DirectWriteSetTransactionNotCapableError
                | ExecutionStatus::SpeculativeExecutionAbortError(_)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => {
                    Box::new(empty::<(T::Event, Option<MoveTypeLayout>)>())
                },
            },
        )
    }

    pub(crate) fn record_finalized_group(
        &self,
        txn_idx: TxnIndex,
        finalized_groups: Vec<(T::Key, T::Value, Vec<(T::Tag, ValueWithLayout<T::Value>)>)>,
    ) {
        *self.finalized_groups[txn_idx as usize].acquire() = finalized_groups;
    }

    pub(crate) fn take_finalized_group(
        &self,
        txn_idx: TxnIndex,
    ) -> Vec<(T::Key, T::Value, Vec<(T::Tag, ValueWithLayout<T::Value>)>)> {
        std::mem::take(&mut self.finalized_groups[txn_idx as usize].acquire())
    }

    // Called when a transaction is committed to record WriteOps for materialized aggregator values
    // corresponding to the (deltas) in the recorded final output of the transaction, as well as
    // finalized group updates.
    pub(crate) fn record_materialized_txn_output(
        &self,
        txn_idx: TxnIndex,
        delta_writes: Vec<(T::Key, WriteOp)>,
        patched_resource_write_set: Vec<(T::Key, T::Value)>,
        patched_events: Vec<T::Event>,
    ) {
        match &self.outputs[txn_idx as usize]
            .load_full()
            .expect("Output must exist")
            .output_status
        {
            ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => {
                t.incorporate_materialized_txn_output(
                    delta_writes,
                    patched_resource_write_set,
                    patched_events,
                );
            },
            ExecutionStatus::Abort(_)
            | ExecutionStatus::DirectWriteSetTransactionNotCapableError
            | ExecutionStatus::SpeculativeExecutionAbortError(_)
            | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => {},
        };
    }

    pub(crate) fn get_txn_read_write_summary(&self, txn_idx: TxnIndex) -> ReadWriteSummary<T> {
        let read_set = self.read_set(txn_idx).expect("Read set must be recorded");

        let reads = read_set.get_read_summary();
        let writes = self.get_write_summary(txn_idx);
        ReadWriteSummary::new(reads, writes)
    }

    pub(crate) fn get_write_summary(
        &self,
        txn_idx: TxnIndex,
    ) -> HashSet<InputOutputKey<T::Key, T::Tag, T::Identifier>> {
        match &self.outputs[txn_idx as usize]
            .load_full()
            .expect("Output must exist")
            .output_status
        {
            ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => t.get_write_summary(),
            ExecutionStatus::Abort(_)
            | ExecutionStatus::DirectWriteSetTransactionNotCapableError
            | ExecutionStatus::SpeculativeExecutionAbortError(_)
            | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => HashSet::new(),
        }
    }

    // Must be executed after parallel execution is done, grabs outputs. Will panic if
    // other outstanding references to the recorded outputs exist.
    pub(crate) fn take_output(&self, txn_idx: TxnIndex) -> ExecutionStatus<O, Error<E>> {
        let owning_ptr = self.outputs[txn_idx as usize]
            .swap(None)
            .expect("[BlockSTM]: Output must be recorded after execution");

        Arc::try_unwrap(owning_ptr)
            .map(|output| output.output_status)
            .expect("[BlockSTM]: Output should be uniquely owned after execution")
    }
}
