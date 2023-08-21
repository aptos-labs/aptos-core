// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    errors::Error,
    task::{ExecutionStatus, Transaction, TransactionOutput},
};
use anyhow::anyhow;
use aptos_mvhashmap::types::{Incarnation, TxnIndex, Version};
use aptos_types::{
    access_path::AccessPath, executable::ModulePath, fee_statement::FeeStatement,
    write_set::WriteOp,
};
use arc_swap::ArcSwapOption;
use crossbeam::utils::CachePadded;
use dashmap::DashSet;
use std::{
    collections::HashSet,
    fmt::Debug,
    iter::{empty, Iterator},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

type TxnInput<K> = Vec<ReadDescriptor<K>>;
// When a transaction is committed, the output delta writes must be populated by
// the WriteOps corresponding to the deltas in the corresponding outputs.
#[derive(Debug)]
pub(crate) struct TxnOutput<T: TransactionOutput, E: Debug> {
    output_status: ExecutionStatus<T, Error<E>>,
}
type KeySet<T> = HashSet<<<T as TransactionOutput>::Txn as Transaction>::Key>;

impl<T: TransactionOutput, E: Debug> TxnOutput<T, E> {
    pub fn from_output_status(output_status: ExecutionStatus<T, Error<E>>) -> Self {
        Self { output_status }
    }

    pub fn output_status(&self) -> &ExecutionStatus<T, Error<E>> {
        &self.output_status
    }
}

/// Information about the read which is used by validation.
#[derive(Clone, PartialEq)]
enum ReadKind {
    /// Read returned a value from the multi-version data-structure, with index
    /// and incarnation number of the execution associated with the write of
    /// that entry.
    Version(TxnIndex, Incarnation),
    /// Read resolved a delta.
    Resolved(u128),
    /// Read occurred from storage.
    Storage,
    /// Read triggered a delta application failure.
    DeltaApplicationFailure,
}

#[derive(Clone)]
pub struct ReadDescriptor<K> {
    access_path: K,

    kind: ReadKind,
}

impl<K: ModulePath> ReadDescriptor<K> {
    pub fn from_version(access_path: K, txn_idx: TxnIndex, incarnation: Incarnation) -> Self {
        Self {
            access_path,
            kind: ReadKind::Version(txn_idx, incarnation),
        }
    }

    pub fn from_resolved(access_path: K, value: u128) -> Self {
        Self {
            access_path,
            kind: ReadKind::Resolved(value),
        }
    }

    pub fn from_storage(access_path: K) -> Self {
        Self {
            access_path,
            kind: ReadKind::Storage,
        }
    }

    pub fn from_delta_application_failure(access_path: K) -> Self {
        Self {
            access_path,
            kind: ReadKind::DeltaApplicationFailure,
        }
    }

    fn module_path(&self) -> Option<AccessPath> {
        self.access_path.module_path()
    }

    pub fn path(&self) -> &K {
        &self.access_path
    }

    // Does the read descriptor describe a read from MVHashMap w. a specified version.
    pub fn validate_version(&self, version: Version) -> bool {
        let (txn_idx, incarnation) = version;
        self.kind == ReadKind::Version(txn_idx, incarnation)
    }

    // Does the read descriptor describe a read from MVHashMap w. a resolved delta.
    pub fn validate_resolved(&self, value: u128) -> bool {
        self.kind == ReadKind::Resolved(value)
    }

    // Does the read descriptor describe a read from storage.
    pub fn validate_storage(&self) -> bool {
        self.kind == ReadKind::Storage
    }

    // Does the read descriptor describe to a read with a delta application failure.
    pub fn validate_delta_application_failure(&self) -> bool {
        self.kind == ReadKind::DeltaApplicationFailure
    }
}

pub struct TxnLastInputOutput<K, T: TransactionOutput, E: Debug> {
    inputs: Vec<CachePadded<ArcSwapOption<TxnInput<K>>>>, // txn_idx -> input.

    outputs: Vec<CachePadded<ArcSwapOption<TxnOutput<T, E>>>>, // txn_idx -> output.

    // Record all writes and reads to access paths corresponding to modules (code) in any
    // (speculative) executions. Used to avoid a potential race with module publishing and
    // Move-VM loader cache - see 'record' function comment for more information.
    module_writes: DashSet<AccessPath>,
    module_reads: DashSet<AccessPath>,

    module_read_write_intersection: AtomicBool,
}

impl<K: ModulePath, T: TransactionOutput, E: Debug + Send + Clone> TxnLastInputOutput<K, T, E> {
    pub fn new(num_txns: TxnIndex) -> Self {
        Self {
            inputs: (0..num_txns)
                .map(|_| CachePadded::new(ArcSwapOption::empty()))
                .collect(),
            outputs: (0..num_txns)
                .map(|_| CachePadded::new(ArcSwapOption::empty()))
                .collect(),
            module_writes: DashSet::new(),
            module_reads: DashSet::new(),
            module_read_write_intersection: AtomicBool::new(false),
        }
    }

    fn append_and_check(
        paths: Vec<AccessPath>,
        set_to_append: &DashSet<AccessPath>,
        set_to_check: &DashSet<AccessPath>,
    ) -> bool {
        for path in paths {
            // Standard flags, first show, then look.
            set_to_append.insert(path.clone());

            if set_to_check.contains(&path) {
                return true;
            }
        }
        false
    }

    /// Returns an error if a module path that was read was previously written to, and vice versa.
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
        input: Vec<ReadDescriptor<K>>,
        output: ExecutionStatus<T, Error<E>>,
    ) -> anyhow::Result<()> {
        let read_modules: Vec<AccessPath> =
            input.iter().filter_map(|desc| desc.module_path()).collect();
        let written_modules: Vec<AccessPath> = match &output {
            ExecutionStatus::Success(output) | ExecutionStatus::SkipRest(output) => output
                .get_writes()
                .into_iter()
                .filter_map(|(k, _)| k.module_path())
                .collect(),
            ExecutionStatus::Abort(_) => Vec::new(),
        };

        if !self.module_read_write_intersection.load(Ordering::Relaxed) {
            // Check if adding new read & write modules leads to intersections.
            if Self::append_and_check(read_modules, &self.module_reads, &self.module_writes)
                || Self::append_and_check(written_modules, &self.module_writes, &self.module_reads)
            {
                self.module_read_write_intersection
                    .store(true, Ordering::Release);
                return Err(anyhow!(
                    "[BlockSTM]: Detect module r/w intersection, will fallback to sequential execution"
                ));
            }
        }

        self.inputs[txn_idx as usize].store(Some(Arc::new(input)));
        self.outputs[txn_idx as usize].store(Some(Arc::new(TxnOutput::from_output_status(output))));

        Ok(())
    }

    pub(crate) fn module_publishing_may_race(&self) -> bool {
        self.module_read_write_intersection.load(Ordering::Acquire)
    }

    pub(crate) fn read_set(&self, txn_idx: TxnIndex) -> Option<Arc<Vec<ReadDescriptor<K>>>> {
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

    /// Does a transaction at txn_idx have SkipRest or Abort status.
    pub(crate) fn block_truncated_at_idx(&self, txn_idx: TxnIndex) -> bool {
        matches!(
            &self.outputs[txn_idx as usize]
                .load_full()
                .expect("[BlockSTM]: Execution output must be recorded after execution")
                .output_status,
            ExecutionStatus::SkipRest(_) | ExecutionStatus::Abort(_)
        )
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

    pub(crate) fn txn_output(&self, txn_idx: TxnIndex) -> Option<Arc<TxnOutput<T, E>>> {
        self.outputs[txn_idx as usize].load_full()
    }

    // Extracts a set of paths written or updated during execution from transaction
    // output: (modified by writes, modified by deltas).
    pub(crate) fn modified_keys(&self, txn_idx: TxnIndex) -> KeySet<T> {
        match &self.outputs[txn_idx as usize].load_full() {
            None => HashSet::new(),
            Some(txn_output) => match &txn_output.output_status {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => t
                    .get_writes()
                    .into_iter()
                    .map(|(k, _)| k)
                    .chain(t.get_deltas().into_iter().map(|(k, _)| k))
                    .collect(),
                ExecutionStatus::Abort(_) => HashSet::new(),
            },
        }
    }

    pub(crate) fn delta_keys(
        &self,
        txn_idx: TxnIndex,
    ) -> (
        usize,
        Box<dyn Iterator<Item = <<T as TransactionOutput>::Txn as Transaction>::Key>>,
    ) {
        let ret: (
            usize,
            Box<dyn Iterator<Item = <<T as TransactionOutput>::Txn as Transaction>::Key>>,
        ) = self.outputs[txn_idx as usize].load().as_ref().map_or(
            (
                0,
                Box::new(empty::<<<T as TransactionOutput>::Txn as Transaction>::Key>()),
            ),
            |txn_output| match &txn_output.output_status {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => {
                    let deltas = t.get_deltas();
                    (deltas.len(), Box::new(deltas.into_iter().map(|(k, _)| k)))
                },
                ExecutionStatus::Abort(_) => (
                    0,
                    Box::new(empty::<<<T as TransactionOutput>::Txn as Transaction>::Key>()),
                ),
            },
        );
        ret
    }

    pub(crate) fn events(
        &self,
        txn_idx: TxnIndex,
    ) -> Box<dyn Iterator<Item = <<T as TransactionOutput>::Txn as Transaction>::Event>> {
        self.outputs[txn_idx as usize].load().as_ref().map_or(
            Box::new(empty::<<<T as TransactionOutput>::Txn as Transaction>::Event>()),
            |txn_output| match &txn_output.output_status {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => {
                    let events = t.get_events();
                    Box::new(events.into_iter())
                },
                ExecutionStatus::Abort(_) => {
                    Box::new(empty::<<<T as TransactionOutput>::Txn as Transaction>::Event>())
                },
            },
        )
    }

    // Called when a transaction is committed to record WriteOps for materialized aggregator values
    // corresponding to the (deltas) in the recorded final output of the transaction.
    pub(crate) fn record_delta_writes(
        &self,
        txn_idx: TxnIndex,
        delta_writes: Vec<(<<T as TransactionOutput>::Txn as Transaction>::Key, WriteOp)>,
    ) {
        match &self.outputs[txn_idx as usize]
            .load_full()
            .expect("Output must exist")
            .output_status
        {
            ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => {
                t.incorporate_delta_writes(delta_writes);
            },
            ExecutionStatus::Abort(_) => {},
        };
    }

    // Must be executed after parallel execution is done, grabs outputs. Will panic if
    // other outstanding references to the recorded outputs exist.
    pub(crate) fn take_output(&self, txn_idx: TxnIndex) -> ExecutionStatus<T, Error<E>> {
        let owning_ptr = self.outputs[txn_idx as usize]
            .swap(None)
            .expect("[BlockSTM]: Output must be recorded after execution");

        Arc::try_unwrap(owning_ptr)
            .map(|output| output.output_status)
            .expect("[BlockSTM]: Output should be uniquely owned after execution")
    }
}
