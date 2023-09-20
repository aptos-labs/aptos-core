// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters,
    scheduler::{DependencyResult, DependencyStatus, Scheduler},
    task::Transaction,
    txn_last_input_output::ReadDescriptor,
};
use aptos_aggregator::{
    delta_change_set::serialize,
    resolver::{AggregatorReadMode, TAggregatorView},
};
use aptos_logger::error;
use aptos_mvhashmap::{
    types::{MVDataError, MVDataOutput, MVModulesError, MVModulesOutput, TxnIndex},
    unsync_map::UnsyncMap,
    MVHashMap,
};
use aptos_state_view::{StateViewId, TStateView};
use aptos_types::{
    executable::{Executable, ModulePath},
    state_store::{state_storage_usage::StateStorageUsage, state_value::StateValue},
    write_set::TransactionWrite,
};
use aptos_vm_logging::{log_schema::AdapterLogSchema, prelude::*};
use aptos_vm_types::resolver::{StateStorageView, TModuleView, TResourceView};
use move_core_types::{
    value::MoveTypeLayout,
    vm_status::{StatusCode, VMStatus},
};
use std::{
    cell::RefCell,
    fmt::Debug,
    sync::{atomic::AtomicU32, Arc},
};

/// A struct which describes the result of the read from the proxy. The client
/// can interpret these types to further resolve the reads.
#[derive(Debug)]
pub(crate) enum ReadResult<V> {
    // Successful read of a value.
    Value(Arc<V>, Option<Arc<MoveTypeLayout>>),
    // Similar to above, but the value was aggregated and is an integer.
    U128(u128),
    // Read did not return anything.
    Uninitialized,
    // Must half the execution of the calling transaction. This might be because
    // there was an inconsistency in observed speculative state, or dependency
    // waiting indicated that the parallel execution had been halted. The String
    // parameter provides more context (error description / message).
    HaltSpeculativeExecution(String),
}

pub(crate) struct ParallelState<'a, T: Transaction, X: Executable> {
    versioned_map: &'a MVHashMap<T::Key, T::Tag, T::Value, X, T::Identifier>,
    scheduler: &'a Scheduler,
    _counter: &'a AtomicU32,
    captured_reads: RefCell<Vec<ReadDescriptor<T::Key>>>,
}

impl<'a, T: Transaction, X: Executable> ParallelState<'a, T, X> {
    pub(crate) fn new(
        shared_map: &'a MVHashMap<T::Key, T::Tag, T::Value, X, T::Identifier>,
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
                Ok(Versioned(version, v, layout)) => {
                    self.captured_reads
                        .borrow_mut()
                        .push(ReadDescriptor::from_versioned(key.clone(), version));
                    return ReadResult::Value(v, layout);
                },
                Ok(Resolved(value)) => {
                    self.captured_reads
                        .borrow_mut()
                        .push(ReadDescriptor::from_resolved(key.clone(), value));
                    return ReadResult::U128(value);
                },
                Err(Uninitialized) | Err(Unresolved(_)) => {
                    // The underlying assumption here for not recording anything about the read is
                    // that the caller is expected to initialize the contents and serve the reads
                    // solely via the 'fetch_read' interface. Thus, the later, successful read,
                    // will make the needed recordings.
                    return ReadResult::Uninitialized;
                },
                Err(Dependency(dep_idx)) => {
                    // `self.txn_idx` estimated to depend on a write from `dep_idx`.
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
                            if let DependencyStatus::ExecutionHalted = *dep_resolved {
                                return ReadResult::HaltSpeculativeExecution(
                                    "Speculative error to halt BlockSTM early.".to_string(),
                                );
                            }
                        },
                        DependencyResult::ExecutionHalted => {
                            return ReadResult::HaltSpeculativeExecution(
                                "Speculative error to halt BlockSTM early.".to_string(),
                            );
                        },
                        DependencyResult::Resolved => continue,
                    }
                },
                Err(DeltaApplicationFailure) => {
                    // Delta application failure currently should never happen. Here, we assume it
                    // happened because of speculation and return 0 to the Move-VM. Validation will
                    // ensure the transaction re-executes if 0 wasn't the right number.

                    self.captured_reads
                        .borrow_mut()
                        .push(ReadDescriptor::from_speculative_failure(key.clone()));

                    return ReadResult::HaltSpeculativeExecution(
                        "Delta application failure (must be speculative)".to_string(),
                    );
                },
            };
        }
    }
}

