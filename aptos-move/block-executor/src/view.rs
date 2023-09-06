// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters,
    liftings::{SyncLiftings, UnsyncLiftings},
    scheduler::{DependencyResult, DependencyStatus, Scheduler},
    task::Transaction,
    txn_last_input_output::ReadDescriptor,
};
use anyhow::Result;
use aptos_aggregator::delta_change_set::serialize;
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
    vm_status::{StatusCode, VMStatus},
    write_set::TransactionWrite,
};
use aptos_vm_logging::{log_schema::AdapterLogSchema, prelude::*};
use dashmap::DashMap;
use move_core_types::value::MoveTypeLayout;
use move_vm_types::{
    value_transformation::{
        deserialize_and_exchange, AsIdentifier, IdentifierBuilder, TransformationError,
        TransformationResult, ValueExchange,
    },
    values::Value,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::Debug,
    sync::{atomic::AtomicU64, Arc},
};

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

pub(crate) struct ParallelState<'a, T: Transaction, X: Executable> {
    versioned_map: &'a MVHashMap<T::Key, T::Value, X>,
    scheduler: &'a Scheduler,
    liftings: SyncLiftings<'a>,
    lifted_cache: DashMap<T::Key, StateValue>,
    captured_reads: RefCell<Vec<ReadDescriptor<T::Key>>>,
}

impl<'a, T: Transaction, X: Executable> ParallelState<'a, T, X> {
    pub(crate) fn new(
        shared_map: &'a MVHashMap<T::Key, T::Value, X>,
        shared_scheduler: &'a Scheduler,
        shared_counter: &'a AtomicU64,
    ) -> Self {
        Self {
            versioned_map: shared_map,
            scheduler: shared_scheduler,
            liftings: SyncLiftings::new(shared_counter),
            lifted_cache: DashMap::new(),
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

        self.versioned_map.fetch_module(key, txn_idx)
    }

    fn set_aggregator_base_value(&self, key: &T::Key, value: u128) {
        self.versioned_map.set_aggregator_base_value(key, value);
    }

    /// Captures a read from the VM execution, but not unresolved deltas, as in this case it is the
    /// callers responsibility to set the aggregator's base value and call fetch_data again.
    fn fetch_data(&self, key: &T::Key, txn_idx: TxnIndex) -> ReadResult<T::Value> {
        use MVDataError::*;
        use MVDataOutput::*;

        loop {
            match self.versioned_map.fetch_data(key, txn_idx) {
                Ok(Versioned(version, v)) => {
                    let (idx, incarnation) = version;
                    self.captured_reads
                        .borrow_mut()
                        .push(ReadDescriptor::from_version(key.clone(), idx, incarnation));
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
                        .push(ReadDescriptor::from_storage(key.clone()));
                    return ReadResult::None;
                },
                Err(Unresolved(_)) => return ReadResult::Unresolved,
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
                                return ReadResult::ExecutionHalted;
                            }
                        },
                        DependencyResult::ExecutionHalted => {
                            return ReadResult::ExecutionHalted;
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
                        .push(ReadDescriptor::from_delta_application_failure(key.clone()));
                    return ReadResult::U128(0);
                },
            };
        }
    }
}

pub(crate) struct SequentialState<'a, T: Transaction, X: Executable> {
    unsync_map: &'a UnsyncMap<T::Key, T::Value, X>,
    unsync_liftings: RefCell<UnsyncLiftings>,
    lifted_cache: RefCell<HashMap<T::Key, StateValue>>,
}

