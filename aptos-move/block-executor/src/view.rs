// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    captured_reads::{CapturedReads, DataRead, ReadKind},
    counters,
    scheduler::{DependencyResult, DependencyStatus, Scheduler},
};
use aptos_aggregator::{
    delta_change_set::serialize,
    resolver::{DelayedFieldReadMode, TDelayedFieldView},
    types::{DelayedFieldValue, PanicOr, ReadPosition, TryFromMoveValue, TryIntoMoveValue},
};
use aptos_logger::error;
use aptos_mvhashmap::{
    types::{
        MVDataError, MVDataOutput, MVDelayedFieldsError, MVModulesError, MVModulesOutput, TxnIndex,
    },
    unsync_map::UnsyncMap,
    MVHashMap,
};
use aptos_state_view::{StateViewId, TStateView};
use aptos_types::{
    executable::{Executable, ModulePath},
    state_store::{
        state_storage_usage::StateStorageUsage,
        state_value::{StateValue, StateValueMetadataKind},
    },
    transaction::BlockExecutableTransaction as Transaction,
    write_set::TransactionWrite,
};
use aptos_vm_logging::{log_schema::AdapterLogSchema, prelude::*};
use aptos_vm_types::resolver::{StateStorageView, TModuleView, TResourceView};
use bytes::Bytes;
use move_core_types::{
    value::{IdentifierMappingKind, MoveTypeLayout},
    vm_status::{StatusCode, VMStatus},
};
use move_vm_types::{
    value_transformation::{
        deserialize_and_replace_values_with_ids, serialize_and_replace_ids_with_values,
        TransformationError, TransformationResult, ValueToIdentifierMapping,
    },
    values::Value,
};
use std::{
    cell::RefCell,
    collections::HashSet,
    fmt::Debug,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

/// A struct which describes the result of the read from the proxy. The client
/// can interpret these types to further resolve the reads.
#[derive(Debug)]
pub(crate) enum ReadResult {
    Value(Option<StateValue>, Option<Arc<MoveTypeLayout>>),
    Metadata(Option<StateValueMetadataKind>),
    Exists(bool),
    Uninitialized,
    // Must halt the execution of the calling transaction. This might be because
    // there was an inconsistency in observed speculative state, or dependency
    // waiting indicated that the parallel execution had been halted. The String
    // parameter provides more context (error description / message).
    HaltSpeculativeExecution(String),
}

impl ReadResult {
    fn from_data_read<V: TransactionWrite>(data: DataRead<V>) -> Self {
        match data {
            DataRead::Versioned(_, v, layout) => ReadResult::Value(v.as_state_value(), layout),
            DataRead::Resolved(v) => {
                // TODO confirm None layout for V1 aggregators is OK
                ReadResult::Value(Some(StateValue::new_legacy(serialize(&v).into())), None)
            },
            DataRead::Metadata(maybe_metadata) => ReadResult::Metadata(maybe_metadata),
            DataRead::Exists(exists) => ReadResult::Exists(exists),
        }
    }
}

pub(crate) struct ParallelState<'a, T: Transaction, X: Executable> {
    versioned_map: &'a MVHashMap<T::Key, T::Tag, T::Value, X, T::Identifier>,
    scheduler: &'a Scheduler,
    counter: &'a AtomicU32,
    captured_reads: RefCell<CapturedReads<T>>,
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
            counter: shared_counter,
            captured_reads: RefCell::new(CapturedReads::new()),
        }
    }

    fn set_delayed_field_value(&self, id: T::Identifier, base_value: DelayedFieldValue) {
        self.versioned_map
            .delayed_fields()
            .set_base_value(id, base_value)
    }

    fn read_delayed_field_last_committed_value(
        &self,
        id: T::Identifier,
        txn_idx: TxnIndex,
        read_position: ReadPosition,
    ) -> Result<DelayedFieldValue, MVDelayedFieldsError> {
        self.versioned_map
            .delayed_fields()
            .read_latest_committed_value(id, txn_idx, read_position)
    }

    fn read_delayed_field_aggregated_value(
        &self,
        id: T::Identifier,
        txn_idx: TxnIndex,
    ) -> Result<DelayedFieldValue, PanicOr<MVDelayedFieldsError>> {
        match self.versioned_map.delayed_fields().read(id, txn_idx) {
            Ok(value) => {
                self.captured_reads
                    .borrow_mut()
                    .capture_delayed_field_read(id, value.clone())?;
                Ok(value)
            },
            Err(e) => {
                self.captured_reads
                    .borrow_mut()
                    .capture_delayed_field_read_error(&e);
                Err(e)
            },
        }
    }

    // TODO: Actually fill in the logic to record fetched executables, etc.
    fn fetch_module(
        &self,
        key: &T::Key,
        txn_idx: TxnIndex,
    ) -> anyhow::Result<MVModulesOutput<T::Value, X>, MVModulesError> {
        // Record for the R/W path intersection fallback for modules.
        self.captured_reads
            .borrow_mut()
            .module_reads
            .push(key.clone());

        self.versioned_map.modules().fetch_module(key, txn_idx)
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

    /// Captures a read from the VM execution, but not unresolved deltas, as in this case it is the
    /// callers responsibility to set the aggregator's base value and call fetch_data again.
    fn read_data_by_kind(
        &self,
        key: &T::Key,
        txn_idx: TxnIndex,
        target_kind: ReadKind,
    ) -> ReadResult {
        use MVDataError::*;
        use MVDataOutput::*;

        if let Some(data) = self
            .captured_reads
            .borrow()
            .get_by_kind(key, None, target_kind.clone())
        {
            return ReadResult::from_data_read(data);
        }

        loop {
            match self.versioned_map.data().fetch_data(key, txn_idx) {
                Ok(Versioned(version, v, layout)) => {
                    let data_read = DataRead::Versioned(version, v.clone(), layout)
                        .downcast(target_kind)
                        .expect("Downcast from Versioned must succeed");

                    if self
                        .captured_reads
                        .borrow_mut()
                        .capture_read(key.clone(), None, data_read.clone())
                        .is_err()
                    {
                        // Inconsistency in recorded reads.
                        return ReadResult::HaltSpeculativeExecution(
                            "Inconsistency in reads (must be due to speculation)".to_string(),
                        );
                    }

                    return ReadResult::from_data_read(data_read);
                },
                Ok(Resolved(value)) => {
                    let data_read = DataRead::Resolved(value)
                        .downcast(target_kind)
                        .expect("Downcast from Resolved must succeed");

                    if self
                        .captured_reads
                        .borrow_mut()
                        .capture_read(key.clone(), None, data_read.clone())
                        .is_err()
                    {
                        // Inconsistency in recorded reads.
                        return ReadResult::HaltSpeculativeExecution(
                            "Inconsistency in reads (must be due to speculation)".to_string(),
                        );
                    }

                    return ReadResult::from_data_read(data_read);
                },
                Err(Uninitialized) | Err(Unresolved(_)) => {
                    // The underlying assumption here for not recording anything about the read is
                    // that the caller is expected to initialize the contents and serve the reads
                    // solely via the 'fetch_read' interface. Thus, the later, successful read,
                    // will make the needed recordings.
                    return ReadResult::Uninitialized;
                },
                Err(Dependency(dep_idx)) => {
                    if !self.wait_for_dependency(txn_idx, dep_idx) {
                        return ReadResult::HaltSpeculativeExecution(
                            "Interrupted as block execution was halted".to_string(),
                        );
                    }
                },
                Err(DeltaApplicationFailure) => {
                    self.captured_reads.borrow_mut().mark_failure();
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
    pub(crate) counter: &'a RefCell<u32>,
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
    pub(crate) fn take_reads(&self) -> CapturedReads<T> {
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

        // TODO: AggregatorID in V2 can be replaced here.
        ret
    }

    /// Given a state value, performs deserialization-serialization round-trip
    /// to replace any aggregator / snapshot values.
    fn replace_values_with_identifiers(
        &self,
        state_value: StateValue,
        layout: &MoveTypeLayout,
    ) -> anyhow::Result<(StateValue, HashSet<T::Identifier>)> {
        let mapping = TemporaryValueToIdentifierMapping::new(self, self.txn_idx);
        state_value
            .map_bytes(|bytes| {
                // This call will replace all occurrences of aggregator / snapshot
                // values with unique identifiers with the same type layout.
                // The values are stored in aggregators multi-version data structure,
                // see the actual trait implementation for more details.
                let patched_value =
                    deserialize_and_replace_values_with_ids(bytes.as_ref(), layout, &mapping)
                        .ok_or_else(|| {
                            anyhow::anyhow!("Failed to deserialize resource during id replacement")
                        })?;
                patched_value
                    .simple_serialize(layout)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Failed to serialize value {} after id replacement",
                            patched_value
                        )
                    })
                    .map(|b| b.into())
            })
            .map(|v| (v, mapping.into_inner()))
    }

    /// Given a state value, performs deserialization-serialization round-trip
    /// to replace any aggregator / snapshot values.
    pub(crate) fn replace_identifiers_with_values(
        &self,
        bytes: &Bytes,
        layout: &MoveTypeLayout,
    ) -> anyhow::Result<(Bytes, HashSet<T::Identifier>)> {
        // This call will replace all occurrences of aggregator / snapshot
        // identifiers with values with the same type layout.
        let value = Value::simple_deserialize(bytes, layout).ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to deserialize resource during id replacement: {:?}",
                bytes
            )
        })?;
        let mapping = TemporaryValueToIdentifierMapping::new(self, self.txn_idx);
        let patched_bytes = serialize_and_replace_ids_with_values(&value, layout, &mapping)
            .ok_or_else(|| anyhow::anyhow!("Failed to serialize resource during id replacement"))?
            .into();
        Ok((patched_bytes, mapping.into_inner()))
    }

    fn get_resource_state_value_impl(
        &self,
        state_key: &T::Key,
        maybe_layout: Option<&MoveTypeLayout>,
        kind: ReadKind,
    ) -> anyhow::Result<ReadResult> {
        debug_assert!(
            state_key.module_path().is_none(),
            "Reading a module {:?} using ResourceView",
            state_key,
        );

        match &self.latest_view {
            ViewState::Sync(state) => {
                let mut ret = state.read_data_by_kind(state_key, self.txn_idx, kind.clone());

                if matches!(ret, ReadResult::Uninitialized) {
                    let from_storage = self.base_view.get_state_value(state_key)?;
                    let maybe_patched_from_storage = match (from_storage, maybe_layout) {
                        // There are aggregators / aggregator snapshots in the
                        // resource, so we have to replace the actual values with
                        // identifiers.
                        // TODO(aggregator): gate by the flag.
                        (Some(state_value), Some(layout)) => {
                            let res = self.replace_values_with_identifiers(state_value, layout);
                            match res {
                                Ok((value, _)) => Some(value),
                                Err(err) => {
                                    // TODO(aggregator): This means replacement failed
                                    //       and most likely there is a bug. Log the error
                                    //       for now, and add recovery mechanism later.
                                    let log_context = AdapterLogSchema::new(
                                        self.base_view.id(),
                                        self.txn_idx as usize,
                                    );
                                    alert!(
                                        log_context,
                                        "[VM, ResourceView] Error during value to id replacement for {:?}: {}",
                                        state_key,
                                        err
                                    );
                                    None
                                },
                            }
                        },
                        (from_storage, _) => from_storage,
                    };

                    // This base value can also be used to resolve AggregatorV1 directly from
                    // the versioned data-structure (without more storage calls).
                    state.versioned_map.data().provide_base_value(
                        state_key.clone(),
                        TransactionWrite::from_state_value(maybe_patched_from_storage),
                    );

                    ret = state.read_data_by_kind(state_key, self.txn_idx, kind);
                }

                match ret {
                    // ExecutionHalted indicates that the parallel execution is halted.
                    // The read should return immediately and log the error.
                    // For now we use SPECULATIVE_EXECUTION_ABORT_ERROR as the VM
                    // will not log the speculative error,
                    // so no actual error will be logged once the execution is halted and
                    // the speculative logging is flushed.
                    ReadResult::HaltSpeculativeExecution(msg) => Err(anyhow::Error::new(
                        VMStatus::error(StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR, Some(msg)),
                    )),
                    ReadResult::Uninitialized => {
                        unreachable!("base value must already be recorded in the MV data structure")
                    },
                    _ => Ok(ret),
                }
            },
            ViewState::Unsync(state) => {
                let ret = state.unsync_map.fetch_data(state_key).map_or_else(
                    || self.get_base_value(state_key),
                    |v| Ok(v.as_state_value()),
                );
                ret.map(|maybe_state_value| match kind {
                    // TODO: check if we need to track layout for unsync
                    ReadKind::Value => ReadResult::Value(maybe_state_value, None),
                    ReadKind::Metadata => {
                        ReadResult::Metadata(maybe_state_value.map(StateValue::into_metadata))
                    },
                    ReadKind::Exists => ReadResult::Exists(maybe_state_value.is_some()),
                })
            },
        }
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
        maybe_layout: Option<&Self::Layout>,
    ) -> anyhow::Result<Option<StateValue>> {
        self.get_resource_state_value_impl(state_key, maybe_layout, ReadKind::Value)
            .map(|res| {
                if let ReadResult::Value(v, _layout) = res {
                    v
                } else {
                    unreachable!("Read result must be Value kind")
                }
            })
    }

    fn get_resource_state_value_metadata(
        &self,
        state_key: &Self::Key,
    ) -> anyhow::Result<Option<StateValueMetadataKind>> {
        // TODO check that passing None here is correct
        self.get_resource_state_value_impl(state_key, None, ReadKind::Metadata)
            .map(|res| {
                if let ReadResult::Metadata(v) = res {
                    v
                } else {
                    unreachable!("Read result must be Metadata kind")
                }
            })
    }

    fn resource_exists(&self, state_key: &Self::Key) -> anyhow::Result<bool> {
        self.get_resource_state_value_impl(state_key, None, ReadKind::Exists)
            .map(|res| {
                if let ReadResult::Exists(v) = res {
                    v
                } else {
                    unreachable!("Read result must be Exists kind")
                }
            })
    }
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

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> TDelayedFieldView
    for LatestView<'a, T, S, X>
{
    type IdentifierV1 = T::Key;
    type IdentifierV2 = T::Identifier;

    fn get_aggregator_v1_state_value(
        &self,
        state_key: &Self::IdentifierV1,
        _mode: DelayedFieldReadMode,
    ) -> anyhow::Result<Option<StateValue>> {
        // TODO: Integrate aggregators V1. That is, we can lift the u128 value
        //       from the state item by passing the right layout here. This can
        //       be useful for cross-testing the old and the new flows.
        // self.get_resource_state_value(state_key, Some(&MoveTypeLayout::U128))
        self.get_resource_state_value(state_key, None)
    }

    fn get_delayed_field_value(
        &self,
        id: &Self::IdentifierV2,
        mode: DelayedFieldReadMode,
    ) -> anyhow::Result<aptos_aggregator::types::DelayedFieldValue> {
        match &self.latest_view {
            ViewState::Sync(state) => match mode {
                DelayedFieldReadMode::Aggregated => state
                    .read_delayed_field_aggregated_value(*id, self.txn_idx)
                    .map_err(|e| {
                        anyhow::Error::new(VMStatus::error(
                            StatusCode::from(&e),
                            Some(format!("Error during read: {:?}", e)),
                        ))
                    }),
                DelayedFieldReadMode::LastCommitted => state
                    .read_delayed_field_last_committed_value(
                        *id,
                        self.txn_idx,
                        ReadPosition::BeforeCurrentTxn,
                    )
                    .map_err(|e| {
                        anyhow::Error::new(VMStatus::error(
                            StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR,
                            Some(format!("Error during read: {:?}", e)),
                        ))
                    }),
            },
            ViewState::Unsync(state) => state.unsync_map.fetch_delayed_field(id).ok_or_else(|| {
                anyhow::Error::new(VMStatus::error(
                    StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR,
                    Some(format!("Aggregator for id {:?} doesn't exist", id)),
                ))
            }),
        }
    }

    fn generate_delayed_field_id(&self) -> Self::IdentifierV2 {
        match &self.latest_view {
            ViewState::Sync(state) => (state.counter.fetch_add(1, Ordering::SeqCst) as u64).into(),
            ViewState::Unsync(state) => {
                let mut counter = state.counter.borrow_mut();
                let id = (*counter as u64).into();
                *counter += 1;
                id
            },
        }
    }
}

