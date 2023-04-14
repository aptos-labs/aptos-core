// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters, scheduler::Scheduler, task::Transaction, txn_last_input_output::ReadDescriptor,
};
use anyhow::Result;
use aptos_aggregator::delta_change_set::{deserialize, serialize};
use aptos_logger::error;
use aptos_mvhashmap::{
    types::{MVCodeError, MVCodeOutput, MVDataError, MVDataOutput, TxnIndex},
    unsync_map::UnsyncMap,
    MVHashMap,
};
use aptos_state_view::{StateViewId, TStateView};
use aptos_types::{
    executable::{Executable, ExecutableDescriptor, ExecutableView, FetchedModule, ModulePath},
    state_store::{state_storage_usage::StateStorageUsage, state_value::StateValue},
    vm_status::{StatusCode, VMStatus},
    write_set::TransactionWrite,
};
use aptos_vm_logging::{log_schema::AdapterLogSchema, prelude::*};
use std::{cell::RefCell, fmt::Debug, hash::Hash, sync::Arc};

/// A struct that is always used by a single thread performing an execution task. The struct is
/// passed to the VM and acts as a proxy to resolve reads first in the shared multi-version
/// data-structure. It also allows the caller to track the read-set and any dependencies.
///
/// TODO(issue 10177): MvHashMapView currently needs to be sync due to trait bounds, but should
/// not be. In this case, the read_dependency member can have a RefCell<bool> type and the
/// captured_reads member can have RefCell<Vec<ReadDescriptor<K>>> type.
pub(crate) struct MVHashMapView<
    'a,
    K: ModulePath + PartialOrd + Ord + Send + Clone + Hash + Eq + Sync + Debug,
    V: TransactionWrite + Send + Sync,
    X: Executable,
> {
    versioned_map: &'a MVHashMap<K, V, X>,
    scheduler: &'a Scheduler,
    captured_reads: RefCell<Vec<(K, ReadDescriptor)>>,
    captured_executables: RefCell<Vec<(K, ExecutableDescriptor)>>,
}

/// A struct which describes the result of the read from the proxy. The client
/// can interpret these types to further resolve the reads.
#[derive(Debug)]
pub(crate) enum ReadResult<V> {
    // Successful read of a value.
    Value(Arc<V>),
    // Similar to above, but the value was aggregated and is an integer.
    U128(u128),
    // Read could not resolve the delta (no base value).
    Unresolved,
    // Read did not return anything.
    None,
}