impl<'a, T: Transaction, X: Executable> SequentialState<'a, T, X> {
    pub(crate) fn new(unsync_map: &'a UnsyncMap<T::Key, T::Value, X>, counter: u64) -> Self {
        Self {
            unsync_map,
            unsync_liftings: RefCell::new(UnsyncLiftings::new(counter)),
            lifted_cache: RefCell::new(HashMap::new()),
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

    #[allow(unused)]
    fn get_base_value_with_layout(
        &self,
        layout: &MoveTypeLayout,
        state_key: &T::Key,
    ) -> anyhow::Result<Option<StateValue>> {
        let process_lifting = |bytes: Vec<u8>| -> anyhow::Result<Vec<u8>> {
            // TODO: alert on fail?
            let patched_value = deserialize_and_exchange(&bytes, layout, self)
                .ok_or_else(|| anyhow::anyhow!("Failed to deserialize for {:?}", state_key))?;
            patched_value
                .simple_serialize(layout)
                .ok_or_else(|| anyhow::anyhow!("Failed to serialize value {}", patched_value))
        };

        // TODO: deduplicate this logic.
        match &self.latest_view {
            ViewState::Sync(state) => match state.lifted_cache.get(state_key) {
                Some(state_value) => Ok(Some(state_value.clone())),
                None => {
                    let maybe_state_value = self.get_base_value(state_key)?;
                    match maybe_state_value {
                        None => Ok(None),
                        Some(state_value) => {
                            let state_value = state_value.try_transform_bytes(process_lifting)?;
                            state
                                .lifted_cache
                                .insert(state_key.clone(), state_value.clone());
                            Ok(Some(state_value))
                        },
                    }
                },
            },
            ViewState::Unsync(state) => {
                let mut lifted_cache = state.lifted_cache.borrow_mut();
                match lifted_cache.get(state_key) {
                    Some(state_value) => Ok(Some(state_value.clone())),
                    None => {
                        let maybe_state_value = self.get_base_value(state_key)?;
                        match maybe_state_value {
                            None => Ok(None),
                            Some(state_value) => {
                                let state_value =
                                    state_value.try_transform_bytes(process_lifting)?;
                                lifted_cache.insert(state_key.clone(), state_value.clone());
                                Ok(Some(state_value))
                            },
                        }
                    },
                }
            },
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
                        state.set_aggregator_base_value(state_key, from_storage);

                        mv_value = state.fetch_data(state_key, self.txn_idx);
                    }

                    match mv_value {
                        ReadResult::Value(v) => Ok(v.as_state_value()),
                        ReadResult::U128(v) => Ok(Some(StateValue::new_legacy(serialize(&v)))),
                        // ExecutionHalted indicates that the parallel execution is halted.
                        // The read should return immediately and log the error.
                        // For now we use STORAGE_ERROR as the VM will not log the speculative error,
                        // so no actual error will be logged once the execution is halted and
                        // the speculative logging is flushed.
                        ReadResult::ExecutionHalted => Err(anyhow::Error::new(VMStatus::error(
                            StatusCode::STORAGE_ERROR,
                            Some("Speculative error to halt BlockSTM early.".to_string()),
                        ))),
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

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> ValueExchange
    for LatestView<'a, T, S, X>
{
    fn try_exchange(
        &self,
        layout: &MoveTypeLayout,
        value_to_exchange: Value,
    ) -> TransformationResult<Value> {
        let identifier = match &self.latest_view {
            ViewState::Sync(state) => state.liftings.insert(value_to_exchange),
            ViewState::Unsync(state) => {
                let mut liftings = state.unsync_liftings.borrow_mut();
                liftings.insert(value_to_exchange)
            },
        };
        Value::embed_identifier(layout, identifier).ok_or_else(|| {
            TransformationError::new(&format!("Cannot embed identifier for {}", layout))
        })
    }

    fn try_claim_back(&self, value_to_exchange: Value) -> TransformationResult<Value> {
        let identifier = value_to_exchange.as_identifier().ok_or_else(|| {
            TransformationError::new(&format!(
                "Value {} cannot be an identifier",
                value_to_exchange,
            ))
        })?;

        let post_process = |maybe_value: Option<&Value>| -> TransformationResult<Value> {
            let value = maybe_value.ok_or_else(|| {
                TransformationError::new(&format!("Identifier {} does not exist", identifier))
            })?;
            value
                .copy_value()
                .map_err(|_| TransformationError::new(&format!("Failed to copy value {}", value)))
        };

        match &self.latest_view {
            ViewState::Sync(state) => post_process(state.liftings.get(identifier).as_deref()),
            ViewState::Unsync(state) => {
                let liftings = state.unsync_liftings.borrow();
                post_process(liftings.get(identifier))
            },
        }
    }
}