struct TemporaryValueToIdentifierMapping<
    'a,
    T: Transaction,
    S: TStateView<Key = T::Key>,
    X: Executable,
> {
    latest_view: &'a LatestView<'a, T, S, X>,
    txn_idx: TxnIndex,
    // These are the delayed field keys that were touched when utilizing this mapping
    // to replace ids with values or values with ids
    delayed_field_keys: RefCell<HashSet<T::Identifier>>,
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable>
    TemporaryValueToIdentifierMapping<'a, T, S, X>
{
    pub fn new(latest_view: &'a LatestView<'a, T, S, X>, txn_idx: TxnIndex) -> Self {
        Self {
            latest_view,
            txn_idx,
            delayed_field_keys: RefCell::new(HashSet::new()),
        }
    }

    fn generate_delayed_field_id(&self) -> T::Identifier {
        match &self.latest_view.latest_view {
            ViewState::Sync(state) => (state.counter.fetch_add(1, Ordering::SeqCst) as u64).into(),
            ViewState::Unsync(state) => {
                let mut counter = state.counter.borrow_mut();
                let id = (*counter as u64).into();
                *counter += 1;
                id
            },
        }
    }

    pub fn into_inner(self) -> HashSet<T::Identifier> {
        self.delayed_field_keys.borrow().clone()
    }
}

// For aggregators V2, values are replaced with identifiers at deserialization time,
// and are replaced back when the value is serialized. The "lifted" values are cached
// by the `LatestView` in the aggregators multi-version data structure.
impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> ValueToIdentifierMapping
    for TemporaryValueToIdentifierMapping<'a, T, S, X>
{
    fn value_to_identifier(
        &self,
        kind: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
        value: Value,
    ) -> TransformationResult<Value> {
        let id = self.generate_delayed_field_id();
        match &self.latest_view.latest_view {
            ViewState::Sync(state) => {
                let base_value = DelayedFieldValue::try_from_move_value(layout, value, kind)?;
                state.set_delayed_field_value(id, base_value)
            },
            ViewState::Unsync(_state) => {
                // TODO(aggregator): Support sequential execution.
                unimplemented!("Value to ID replacement for sequential execution is not supported")
            },
        };
        self.delayed_field_keys.borrow_mut().insert(id);
        id.try_into_move_value(layout)
            .map_err(|e| TransformationError(format!("{:?}", e)))
    }

    fn identifier_to_value(
        &self,
        layout: &MoveTypeLayout,
        identifier_value: Value,
    ) -> TransformationResult<Value> {
        let id = T::Identifier::try_from_move_value(layout, identifier_value, &())
            .map_err(|e| TransformationError(format!("{:?}", e)))?;
        self.delayed_field_keys.borrow_mut().insert(id);
        match &self.latest_view.latest_view {
            ViewState::Sync(state) => Ok(state
                .read_delayed_field_last_committed_value(
                    id,
                    self.txn_idx,
                    ReadPosition::AfterCurrentTxn,
                )
                .expect("Committed value for ID must always exist")
                .try_into_move_value(layout)?),
            ViewState::Unsync(_state) => {
                // TODO(aggregator): Support sequential execution.
                unimplemented!("ID to value replacement for sequential execution is not supported")
            },
        }
    }
}
