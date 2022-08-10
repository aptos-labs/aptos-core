// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    errors::Error,
    scheduler::{Incarnation, TxnIndex, Version},
    task::{ExecutionStatus, ModulePath, Transaction, TransactionOutput},
};
use anyhow::{bail, Result};
use aptos_types::access_path::AccessPath;
use arc_swap::ArcSwapOption;
use crossbeam::utils::CachePadded;
use dashmap::DashSet;
use std::{collections::HashSet, sync::Arc};

type TxnInput<K> = Vec<ReadDescriptor<K>>;
type TxnOutput<T, E> = ExecutionStatus<T, Error<E>>;

// If an entry was read from the multi-version data-structure, then kind is
// MVHashMap(txn_idx, incarnation), with transaction index and incarnation number
// of the execution associated with the write of the entry. Otherwise, if the read
// occured from storage, and kind is set to Storage.
#[derive(Clone, PartialEq)]
enum ReadKind {
    MVHashMap(TxnIndex, Incarnation),
    Storage,
}

#[derive(Clone)]
pub struct ReadDescriptor<K> {
    access_path: K,

    kind: ReadKind,
}

impl<K: ModulePath> ReadDescriptor<K> {
    pub fn from(access_path: K, txn_idx: TxnIndex, incarnation: Incarnation) -> Self {
        Self {
            access_path,
            kind: ReadKind::MVHashMap(txn_idx, incarnation),
        }
    }

    pub fn from_storage(access_path: K) -> Self {
        Self {
            access_path,
            kind: ReadKind::Storage,
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
        self.kind == ReadKind::MVHashMap(txn_idx, incarnation)
    }

    // Does the read descriptor describe a read from storage.
    pub fn validate_storage(&self) -> bool {
        self.kind == ReadKind::Storage
    }
}

pub struct TxnLastInputOutput<K, T, E> {
    inputs: Vec<CachePadded<ArcSwapOption<TxnInput<K>>>>, // txn_idx -> input.

    outputs: Vec<CachePadded<ArcSwapOption<TxnOutput<T, E>>>>, // txn_idx -> output.

    // Record all writes and reads to access paths corresponding to modules (code) in any
    // (speculative) executions. Used to avoid a potential race with module publishing and
    // Move-VM loader cache - see 'record' function comment for more information.
    module_writes: DashSet<AccessPath>,
    module_reads: DashSet<AccessPath>,
}

impl<K: ModulePath, T: TransactionOutput, E: Send + Clone> TxnLastInputOutput<K, T, E> {
    pub fn new(num_txns: usize) -> Self {
        Self {
            inputs: (0..num_txns)
                .map(|_| CachePadded::new(ArcSwapOption::empty()))
                .collect(),
            outputs: (0..num_txns)
                .map(|_| CachePadded::new(ArcSwapOption::empty()))
                .collect(),
            module_writes: DashSet::new(),
            module_reads: DashSet::new(),
        }
    }

    fn append_and_check(
        paths: Vec<AccessPath>,
        set_to_append: &DashSet<AccessPath>,
        set_to_check: &DashSet<AccessPath>,
    ) -> Result<()> {
        for path in paths {
            // Standard flags, first show, then look.
            set_to_append.insert(path.clone());

            if set_to_check.contains(&path) {
                bail!("Module published and also read");
            }
        }
        Ok(())
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
    pub fn record(
        &self,
        txn_idx: TxnIndex,
        input: Vec<ReadDescriptor<K>>,
        output: ExecutionStatus<T, Error<E>>,
    ) -> Result<()> {
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

        Self::append_and_check(read_modules, &self.module_reads, &self.module_writes)?;
        Self::append_and_check(written_modules, &self.module_writes, &self.module_reads)?;

        self.inputs[txn_idx].store(Some(Arc::new(input)));
        self.outputs[txn_idx].store(Some(Arc::new(output)));
        Ok(())
    }

    pub fn read_set(&self, txn_idx: TxnIndex) -> Option<Arc<Vec<ReadDescriptor<K>>>> {
        self.inputs[txn_idx].load_full()
    }

    // Extracts a set of paths written during execution from transaction output.
    pub fn write_set(
        &self,
        txn_idx: TxnIndex,
    ) -> HashSet<<<T as TransactionOutput>::T as Transaction>::Key> {
        match &self.outputs[txn_idx].load_full() {
            None => HashSet::new(),
            Some(txn_output) => match txn_output.as_ref() {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => {
                    t.get_writes().into_iter().map(|(k, _)| k).collect()
                }
                ExecutionStatus::Abort(_) => HashSet::new(),
            },
        }
    }

    // Must be executed after parallel execution is done, grabs outputs. Will panic if
    // other outstanding references to the recorded outputs exist.
    pub fn take_output(&self, txn_idx: TxnIndex) -> ExecutionStatus<T, Error<E>> {
        let owning_ptr = self.outputs[txn_idx]
            .swap(None)
            .expect("Output must be recorded after execution");

        if let Ok(output) = Arc::try_unwrap(owning_ptr) {
            output
        } else {
            unreachable!("Output should be uniquely owned after execution");
        }
    }
}