impl<
        'a,
        K: ModulePath + PartialOrd + Ord + Send + Sync + Clone + Debug + Hash + Eq,
        V: TransactionWrite + Send + Sync,
        X: Executable,
    > MVHashMapView<'a, K, V, X>
{
    pub(crate) fn new(versioned_map: &'a MVHashMap<K, V, X>, scheduler: &'a Scheduler) -> Self {
        Self {
            versioned_map,
            scheduler,
            captured_reads: RefCell::new(Vec::new()),
            captured_executables: RefCell::new(Vec::new()),
        }
    }

    /// Drains the captured reads.
    pub(crate) fn take_captured_inputs(
        self,
    ) -> (Vec<(K, ReadDescriptor)>, Vec<(K, ExecutableDescriptor)>) {
        (self.captured_reads.take(), self.captured_executables.take())
    }

    /// Captures a key, executable descriptor pair if an executable is returned,
    /// otherwise, may return a module blob without capturing anything (for the
    /// VM to produce executable), or None, indicating the module blob must be
    /// returned from storage. May internally wait for a dependency.
    fn fetch_code(
        &self,
        key: &K,
        txn_idx: TxnIndex,
    ) -> Option<(ExecutableDescriptor, FetchedModule<X>)> {
        use MVCodeError::*;
        use MVCodeOutput::*;

        loop {
            match self.versioned_map.fetch_code(key, txn_idx) {
                Ok(Executable((executable, descriptor))) => {
                    self.captured_executables
                        .borrow_mut()
                        .push((key.clone(), descriptor.clone()));
                    return Some((descriptor, FetchedModule::Executable(executable)));
                },
                Ok(Module((v, hash))) => {
                    return Some((
                        ExecutableDescriptor::Published(hash),
                        FetchedModule::Blob(
                            v.extract_raw_bytes().expect("Module can't be deleted"),
                        ),
                    ));
                },
                Err(Dependency(dep_idx)) => {
                    // `txn_idx` estimated to depend on a write from `dep_idx`.
                    self.wait_for_dependency(txn_idx, dep_idx);
                },
                Err(NotFound) => return None,
            }
        }
    }

    // Legacy API to fetch modules as data via StateView interface.
    // TODO: delete once executables are fetched via ExecutableView interface.
    fn fetch_code_legacy(
        &self,
        key: &K,
        txn_idx: TxnIndex,
    ) -> anyhow::Result<MVCodeOutput<Arc<V>, X>, MVCodeError> {
        // Add a fake read from storage to register in reads for now in order
        // for the read / write path intersection fallback for modules to still work.
        self.captured_reads
            .borrow_mut()
            .push((key.clone(), ReadDescriptor::Storage));

        self.versioned_map.fetch_code(key, txn_idx)
    }

    fn set_aggregator_base_value(&self, key: &K, value: u128) {
        self.versioned_map.set_aggregator_base_value(key, value);
    }

    /// Captures a read from the VM execution, but not unresolved deltas, as in this case it is the
    /// callers responsibility to set the aggregator's base value and call fetch_data again.
    fn wait_for_dependency(&self, txn_idx: TxnIndex, dep_idx: TxnIndex) {
        if let Some(dep_condition) = self.scheduler.wait_for_dependency(txn_idx, dep_idx) {
            let _timer = counters::DEPENDENCY_WAIT_SECONDS.start_timer();
            // Wait on a condition variable corresponding to the encountered
            // read dependency. Once the dep_idx finishes re-execution, scheduler
            // will mark the dependency as resolved, and then the txn_idx will be
            // scheduled for re-execution, which will re-awaken cvar here.
            // A deadlock is not possible due to these condition variables:
            // suppose all threads are waiting on read dependency, and consider
            // one with lowest txn_idx. It observed a dependency, so some thread
            // aborted dep_idx. If that abort returned execution task, by
            // minimality (lower transactions aren't waiting), that thread would
            // finish execution unblock txn_idx, contradiction. Otherwise,
            // execution_idx in scheduler was lower at a time when at least the
            // thread that aborted dep_idx was alive, and again, since lower txns
            // than txn_idx are not blocked, so the execution of dep_idx will
            // eventually finish and lead to unblocking txn_idx, contradiction.
            let (lock, cvar) = &*dep_condition;
            let mut dep_resolved = lock.lock();
            while !*dep_resolved {
                dep_resolved = cvar.wait(dep_resolved).unwrap();
            }
        }
    }

    /// Captures a read from the VM execution and returns ReadResult. May internally wait
    /// for a dependency.
    fn fetch_data(&self, key: &K, txn_idx: TxnIndex) -> ReadResult<V> {
        use MVDataError::*;
        use MVDataOutput::*;

        loop {
            match self.versioned_map.fetch_data(key, txn_idx) {
                Ok(Versioned(version, v)) => {
                    let (idx, incarnation) = version;
                    self.captured_reads
                        .borrow_mut()
                        .push((key.clone(), ReadDescriptor::Version(idx, incarnation)));
                    return ReadResult::Value(v);
                },
                Ok(Resolved(value)) => {
                    self.captured_reads
                        .borrow_mut()
                        .push((key.clone(), ReadDescriptor::Resolved(value)));
                    return ReadResult::U128(value);
                },
                Err(NotFound) => {
                    self.captured_reads
                        .borrow_mut()
                        .push((key.clone(), ReadDescriptor::Storage));
                    return ReadResult::None;
                },
                Err(Unresolved(_)) => return ReadResult::Unresolved,
                Err(Dependency(dep_idx)) => {
                    // `txn_idx` estimated to depend on a write from `dep_idx`.
                    self.wait_for_dependency(txn_idx, dep_idx);
                },
                Err(DeltaApplicationFailure) => {
                    // Delta application failure currently should never happen. Here, we assume it
                    // happened because of speculation and return 0 to the Move-VM. Validation will
                    // ensure the transaction re-executes if 0 wasn't the right number.
                    self.captured_reads
                        .borrow_mut()
                        .push((key.clone(), ReadDescriptor::DeltaApplicationFailure));
                    return ReadResult::U128(0);
                },
            };
        }
    }
}

enum ViewMapKind<'a, T: Transaction, X: Executable> {
    MultiVersion(&'a MVHashMapView<'a, T::Key, T::Value, X>),
    Unsync(&'a UnsyncMap<T::Key, T::Value, X>),
}

