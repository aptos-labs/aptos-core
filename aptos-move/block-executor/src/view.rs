// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters,
    scheduler::{DependencyResult, DependencyStatus, Scheduler},
    task::Transaction,
    txn_last_input_output::ReadDescriptor,
};
use anyhow::{bail, Result};
use aptos_aggregator::delta_change_set::serialize;
use aptos_logger::error;
use aptos_mvhashmap::{
    types::{MVDataError, MVDataOutput, MVGroupError, MVModulesError, MVModulesOutput, TxnIndex},
    unsync_map::UnsyncMap,
    MVHashMap,
};
use aptos_state_view::{StateViewId, TStateView};
use aptos_types::{
    executable::{Executable, ModulePath},
    state_store::{state_storage_usage::StateStorageUsage, state_value::StateValue},
    vm_status::{StatusCode, VMStatus},
    write_set::TransactionWrite,
};
use aptos_vm_logging::{log_schema::AdapterLogSchema, prelude::*};
use aptos_vm_types::resolver::TResourceGroupResolver;
use claims::assert_none;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fmt::Debug,
    sync::{atomic::AtomicU32, Arc},
};

fn execution_halted_error() -> anyhow::Error {
    anyhow::Error::new(VMStatus::error(
        StatusCode::STORAGE_ERROR,
        Some("Speculative error to halt BlockSTM early.".to_string()),
    ))
}

fn process_group_read_result(
    read_result: GroupReadResult,
) -> anyhow::Result<(Option<Vec<u8>>, Option<usize>)> {
    use GroupReadResult::*;

    match read_result {
        Value(v) => Ok(v),
        ExecutionHalted => Err(execution_halted_error()),
        TagSerializationError => {
            bail!("Resource group member tag serialization error")
        },
        NotInitialized => unreachable!("Group must already be initialized"),
    }
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
    // Parallel execution halts.
    ExecutionHalted,
    // Read did not return anything.
    None,
}

// Mimicking read result for local processing purposes.
enum GroupReadResult {
    // Successful read of value and optionally, of group size.
    Value((Option<Vec<u8>>, Option<usize>)),
    // Parallel execution halts.
    ExecutionHalted,
    // Tag serialization error.
    TagSerializationError,
    // Needing initialization.
    NotInitialized,
}

pub(crate) struct ParallelState<'a, T: Transaction, X: Executable> {
    pub(crate) versioned_map: &'a MVHashMap<T::Key, T::Tag, T::Value, X>,
    scheduler: &'a Scheduler,
    _counter: &'a AtomicU32,
    captured_reads: RefCell<Vec<ReadDescriptor<T::Key, T::Tag>>>,
}

impl<'a, T: Transaction, X: Executable> ParallelState<'a, T, X> {
    pub(crate) fn new(
        shared_map: &'a MVHashMap<T::Key, T::Tag, T::Value, X>,
        shared_scheduler: &'a Scheduler,
        shared_counter: &'a AtomicU32,
    ) -> Self {
        Self {
            versioned_map: shared_map,
            scheduler: shared_scheduler,
            _counter: shared_counter,
            captured_reads: RefCell::new(Vec::new()),
        }
    }

    // TODO: Actually fill in the logic to record fetched executables, etc.
    fn fetch_module(
        &self,
        key: &T::Key,
        txn_idx: TxnIndex,
    ) -> anyhow::Result<MVModulesOutput<T::Value, X>, MVModulesError> {
        // Register a fake read for the read / write path intersection fallback for modules.
        self.captured_reads
            .borrow_mut()
            .push(ReadDescriptor::from_module(key.clone()));

        self.versioned_map.modules().fetch_module(key, txn_idx)
    }