pub(crate) struct SequentialState<'a, T: Transaction, X: Executable> {
    pub(crate) unsync_map: &'a UnsyncMap<T::Key, T::Value, X, T::Identifier>,
    pub(crate) _counter: &'a u32,
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
    pub(crate) fn take_reads(&self) -> Vec<ReadDescriptor<T::Key>> {
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
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> TResourceView
    for LatestView<'a, T, S, X>
{
    type Key = T::Key;
    type Layout = MoveTypeLayout;

    fn get_resource_state_value(
        &self,
        state_key: &Self::Key,
        _maybe_layout: Option<&Self::Layout>,
    ) -> anyhow::Result<Option<StateValue>> {
        debug_assert!(
            state_key.module_path().is_none(),
            "Reading a module {:?} using ResourceView",
            state_key,
        );

        match &self.latest_view {
            ViewState::Sync(state) => {
                let mut mv_value = state.fetch_data(state_key, self.txn_idx);

                if matches!(mv_value, ReadResult::Uninitialized) {
                    let from_storage = self.base_view.get_state_value(state_key)?;

                    // This base value can also be used to resolve AggregatorV1 directly from
                    // the versioned data-structure (without more storage calls).
                    state.versioned_map.data().provide_base_value(
                        state_key.clone(),
                        TransactionWrite::from_state_value(from_storage),
                    );

                    mv_value = state.fetch_data(state_key, self.txn_idx);
                }

                match mv_value {
                    ReadResult::Value(v, _) => Ok(v.as_ref().as_state_value()),
                    ReadResult::U128(v) => Ok(Some(StateValue::new_legacy(serialize(&v).into()))),
                    // ExecutionHalted indicates that the parallel execution is halted.
                    // The read should return immediately and log the error.
                    // For now we use STORAGE_ERROR as the VM will not log the speculative eror,
                    // so no actual error will be logged once the execution is halted and
                    // the speculative logging is flushed.
                    ReadResult::HaltSpeculativeExecution(msg) => Err(anyhow::Error::new(
                        VMStatus::error(StatusCode::STORAGE_ERROR, Some(msg)),
                    )),
                    ReadResult::Uninitialized => {
                        unreachable!("base value must already be recorded in the MV data structure")
                    },
                }
            },
            ViewState::Unsync(state) => state.unsync_map.fetch_data(state_key).map_or_else(
                || {
                    // TODO: AggregatorV2 ID for sequential must be replaced in this flow.
                    self.get_base_value(state_key)
                },
                |v| Ok(v.as_state_value()),
            ),
        }
    }

    // TODO: implement here fn get_resource_state_value_metadata & resource_exists.
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> TModuleView
    for LatestView<'a, T, S, X>
{
    type Key = T::Key;

    fn get_module_state_value(&self, state_key: &Self::Key) -> anyhow::Result<Option<StateValue>> {
        debug_assert!(
            state_key.module_path().is_some(),
            "Reading a resource {:?} using ModuleView",
            state_key,
        );

        match &self.latest_view {
            ViewState::Sync(state) => {
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
            ViewState::Unsync(state) => state.unsync_map.fetch_data(state_key).map_or_else(
                || self.get_base_value(state_key),
                |v| Ok(v.as_state_value()),
            ),
        }
    }
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> StateStorageView
    for LatestView<'a, T, S, X>
{
    fn id(&self) -> StateViewId {
        self.base_view.id()
    }

    fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
        self.base_view.get_usage()
    }
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> TAggregatorView
    for LatestView<'a, T, S, X>
{
    type IdentifierV1 = T::Key;
    type IdentifierV2 = T::Identifier;

    fn get_aggregator_v1_state_value(
        &self,
        state_key: &Self::IdentifierV1,
        _mode: AggregatorReadMode,
    ) -> anyhow::Result<Option<StateValue>> {
        // TODO: Integrate aggregators.
        self.get_resource_state_value(state_key, None)
    }

    fn get_aggregator_v2_value(
        &self,
        id: &Self::IdentifierV2,
        mode: AggregatorReadMode,
    ) -> anyhow::Result<aptos_aggregator::types::AggregatorValue> {
        match &self.latest_view {
            ViewState::Sync(state) => {
                let result = match mode {
                    AggregatorReadMode::Aggregated => {
                        state.versioned_map.aggregators().read(*id, self.txn_idx)
                    },
                    AggregatorReadMode::LastCommitted => state
                        .versioned_map
                        .aggregators()
                        .read_latest_committed_value(*id),
                };
                result.map_err(|e| {
                    anyhow::Error::new(VMStatus::error(
                        StatusCode::STORAGE_ERROR,
                        Some(format!("Error during read: {:?}", e)),
                    ))
                })
            },
            ViewState::Unsync(state) => state.unsync_map.fetch_aggregator(id).ok_or_else(|| {
                anyhow::Error::new(VMStatus::error(
                    StatusCode::STORAGE_ERROR,
                    Some("Aggregator doesn't exist".to_string()),
                ))
            }),
        }
    }
}