pub(crate) struct LatestView<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> {
    base_view: &'a S,
    latest_view: ViewMapKind<'a, T, X>,
    txn_idx: TxnIndex,
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> LatestView<'a, T, S, X> {
    pub(crate) fn new_mv_view(
        base_view: &'a S,
        map: &'a MVHashMapView<'a, T::Key, T::Value, X>,
        txn_idx: TxnIndex,
    ) -> LatestView<'a, T, S, X> {
        LatestView {
            base_view,
            latest_view: ViewMapKind::MultiVersion(map),
            txn_idx,
        }
    }

    pub(crate) fn new_unsync_view(
        base_view: &'a S,
        map: &'a UnsyncMap<T::Key, T::Value, X>,
        txn_idx: TxnIndex,
    ) -> LatestView<'a, T, S, X> {
        LatestView {
            base_view,
            latest_view: ViewMapKind::Unsync(map),
            txn_idx,
        }
    }

    fn log_storage_error(&self, key: &T::Key) {
        // Even speculatively, reading from base view should not return an error.
        // Thus, this critical error log and count does not need to be buffered.
        let log_context = AdapterLogSchema::new(self.base_view.id(), self.txn_idx as usize);
        alert!(
            log_context,
            "[VM, StateView] Error getting data from storage for {:?}",
            key
        );
    }

    fn get_base_value(&self, key: &T::Key) -> anyhow::Result<Option<StateValue>> {
        self.base_view.get_state_value(key).map_err(|e| {
            self.log_storage_error(key);
            e
        })
    }

    fn get_base_module(
        &self,
        key: &T::Key,
    ) -> anyhow::Result<(ExecutableDescriptor, FetchedModule<X>)> {
        self.base_view.get_state_value_bytes(key).map_or_else(
            |e| {
                self.log_storage_error(key);
                Err(e)
            },
            |maybe_bytes| {
                Ok((
                    ExecutableDescriptor::Storage,
                    FetchedModule::Blob(maybe_bytes.expect("Module can't be deleted")),
                ))
            },
        )
    }
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> TStateView
    for LatestView<'a, T, S, X>
{
    type Key = T::Key;

    fn get_state_value(&self, state_key: &T::Key) -> anyhow::Result<Option<StateValue>> {
        match self.latest_view {
            // TODO: get rid of the dispatch once caller statically dispatches data vs executable
            // fetching via StateView or ExecutableView traits.
            ViewMapKind::MultiVersion(map) => match state_key.module_path() {
                Some(_) => {
                    use MVCodeError::*;
                    use MVCodeOutput::*;

                    match map.fetch_code_legacy(state_key, self.txn_idx) {
                        Ok(Executable(_)) => unreachable!("Versioned executable not implemented"),
                        Ok(Module((v, _))) => Ok(v.as_state_value()),
                        Err(Dependency(_)) => {
                            // Return anything (e.g. module does not exist) to avoid waiting,
                            // because parallel execution will fall back to sequential anyway.
                            Ok(None)
                        },
                        Err(NotFound) => self.base_view.get_state_value(state_key),
                    }
                },
                None => {
                    let mut mv_value = map.fetch_data(state_key, self.txn_idx);

                    if matches!(mv_value, ReadResult::Unresolved) {
                        let from_storage =
                            self.base_view.get_state_value_bytes(state_key)?.map_or(
                                Err(VMStatus::Error(StatusCode::STORAGE_ERROR, None)),
                                |bytes| Ok(deserialize(&bytes)),
                            )?;

                        // Store base value in the versioned data-structure directly, so subsequent
                        // reads can be resolved to U128 directly without storage calls.
                        map.set_aggregator_base_value(state_key, from_storage);

                        mv_value = map.fetch_data(state_key, self.txn_idx);
                    }

                    match mv_value {
                        ReadResult::Value(v) => Ok(v.as_state_value()),
                        ReadResult::U128(v) => Ok(Some(StateValue::new_legacy(serialize(&v)))),
                        ReadResult::None => self.get_base_value(state_key),
                        ReadResult::Unresolved => unreachable!(
                            "Must be resolved as base value is recorded in the MV data structure"
                        ),
                    }
                },
            },
            ViewMapKind::Unsync(map) => map.fetch_data(state_key).map_or_else(
                || self.get_base_value(state_key),
                |v| Ok(v.as_state_value()),
            ),
        }
    }

    fn id(&self) -> StateViewId {
        self.base_view.id()
    }

    fn is_genesis(&self) -> bool {
        self.base_view.is_genesis()
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        self.base_view.get_usage()
    }
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> ExecutableView
    for LatestView<'a, T, S, X>
{
    type Executable = X;
    type Key = T::Key;

    fn fetch_module(
        &self,
        key: &T::Key,
    ) -> anyhow::Result<(ExecutableDescriptor, FetchedModule<X>)> {
        match self.latest_view {
            ViewMapKind::MultiVersion(map) => map
                .fetch_code(key, self.txn_idx)
                .map_or_else(|| self.get_base_module(key), Ok),
            ViewMapKind::Unsync(map) => map.fetch_code(key).map_or_else(
                || self.get_base_module(key),
                |res| {
                    Ok(match res {
                        MVCodeOutput::Executable((executable, descriptor)) => {
                            (descriptor, FetchedModule::Executable(executable))
                        },
                        MVCodeOutput::Module((v, hash)) => (
                            ExecutableDescriptor::Published(hash),
                            FetchedModule::Blob(
                                v.extract_raw_bytes().expect("Module can't be deleted"),
                            ),
                        ),
                    })
                },
            ),
        }
    }

    fn store_executable(
        &self,
        key: &Self::Key,
        descriptor: ExecutableDescriptor,
        executable: Self::Executable,
    ) {
        match self.latest_view {
            ViewMapKind::MultiVersion(map) => map
                .versioned_map
                .store_executable(key, descriptor, executable),
            ViewMapKind::Unsync(map) => {
                if !map.store_executable(key, descriptor.clone(), executable) {
                    // Should not happen during sequential execution.
                    let log_context =
                        AdapterLogSchema::new(self.base_view.id(), self.txn_idx as usize);
                    error!(
                        log_context,
                        "Executable w. descriptor = {:?} already stored for key = {:?}",
                        descriptor,
                        key
                    );
                }
            },
        }
    }
}