    /// Captures a read from the VM execution, but not unresolved deltas, as in this case it is the
    /// callers responsibility to set the aggregator's base value and call fetch_data again.
    fn fetch_data(&self, key: &T::Key, txn_idx: TxnIndex) -> ReadResult<T::Value> {
        use MVDataError::*;
        use MVDataOutput::*;

        loop {
            match self.versioned_map.data().fetch_data(key, txn_idx) {
                Ok(Versioned(version, v)) => {
                    let (idx, incarnation) = version;
                    self.captured_reads
                        .borrow_mut()
                        .push(ReadDescriptor::from_version(
                            key.clone(),
                            None,
                            idx,
                            incarnation,
                        ));
                    return ReadResult::Value(v);
                },
                Ok(Resolved(value)) => {
                    self.captured_reads
                        .borrow_mut()
                        .push(ReadDescriptor::from_resolved(key.clone(), value));
                    return ReadResult::U128(value);
                },
                Err(NotFound) => {
                    self.captured_reads
                        .borrow_mut()
                        .push(ReadDescriptor::from_storage(key.clone(), None));
                    return ReadResult::None;
                },
                Err(Unresolved(_)) => return ReadResult::Unresolved,
                Err(Dependency(dep_idx)) => {
                    if !self.wait_for_dependency(txn_idx, dep_idx) {
                        return ReadResult::ExecutionHalted;
                    }
                },
                Err(DeltaApplicationFailure) => {
                    // Delta application failure currently should never happen. Here, we assume it
                    // happened because of speculation and return 0 to the Move-VM. Validation will
                    // ensure the transaction re-executes if 0 wasn't the right number.
                    self.captured_reads
                        .borrow_mut()
                        .push(ReadDescriptor::from_delta_application_failure(key.clone()));
                    return ReadResult::U128(0);
                },
            };
        }
    }

    // txn_idx is estimated to have a r/w dependency on dep_idx.
    // Returns after the dependency has been resolved, the returned indicator is true if
    // it is safe to continue, and false if the execution has been halted.
    fn wait_for_dependency(&self, txn_idx: TxnIndex, dep_idx: TxnIndex) -> bool {
        match self.scheduler.wait_for_dependency(txn_idx, dep_idx) {
            DependencyResult::Dependency(dep_condition) => {
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
                while let DependencyStatus::Unresolved = *dep_resolved {
                    dep_resolved = cvar.wait(dep_resolved).unwrap();
                }
                // dep resolved status is either resolved or execution halted.
                matches!(*dep_resolved, DependencyStatus::Resolved)
            },
            DependencyResult::ExecutionHalted => false,
            DependencyResult::Resolved => true,
        }
    }

    fn get_resource_from_group(
        &self,
        txn_idx: TxnIndex,
        key: &T::Key,
        tag: &T::Tag,
        return_group_size: bool,
        check_existence: bool,
    ) -> GroupReadResult {
        if check_existence {
            assert!(
                !return_group_size,
                "Group size should not be requested when checking existence"
            );
        }

        loop {
            match self.versioned_map.group_data().read_from_group(
                key,
                txn_idx,
                tag,
                return_group_size,
            ) {
                Ok((res, group_size, maybe_version)) => {
                    if return_group_size {
                        // Register a read to make sure size of a group is validated.
                        // (gas charging may depend on it).
                        self.captured_reads
                            .borrow_mut()
                            .push(ReadDescriptor::from_group_size(
                                key.clone(),
                                group_size.expect("group size must be returned"),
                            ));
                    } else {
                        assert_none!(group_size, "not asked to return group size");
                    }

                    if check_existence {
                        // Only record exitence query.
                        self.captured_reads
                            .borrow_mut()
                            .push(ReadDescriptor::from_exists(
                                key.clone(),
                                Some(tag.clone()),
                                !res.is_deletion(),
                            ));
                    } else {
                        match maybe_version {
                            // Record the versioned read.
                            Some((idx, incarnation)) => {
                                self.captured_reads
                                    .borrow_mut()
                                    .push(ReadDescriptor::from_version(
                                        key.clone(),
                                        Some(tag.clone()),
                                        idx,
                                        incarnation,
                                    ))
                            },
                            None => self
                                .captured_reads
                                .borrow_mut()
                                .push(ReadDescriptor::from_storage(key.clone(), Some(tag.clone()))),
                        }
                    }

                    return GroupReadResult::Value((res.extract_raw_bytes(), group_size));
                },
                Err(MVGroupError::NotFound) => {
                    if check_existence {
                        self.captured_reads
                            .borrow_mut()
                            .push(ReadDescriptor::from_exists(
                                key.clone(),
                                Some(tag.clone()),
                                false,
                            ));
                    }
                    return GroupReadResult::Value((None, None));
                },
                Err(MVGroupError::Dependency(dep_idx)) => {
                    if !self.wait_for_dependency(txn_idx, dep_idx) {
                        return GroupReadResult::ExecutionHalted;
                    }
                },
                Err(MVGroupError::TagSerializationError) => {
                    return GroupReadResult::TagSerializationError
                },
                Err(MVGroupError::NotInitialized) => return GroupReadResult::NotInitialized,
            }
        }
    }
}

pub(crate) struct SequentialState<'a, T: Transaction, X: Executable> {
    unsync_map: &'a UnsyncMap<T::Key, T::Value, X>,
    _counter: &'a RefCell<u32>,
}

impl<'a, T: Transaction, X: Executable> SequentialState<'a, T, X> {
    pub(crate) fn new(
        unsync_map: &'a UnsyncMap<T::Key, T::Value, X>,
        shared_counter: &'a RefCell<u32>,
    ) -> Self {
        Self {
            unsync_map,
            _counter: shared_counter,
        }
    }
}

pub(crate) enum ViewState<'a, T: Transaction, X: Executable> {
    Sync(ParallelState<'a, T, X>),
    Unsync(SequentialState<'a, T, X>),
}

/// A struct that represents a single block execution worker thread's view into the state,
/// some of which (in Sync case) might be shared with other workers / threads. By implementing
/// all necessary traits, LatestView is provided to the VM and used to intercept the reads.
/// In the Sync case, also records captured reads for later validation. latest_txn_idx
/// must be set according to the latest transaction that the worker was / is executing.
///
/// LatestView instance should not be re-used among different transactions or incarnations of
/// the same transaction, as parts of the state will not cleaned up (and APIs to do so, including
/// to update the txn_idx, are thus deliberately not exposed). Instead, LatestView should be
/// created from scratch (initialization is cheap as it's based on passing references).
pub(crate) struct LatestView<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> {
    base_view: &'a S,
    latest_view: ViewState<'a, T, X>,
    txn_idx: TxnIndex,
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> LatestView<'a, T, S, X> {
    pub(crate) fn new(
        base_view: &'a S,
        latest_view: ViewState<'a, T, X>,
        txn_idx: TxnIndex,
    ) -> Self {
        Self {
            base_view,
            latest_view,
            txn_idx,
        }
    }

    /// Drains the captured reads.
    pub(crate) fn take_reads(&self) -> Vec<ReadDescriptor<T::Key, T::Tag>> {
        match &self.latest_view {
            ViewState::Sync(state) => state.captured_reads.take(),
            ViewState::Unsync(_) => {
                unreachable!("Take reads called in sequential setting (not captured)")
            },
        }
    }

    fn get_base_value(&self, state_key: &T::Key) -> anyhow::Result<Option<StateValue>> {
        let ret = self.base_view.get_state_value(state_key);

        if ret.is_err() {
            // Even speculatively, reading from base view should not return an error.
            // Thus, this critical error log and count does not need to be buffered.
            let log_context = AdapterLogSchema::new(self.base_view.id(), self.txn_idx as usize);
            alert!(
                log_context,
                "[VM, StateView] Error getting data from storage for {:?}",
                state_key
            );
        }
        ret
    }

    // Returns true if initialization occured.
    fn init_group_if_needed(
        &self,
        key: &T::Key,
        read_result: &GroupReadResult,
    ) -> anyhow::Result<bool> {
        if let ViewState::Sync(state) = &self.latest_view {
            if matches!(read_result, GroupReadResult::NotInitialized) {
                let tree: BTreeMap<T::Tag, Vec<u8>> = self
                    .base_view
                    .get_state_value_bytes(key)?
                    .as_ref()
                    .map(|group_data_blob| {
                        bcs::from_bytes(group_data_blob)
                            .map_err(|_| anyhow::Error::msg("Resource group deserialization error"))
                    })
                    .unwrap_or(Ok(BTreeMap::new()))?;
                state.versioned_map.group_data().initialize_group(
                    key.clone(),
                    tree.into_iter()
                        .map(|(tag, bytes)| (tag, TransactionWrite::creation_from_bytes(bytes))),
                );

                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            unreachable!("Must only be called for Sync / MVHashMap");
        }
    }
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> TResourceGroupResolver
    for LatestView<'a, T, S, X>
{
    type Key = T::Key;
    type Tag = T::Tag;

    fn get_resource_from_group(
        &self,
        key: &T::Key,
        tag: &T::Tag,
        return_group_size: bool,
    ) -> anyhow::Result<(Option<Vec<u8>>, Option<usize>)> {
        match &self.latest_view {
            ViewState::Sync(state) => {
                let mut mv_result =
                    state.get_resource_from_group(self.txn_idx, key, tag, return_group_size, false);

                if self.init_group_if_needed(key, &mv_result)? {
                    mv_result = state.get_resource_from_group(
                        self.txn_idx,
                        key,
                        tag,
                        return_group_size,
                        false,
                    );
                }
                process_group_read_result(mv_result)
            },
            ViewState::Unsync(_) => {
                let maybe_group_data = self.get_state_value_bytes(key)?;
                Ok(match maybe_group_data {
                    Some(group_data) => {
                        let mut group_data: BTreeMap<T::Tag, Vec<u8>> =
                            bcs::from_bytes(&group_data)?;

                        let maybe_group_size = return_group_size.then_some(
                            group_data
                                .iter()
                                .try_fold(0, |len, (tag, res)| {
                                    let delta = bcs::serialized_size(tag)? + res.len();
                                    Ok(len + delta)
                                })
                                .map_err(|_: anyhow::Error| {
                                    anyhow::Error::msg(
                                        "Resource group member tag serialization error",
                                    )
                                })?,
                        );

                        (group_data.remove(tag), maybe_group_size)
                    },
                    None => (None, None),
                })
            },
        }
    }

    fn resource_exists_within_group(
        &self,
        key: &Self::Key,
        tag: &Self::Tag,
    ) -> anyhow::Result<bool> {
        match &self.latest_view {
            ViewState::Sync(state) => {
                let mut mv_result =
                    state.get_resource_from_group(self.txn_idx, key, tag, false, true);

                if self.init_group_if_needed(key, &mv_result)? {
                    mv_result = state.get_resource_from_group(self.txn_idx, key, tag, false, true);
                }
                process_group_read_result(mv_result).map(|(res, _)| res.is_some())
            },
            ViewState::Unsync(_) => self
                .get_resource_from_group(key, tag, false)
                .map(|(res, _)| res.is_some()),
        }
    }
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> TStateView
    for LatestView<'a, T, S, X>
{
    type Key = T::Key;

    fn get_state_value(&self, state_key: &T::Key) -> anyhow::Result<Option<StateValue>> {
        match &self.latest_view {
            ViewState::Sync(state) => match state_key.module_path() {
                Some(_) => {
                    use MVModulesError::*;
                    use MVModulesOutput::*;

                    match state.fetch_module(state_key, self.txn_idx) {
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
                    let mut mv_value = state.fetch_data(state_key, self.txn_idx);

                    if matches!(mv_value, ReadResult::Unresolved) {
                        let from_storage = self
                            .base_view
                            .get_state_value_u128(state_key)?
                            .ok_or(VMStatus::error(StatusCode::STORAGE_ERROR, None))?;

                        // Store base value in the versioned data-structure directly, so subsequent
                        // reads can be resolved to U128 directly without storage calls.
                        state
                            .versioned_map
                            .data()
                            .set_aggregator_base_value(state_key, from_storage);

                        mv_value = state.fetch_data(state_key, self.txn_idx);
                    }

                    match mv_value {
                        ReadResult::Value(v) => Ok(v.as_state_value()),
                        ReadResult::U128(v) => Ok(Some(StateValue::new_legacy(serialize(&v)))),
                        // ExecutionHalted indicates that the parallel execution is halted.
                        // The read should return immediately and log the error.
                        // For now we use STORAGE_ERROR as the VM will not log the speculative eror,
                        // so no actual error will be logged once the execution is halted and
                        // the speculative logging is flushed.
                        ReadResult::ExecutionHalted => Err(execution_halted_error()),
                        ReadResult::None => self.get_base_value(state_key),
                        ReadResult::Unresolved => unreachable!(
                            "Must be resolved as base value is recorded in the MV data structure"
                        ),
                    }
                },
            },
            ViewState::Unsync(state) => state.unsync_map.fetch_data(state_key).map_or_else(
                || self.get_base_value(state_key),
                |v| Ok(v.as_state_value()),
            ),
        }
    }

    fn id(&self) -> StateViewId {
        self.base_view.id()
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        self.base_view.get_usage()
    }
}
