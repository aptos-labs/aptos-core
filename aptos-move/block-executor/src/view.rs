// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
use crate::types::InputOutputKey;
#[cfg(feature = "blockstm-profiling")]
use crate::view_profiler::{ViewKind, ViewProfiler, ViewProfilerGuard};
use crate::{
    captured_reads::{
        CapturedReads, DataRead, DelayedFieldRead, DelayedFieldReadKind, GroupRead, ReadKind,
        UnsyncReadSet,
    },
    code_cache_global::GlobalModuleCache,
    counters,
    scheduler::{DependencyResult, DependencyStatus, TWaitForDependency},
    scheduler_wrapper::SchedulerWrapper,
    value_exchange::TemporaryValueToIdentifierMapping,
};
use aptos_aggregator::{
    bounded_math::{ok_overflow, BoundedMath, SignedU128},
    delta_change_set::serialize,
    delta_math::DeltaHistory,
    resolver::{TAggregatorV1View, TDelayedFieldView},
    types::{DelayedFieldValue, DelayedFieldsSpeculativeError, ReadPosition},
};
use aptos_logger::error;
use aptos_mvhashmap::{
    types::{
        GroupReadResult, Incarnation, MVDataError, MVDataOutput, MVDelayedFieldsError,
        MVGroupError, StorageVersion, TxnIndex, UnknownOrLayout, UnsyncGroupError, ValueWithLayout,
    },
    unsync_map::UnsyncMap,
    versioned_delayed_fields::TVersionedDelayedFieldView,
    MVHashMap,
};
use aptos_types::{
    error::{code_invariant_error, expect_ok, PanicError, PanicOr},
    executable::ModulePath,
    state_store::{
        errors::StateViewError,
        state_storage_usage::StateStorageUsage,
        state_value::{StateValue, StateValueMetadata},
        StateViewId, TStateView,
    },
    transaction::BlockExecutableTransaction as Transaction,
    vm::modules::AptosModuleExtension,
    write_set::TransactionWrite,
};
use aptos_vm_logging::{log_schema::AdapterLogSchema, prelude::*};
use aptos_vm_types::resolver::{
    BlockSynchronizationKillSwitch, ResourceGroupSize, StateStorageView, TResourceGroupView,
    TResourceView,
};
use bytes::Bytes;
use claims::assert_ok;
use move_binary_format::{
    errors::{PartialVMError, PartialVMResult},
    CompiledModule,
};
use move_core_types::{language_storage::ModuleId, value::MoveTypeLayout, vm_status::StatusCode};
use move_vm_runtime::{AsFunctionValueExtension, Module, RuntimeEnvironment};
use move_vm_types::{
    delayed_values::delayed_field_id::{DelayedFieldID, ExtractUniqueIndex},
    value_serde::ValueSerDeContext,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
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
    Metadata(Option<StateValueMetadata>),
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
                // TODO[agg_v1](cleanup): Move AggV1 to Delayed fields, and then handle the layout if needed
                ReadResult::Value(Some(StateValue::new_legacy(serialize(&v).into())), None)
            },
            DataRead::Metadata(maybe_metadata) => ReadResult::Metadata(maybe_metadata),
            DataRead::Exists(exists) => ReadResult::Exists(exists),
        }
    }

    fn from_value_with_layout<V: TransactionWrite>(
        value: ValueWithLayout<V>,
        kind: ReadKind,
    ) -> Option<Self> {
        match (value, kind) {
            (ValueWithLayout::Exchanged(v, layout), ReadKind::Value) => {
                Some(ReadResult::Value(v.as_state_value(), layout))
            },
            (ValueWithLayout::RawFromStorage(_), ReadKind::Value) => None,
            (ValueWithLayout::Exchanged(v, _), ReadKind::Metadata)
            | (ValueWithLayout::RawFromStorage(v), ReadKind::Metadata) => {
                Some(ReadResult::Metadata(v.as_state_value_metadata()))
            },
            (ValueWithLayout::Exchanged(v, _), ReadKind::Exists)
            | (ValueWithLayout::RawFromStorage(v), ReadKind::Exists) => {
                Some(ReadResult::Exists(!v.is_deletion()))
            },
        }
    }

    pub fn into_value(self) -> Option<StateValue> {
        if let ReadResult::Value(v, _layout) = self {
            v
        } else {
            unreachable!("Read result must be Value kind")
        }
    }
}

trait ResourceState<T: Transaction> {
    fn set_base_value(&self, key: T::Key, value: ValueWithLayout<T::Value>);

    fn read_cached_data_by_kind(
        &self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        key: &T::Key,
        target_kind: ReadKind,
        layout: UnknownOrLayout,
        patch_base_value: &dyn Fn(&T::Value, Option<&MoveTypeLayout>) -> PartialVMResult<T::Value>,
    ) -> ReadResult;
}

trait ResourceGroupState<T: Transaction> {
    fn set_raw_group_base_values(
        &self,
        group_key: T::Key,
        base_values: Vec<(T::Tag, T::Value)>,
    ) -> PartialVMResult<()>;

    fn read_cached_group_tagged_data(
        &self,
        txn_idx: TxnIndex,
        group_key: &T::Key,
        resource_tag: &T::Tag,
        maybe_layout: Option<&MoveTypeLayout>,
        patch_base_value: &dyn Fn(&T::Value, Option<&MoveTypeLayout>) -> PartialVMResult<T::Value>,
    ) -> PartialVMResult<GroupReadResult>;
}

pub(crate) struct ParallelState<'a, T: Transaction> {
    pub(crate) versioned_map: &'a MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
    scheduler: SchedulerWrapper<'a>,
    start_counter: u32,
    counter: &'a AtomicU32,
    incarnation: Incarnation,
    pub(crate) captured_reads:
        RefCell<CapturedReads<T, ModuleId, CompiledModule, Module, AptosModuleExtension>>,
}

fn get_delayed_field_value_impl<T: Transaction>(
    captured_reads: &RefCell<
        CapturedReads<T, ModuleId, CompiledModule, Module, AptosModuleExtension>,
    >,
    versioned_delayed_fields: &dyn TVersionedDelayedFieldView<DelayedFieldID>,
    wait_for: &dyn TWaitForDependency,
    id: &DelayedFieldID,
    txn_idx: TxnIndex,
) -> Result<DelayedFieldValue, PanicOr<DelayedFieldsSpeculativeError>> {
    // We expect only DelayedFieldReadKind::Value (which is set from this function),
    // to be a "full materialized/aggregated" read, and so we don't use the value
    // from HistoryBounded reads.
    // If we wanted to make it more dynamic, we could have a type of the read value
    // inside HistoryBounded
    let delayed_read = captured_reads
        .borrow()
        .get_delayed_field_by_kind(id, DelayedFieldReadKind::Value);
    if let Some(data) = delayed_read {
        if let DelayedFieldRead::Value { value, .. } = data {
            return Ok(value);
        } else {
            let err =
                code_invariant_error("Value DelayedField read returned non-value result").into();
            captured_reads
                .borrow_mut()
                .capture_delayed_field_read_error(&err);
            return Err(err);
        }
    }

    loop {
        match versioned_delayed_fields.read(id, txn_idx) {
            Ok(value) => {
                captured_reads.borrow_mut().capture_delayed_field_read(
                    *id,
                    false,
                    DelayedFieldRead::Value {
                        value: value.clone(),
                    },
                )?;
                return Ok(value);
            },
            Err(PanicOr::Or(MVDelayedFieldsError::Dependency(dep_idx))) => {
                if !wait_for_dependency(wait_for, txn_idx, dep_idx)? {
                    // TODO[agg_v2](cleanup): think of correct return type
                    return Err(PanicOr::Or(DelayedFieldsSpeculativeError::InconsistentRead));
                }
            },
            Err(e) => {
                captured_reads
                    .borrow_mut()
                    .capture_delayed_field_read_error(&e);
                // TODO[agg_v2](cleanup): think of correct return type
                return Err(e.map_non_panic(|_| DelayedFieldsSpeculativeError::InconsistentRead));
            },
        }
    }
}

fn compute_delayed_field_try_add_delta_outcome_from_history(
    base_delta: &SignedU128,
    delta: &SignedU128,
    max_value: u128,
    mut history: DeltaHistory,
    base_aggregator_value: u128,
) -> Result<(bool, DelayedFieldRead), PanicOr<DelayedFieldsSpeculativeError>> {
    let math = BoundedMath::new(max_value);

    let before_value = expect_ok(math.unsigned_add_delta(base_aggregator_value, base_delta))?;

    let result = if math.unsigned_add_delta(before_value, delta).is_err() {
        match delta {
            SignedU128::Positive(delta_value) => {
                let overflow_delta = expect_ok(ok_overflow(
                    math.unsigned_add_delta(*delta_value, base_delta),
                ))?;

                // We don't need to record the value if it overflowed.
                if let Some(overflow_delta) = overflow_delta {
                    history.record_overflow(overflow_delta);
                }
            },
            SignedU128::Negative(delta_value) => {
                let underflow_delta = expect_ok(ok_overflow(
                    math.unsigned_add_delta(*delta_value, &base_delta.minus()),
                ))?;
                // We don't need to record the value if it overflowed (delta was smaller than -max_value).
                if let Some(underflow_delta) = underflow_delta {
                    history.record_underflow(underflow_delta);
                }
            },
        };

        false
    } else {
        let new_delta = expect_ok(math.signed_add(base_delta, delta))?;
        history.record_success(new_delta);
        true
    };

    Ok((result, DelayedFieldRead::HistoryBounded {
        restriction: history,
        max_value,
        inner_aggregator_value: base_aggregator_value,
    }))
}

fn compute_delayed_field_try_add_delta_outcome_first_time(
    delta: &SignedU128,
    max_value: u128,
    base_aggregator_value: u128,
) -> Result<(bool, DelayedFieldRead), PanicOr<DelayedFieldsSpeculativeError>> {
    let math = BoundedMath::new(max_value);
    let mut history = DeltaHistory::new();
    let result = if math
        .unsigned_add_delta(base_aggregator_value, delta)
        .is_err()
    {
        match delta {
            SignedU128::Positive(delta_value) => {
                history.record_overflow(*delta_value);
            },
            SignedU128::Negative(delta_value) => {
                history.record_underflow(*delta_value);
            },
        };
        false
    } else {
        history.record_success(*delta);
        true
    };

    Ok((result, DelayedFieldRead::HistoryBounded {
        restriction: history,
        max_value,
        inner_aggregator_value: base_aggregator_value,
    }))
}
// TODO[agg_v2](cleanup): see about the split with CapturedReads,
// and whether anything should be moved there.
fn delayed_field_try_add_delta_outcome_impl<T: Transaction>(
    captured_reads: &RefCell<
        CapturedReads<T, ModuleId, CompiledModule, Module, AptosModuleExtension>,
    >,
    versioned_delayed_fields: &dyn TVersionedDelayedFieldView<DelayedFieldID>,
    wait_for: &dyn TWaitForDependency,
    id: &DelayedFieldID,
    base_delta: &SignedU128,
    delta: &SignedU128,
    max_value: u128,
    txn_idx: TxnIndex,
) -> Result<bool, PanicOr<DelayedFieldsSpeculativeError>> {
    // No need to record or check or try, if input value exceeds the bound.
    if delta.abs() > max_value {
        return Ok(false);
    }

    let delayed_read = captured_reads
        .borrow()
        .get_delayed_field_by_kind(id, DelayedFieldReadKind::HistoryBounded);
    match delayed_read {
        Some(DelayedFieldRead::Value { value }) => {
            let math = BoundedMath::new(max_value);
            let before = expect_ok(
                math.unsigned_add_delta(value.clone().into_aggregator_value()?, base_delta),
            )?;
            Ok(math.unsigned_add_delta(before, delta).is_ok())
        },
        Some(DelayedFieldRead::HistoryBounded {
            restriction: history,
            max_value: before_max_value,
            inner_aggregator_value,
        }) => {
            if before_max_value != max_value {
                return Err(
                    code_invariant_error("Cannot merge deltas with different limits").into(),
                );
            }

            let (result, udpated_delayed_read) =
                compute_delayed_field_try_add_delta_outcome_from_history(
                    base_delta,
                    delta,
                    max_value,
                    history,
                    inner_aggregator_value,
                )?;

            captured_reads.borrow_mut().capture_delayed_field_read(
                *id,
                true,
                udpated_delayed_read,
            )?;
            Ok(result)
        },
        None => {
            if !base_delta.is_zero() {
                return Err(code_invariant_error(
                    "Passed-in delta is not zero, but CapturedReads has no record",
                )
                .into());
            }

            let predicted_value = loop {
                match versioned_delayed_fields.read_latest_predicted_value(
                    id,
                    txn_idx,
                    ReadPosition::BeforeCurrentTxn,
                ) {
                    Ok(v) => break v,
                    Err(MVDelayedFieldsError::Dependency(dep_idx)) => {
                        if !wait_for_dependency(wait_for, txn_idx, dep_idx)? {
                            // TODO[agg_v2](cleanup): think of correct return type
                            return Err(PanicOr::Or(
                                DelayedFieldsSpeculativeError::InconsistentRead,
                            ));
                        }
                    },
                    Err(_) => {
                        return Err(PanicOr::Or(DelayedFieldsSpeculativeError::InconsistentRead))
                    },
                };
            }
            .into_aggregator_value()?;

            let (result, new_delayed_read) =
                compute_delayed_field_try_add_delta_outcome_first_time(
                    delta,
                    max_value,
                    predicted_value,
                )?;

            captured_reads
                .borrow_mut()
                .capture_delayed_field_read(*id, false, new_delayed_read)?;
            Ok(result)
        },
    }
}

// txn_idx is estimated to have a r/w dependency on dep_idx.
// Returns after the dependency has been resolved, the returned indicator is true if
// it is safe to continue, and false if the execution has been halted.
fn wait_for_dependency(
    wait_for: &dyn TWaitForDependency,
    txn_idx: TxnIndex,
    dep_idx: TxnIndex,
) -> Result<bool, PanicError> {
    match wait_for.wait_for_dependency(txn_idx, dep_idx)? {
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
            while matches!(*dep_resolved, DependencyStatus::Unresolved) {
                dep_resolved = cvar.wait(dep_resolved).unwrap();
            }
            // dep resolved status is either resolved or execution halted.
            Ok(matches!(*dep_resolved, DependencyStatus::Resolved))
        },
        DependencyResult::ExecutionHalted => Ok(false),
        DependencyResult::Resolved => Ok(true),
    }
}

impl<'a, T: Transaction> ParallelState<'a, T> {
    pub(crate) fn new(
        shared_map: &'a MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
        shared_scheduler: SchedulerWrapper<'a>,
        start_shared_counter: u32,
        shared_counter: &'a AtomicU32,
        incarnation: Incarnation,
    ) -> Self {
        let block_stm_v2 = shared_scheduler.is_v2();
        Self {
            versioned_map: shared_map,
            scheduler: shared_scheduler,
            start_counter: start_shared_counter,
            counter: shared_counter,
            incarnation,
            captured_reads: RefCell::new(CapturedReads::new(block_stm_v2)),
        }
    }

    pub(crate) fn set_delayed_field_value(
        &self,
        id: DelayedFieldID,
        base_value: DelayedFieldValue,
    ) {
        self.versioned_map
            .delayed_fields()
            .set_base_value(id, base_value)
    }

    fn read_group_size(
        &self,
        group_key: &T::Key,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
    ) -> PartialVMResult<GroupReadResult> {
        use MVGroupError::*;

        if let Some(group_size) = self.captured_reads.borrow().group_size(group_key) {
            return Ok(GroupReadResult::Size(group_size));
        }

        loop {
            let group_size = if self.scheduler.is_v2() {
                self.versioned_map
                    .group_data()
                    .get_group_size_v2(group_key, txn_idx, incarnation)
            } else {
                self.versioned_map.group_data().get_group_size(group_key, txn_idx)
            };

            match group_size
            {
                Ok(group_size) => {
                    assert_ok!(
                        self.captured_reads
                            .borrow_mut()
                            .capture_group_size(group_key.clone(), group_size),
                        "Group size may not be inconsistent: must be recorded once"
                    );

                    return Ok(GroupReadResult::Size(group_size));
                },
                Err(Uninitialized) => {
                    return Ok(GroupReadResult::Uninitialized);
                },
                Err(TagNotFound) => {
                    unreachable!("Reading group size does not require a specific tag look-up");
                },
                Err(Dependency(dep_idx)) => {
                    if !wait_for_dependency(&self.scheduler, txn_idx, dep_idx)? {
                        return Err(PartialVMError::new(
                            StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR,
                        )
                        .with_message("Interrupted as block execution was halted".to_string()));
                    }
                },
            }
        }
    }
}

impl<'a, T: Transaction> ResourceState<T> for ParallelState<'a, T> {
    fn set_base_value(&self, key: T::Key, value: ValueWithLayout<T::Value>) {
        self.versioned_map.data().set_base_value(key, value);
    }

    /// Captures a read from the VM execution, but not unresolved deltas, as in this case it is the
    /// callers responsibility to set the aggregator's base value and call fetch_data again.
    fn read_cached_data_by_kind(
        &self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        key: &T::Key,
        target_kind: ReadKind,
        layout: UnknownOrLayout,
        patch_base_value: &dyn Fn(&T::Value, Option<&MoveTypeLayout>) -> PartialVMResult<T::Value>,
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
            let data = if self.scheduler.is_v2() {
                self.versioned_map
                    .data()
                    .fetch_data_v2(key, txn_idx, incarnation)
            } else {
                self.versioned_map.data().fetch_data(key, txn_idx)
            };

            match data {
                Ok(Versioned(version, value)) => {
                    // If we have a known layout, upgrade RawFromStorage value to Exchanged.
                    if let UnknownOrLayout::Known(layout) = layout {
                        if let ValueWithLayout::RawFromStorage(v) = value {
                            assert_eq!(version, Err(StorageVersion), "Fetched resource has unknown layout but the version is not Err(StorageVersion)");
                            match patch_base_value(v.as_ref(), layout) {
                                Ok(patched_value) => {
                                    self.versioned_map.data().set_base_value(
                                        key.clone(),
                                        ValueWithLayout::Exchanged(
                                            Arc::new(patched_value),
                                            layout.cloned().map(Arc::new),
                                        ),
                                    );
                                    // Refetch in case a concurrent change went through.
                                    continue;
                                },
                                Err(e) => {
                                    error!("Couldn't patch value from versioned map: {}", e);
                                    self.captured_reads.borrow_mut().mark_incorrect_use();
                                    return ReadResult::HaltSpeculativeExecution(
                                        "Couldn't patch value from versioned map".to_string(),
                                    );
                                },
                            }
                        }
                    }

                    let data_read = match DataRead::from_value_with_layout(version, value)
                        .downcast(target_kind)
                    {
                        Some(data_read) => data_read,
                        None => {
                            error!("Couldn't downcast value from versioned map");
                            self.captured_reads.borrow_mut().mark_incorrect_use();
                            return ReadResult::HaltSpeculativeExecution(
                                "Couldn't downcast value from versioned map".to_string(),
                            );
                        },
                    };

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
                    match wait_for_dependency(&self.scheduler, txn_idx, dep_idx) {
                        Err(e) => {
                            error!("Error {:?} in wait for dependency", e);
                            self.captured_reads.borrow_mut().mark_incorrect_use();
                            return ReadResult::HaltSpeculativeExecution(format!(
                                "Error {:?} in wait for dependency",
                                e
                            ));
                        },
                        Ok(false) => {
                            self.captured_reads.borrow_mut().mark_failure(false);
                            return ReadResult::HaltSpeculativeExecution(
                                "Interrupted as block execution was halted".to_string(),
                            );
                        },
                        Ok(true) => {
                            //dependency resolved
                        },
                    }
                },
                Err(DeltaApplicationFailure) => {
                    // AggregatorV1 may have delta application failure due to speculation.
                    self.captured_reads.borrow_mut().mark_failure(false);
                    return ReadResult::HaltSpeculativeExecution(
                        "Delta application failure (must be speculative)".to_string(),
                    );
                },
            };
        }
    }
}

impl<'a, T: Transaction> ResourceGroupState<T> for ParallelState<'a, T> {
    fn set_raw_group_base_values(
        &self,
        group_key: T::Key,
        base_values: Vec<(T::Tag, T::Value)>,
    ) -> PartialVMResult<()> {
        self.versioned_map
            .group_data()
            .set_raw_base_values(group_key.clone(), base_values)
            .map_err(|e| {
                self.captured_reads.borrow_mut().mark_incorrect_use();
                PartialVMError::new(StatusCode::UNEXPECTED_DESERIALIZATION_ERROR)
                    .with_message(e.to_string())
            })
    }

    fn read_cached_group_tagged_data(
        &self,
        txn_idx: TxnIndex,
        group_key: &T::Key,
        resource_tag: &T::Tag,
        maybe_layout: Option<&MoveTypeLayout>,
        patch_base_value: &dyn Fn(&T::Value, Option<&MoveTypeLayout>) -> PartialVMResult<T::Value>,
    ) -> PartialVMResult<GroupReadResult> {
        use MVGroupError::*;

        if let Some(DataRead::Versioned(_, v, layout)) =
            self.captured_reads
                .borrow()
                .get_by_kind(group_key, Some(resource_tag), ReadKind::Value)
        {
            return Ok(GroupReadResult::Value(v.extract_raw_bytes(), layout));
        }

        loop {
            let data = if self.scheduler.is_v2() {
                self.versioned_map
                    .group_data()
                    .fetch_tagged_data_v2(group_key, resource_tag, txn_idx, self.incarnation)
            } else {
                self.versioned_map.group_data().fetch_tagged_data(group_key, resource_tag, txn_idx)
            };

            match data {
                Ok((version, value_with_layout)) => {
                    // If we have a known layout, upgrade RawFromStorage value to Exchanged.
                    match value_with_layout {
                        ValueWithLayout::RawFromStorage(v) => {
                            let patched_value = patch_base_value(v.as_ref(), maybe_layout)?;
                            self.versioned_map
                                .group_data()
                                .update_tagged_base_value_with_layout(
                                    group_key.clone(),
                                    resource_tag.clone(),
                                    patched_value,
                                    maybe_layout.cloned().map(Arc::new),
                                );
                            // Re-fetch in case a concurrent change went through.
                            continue;
                        },
                        ValueWithLayout::Exchanged(value, layout) => {
                            let data_read =
                                DataRead::Versioned(version, value.clone(), layout.clone());
                            assert_ok!(
                                self.captured_reads.borrow_mut().capture_read(
                                    group_key.clone(),
                                    Some(resource_tag.clone()),
                                    data_read
                                ),
                                "Resource read in group recorded once: may not be inconsistent"
                            );
                            return Ok(GroupReadResult::Value(
                                value.extract_raw_bytes(),
                                layout.clone(),
                            ));
                        },
                    }
                },
                Err(Uninitialized) => {
                    return Ok(GroupReadResult::Uninitialized);
                },
                Err(TagNotFound) => {
                    self.versioned_map.group_data().update_tagged_base_value_with_layout(
                        group_key.clone(),
                        resource_tag.clone(),
                        TransactionWrite::from_state_value(None),
                        None,
                    );
                    continue;
                    // let data_read = DataRead::Versioned(
                    //     Err(StorageVersion),
                    //     Arc::<T::Value>::new(TransactionWrite::from_state_value(None)),
                    //     None,
                    // );
                    // assert_ok!(
                    //     self.captured_reads.borrow_mut().capture_read(
                    //         group_key.clone(),
                    //         Some(resource_tag.clone()),
                    //         data_read
                    //     ),
                    //     "Resource read in group recorded once: may not be inconsistent"
                    // );

                    // return Ok(GroupReadResult::Value(None, None));
                },
                Err(Dependency(dep_idx)) => {
                    if !wait_for_dependency(&self.scheduler, txn_idx, dep_idx)? {
                        // TODO[agg_v2](cleanup): consider changing from PartialVMResult<GroupReadResult> to GroupReadResult
                        // like in ReadResult for resources.
                        return Err(PartialVMError::new(
                            StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR,
                        )
                        .with_message("Interrupted as block execution was halted".to_string()));
                    }
                },
            }
        }
    }
}

pub(crate) struct SequentialState<'a, T: Transaction> {
    pub(crate) unsync_map: &'a UnsyncMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
    pub(crate) read_set: RefCell<UnsyncReadSet<T, ModuleId>>,
    pub(crate) start_counter: u32,
    pub(crate) counter: &'a RefCell<u32>,
    // TODO: Move to UnsyncMap.
    pub(crate) incorrect_use: RefCell<bool>,
}

impl<'a, T: Transaction> SequentialState<'a, T> {
    pub fn new(
        unsync_map: &'a UnsyncMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
        start_counter: u32,
        counter: &'a RefCell<u32>,
    ) -> Self {
        Self {
            unsync_map,
            read_set: RefCell::new(UnsyncReadSet::default()),
            start_counter,
            counter,
            incorrect_use: RefCell::new(false),
        }
    }

    pub(crate) fn set_delayed_field_value(
        &self,
        id: DelayedFieldID,
        base_value: DelayedFieldValue,
    ) {
        self.unsync_map.set_base_delayed_field(id, base_value)
    }

    pub(crate) fn read_delayed_field(&self, id: DelayedFieldID) -> Option<DelayedFieldValue> {
        self.unsync_map.fetch_delayed_field(&id)
    }
}

impl<'a, T: Transaction> ResourceState<T> for SequentialState<'a, T> {
    fn set_base_value(&self, key: T::Key, value: ValueWithLayout<T::Value>) {
        self.unsync_map.set_base_value(key, value);
    }

    fn read_cached_data_by_kind(
        &self,
        _txn_idx: TxnIndex,
        _incarnation: Incarnation,
        key: &T::Key,
        target_kind: ReadKind,
        layout: UnknownOrLayout,
        patch_base_value: &dyn Fn(&T::Value, Option<&MoveTypeLayout>) -> PartialVMResult<T::Value>,
    ) -> ReadResult {
        match self.unsync_map.fetch_data(key) {
            Some(mut value) => {
                // If we have a known layout, upgrade RawFromStorage value to Exchanged.
                if let UnknownOrLayout::Known(layout) = layout {
                    if let ValueWithLayout::RawFromStorage(v) = value {
                        match patch_base_value(v.as_ref(), layout) {
                            Ok(patched_value) => {
                                let exchanged_value = ValueWithLayout::Exchanged(
                                    Arc::new(patched_value.clone()),
                                    layout.cloned().map(Arc::new),
                                );
                                self.unsync_map
                                    .set_base_value(key.clone(), exchanged_value.clone());

                                // sequential execution doesn't need to worry about concurrent change going through.
                                value = exchanged_value;
                            },
                            Err(_) => {
                                // TODO[agg_v2](cleanup): `patch_base_value` already marks as incorrect use
                                //               and logs an error! We need to make this uniform across
                                //               resources and groups.
                                *self.incorrect_use.borrow_mut() = true;
                                error!("Unsync map couldn't patch base value");
                                return ReadResult::HaltSpeculativeExecution(
                                    "Unsync map couldn't patch base value".to_string(),
                                );
                            },
                        }
                    }
                }

                if let Some(ret) = ReadResult::from_value_with_layout(value, target_kind.clone()) {
                    if target_kind == ReadKind::Value {
                        self.read_set
                            .borrow_mut()
                            .resource_reads
                            .insert(key.clone());
                    }

                    ret
                } else {
                    *self.incorrect_use.borrow_mut() = true;
                    error!(
                        "Unsync map has RawFromStorage value type, while we are requesting value"
                    );
                    ReadResult::HaltSpeculativeExecution(
                        "Unsync map has RawFromStorage value type, while we are requesting value"
                            .to_string(),
                    )
                }
            },
            None => ReadResult::Uninitialized,
        }
    }
}

impl<'a, T: Transaction> ResourceGroupState<T> for SequentialState<'a, T> {
    fn set_raw_group_base_values(
        &self,
        group_key: T::Key,
        base_values: Vec<(T::Tag, T::Value)>,
    ) -> PartialVMResult<()> {
        self.unsync_map
            .set_group_base_values(group_key.clone(), base_values)
            .map_err(|e| {
                *self.incorrect_use.borrow_mut() = true;
                PartialVMError::new(StatusCode::UNEXPECTED_DESERIALIZATION_ERROR)
                    .with_message(e.to_string())
            })
    }

    fn read_cached_group_tagged_data(
        &self,
        _txn_idx: TxnIndex,
        group_key: &T::Key,
        resource_tag: &T::Tag,
        maybe_layout: Option<&MoveTypeLayout>,
        patch_base_value: &dyn Fn(&T::Value, Option<&MoveTypeLayout>) -> PartialVMResult<T::Value>,
    ) -> PartialVMResult<GroupReadResult> {
        match self
            .unsync_map
            .fetch_group_tagged_data(group_key, resource_tag)
        {
            Ok(mut value) => {
                // If we have a known layout, upgrade RawFromStorage value to Exchanged.
                if let ValueWithLayout::RawFromStorage(v) = value {
                    let patched_value = patch_base_value(v.as_ref(), maybe_layout)?;
                    let maybe_layout = maybe_layout.cloned().map(Arc::new);
                    self.unsync_map.update_tagged_base_value_with_layout(
                        group_key.clone(),
                        resource_tag.clone(),
                        patched_value.clone(),
                        maybe_layout.clone(),
                    );

                    // Sequential execution doesn't need to worry about concurrent change going through.
                    value = ValueWithLayout::Exchanged(Arc::new(patched_value), maybe_layout);
                }

                if let ValueWithLayout::Exchanged(v, l) = value {
                    let bytes = v.extract_raw_bytes();
                    self.read_set
                        .borrow_mut()
                        .group_reads
                        .entry(group_key.clone())
                        .or_default()
                        .insert(resource_tag.clone());
                    Ok(GroupReadResult::Value(bytes, l.clone()))
                } else {
                    *self.incorrect_use.borrow_mut() = true;
                    error!(
                        "Unsync map has RawFromStorage value type, while we are requesting value"
                    );
                    Ok(GroupReadResult::Uninitialized)
                }
            },
            Err(UnsyncGroupError::Uninitialized) => Ok(GroupReadResult::Uninitialized),
            Err(UnsyncGroupError::TagNotFound) => {
                self.read_set
                    .borrow_mut()
                    .group_reads
                    .entry(group_key.clone())
                    .or_default()
                    .insert(resource_tag.clone());
                Ok(GroupReadResult::Value(None, None))
            },
        }
    }
}

pub(crate) enum ViewState<'a, T: Transaction> {
    Sync(ParallelState<'a, T>),
    Unsync(SequentialState<'a, T>),
}

impl<'a, T: Transaction> ViewState<'a, T> {
    fn get_resource_state(&self) -> &dyn ResourceState<T> {
        match self {
            ViewState::Sync(state) => state,
            ViewState::Unsync(state) => state,
        }
    }

    fn get_resource_group_state(&self) -> &dyn ResourceGroupState<T> {
        match self {
            ViewState::Sync(state) => state,
            ViewState::Unsync(state) => state,
        }
    }

    fn incarnation(&self) -> Incarnation {
        match self {
            ViewState::Sync(state) => state.incarnation,
            ViewState::Unsync(_) => 0,
        }
    }
}

/// A struct that represents a single block execution worker thread's view into the state,
/// some of which (in Sync case) might be shared with other workers / threads. By implementing
/// all necessary traits, LatestView is provided to the VM and used to intercept the reads.
/// In the Sync case, also records captured reads for later validation. latest_txn_idx
/// must be set according to the latest transaction that the worker was / is executing.
pub(crate) struct LatestView<'a, T: Transaction, S: TStateView<Key = T::Key>> {
    base_view: &'a S,
    pub(crate) global_module_cache:
        &'a GlobalModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension>,
    pub(crate) runtime_environment: &'a RuntimeEnvironment,
    pub(crate) latest_view: ViewState<'a, T>,
    pub(crate) txn_idx: TxnIndex,
    #[cfg(feature = "blockstm-profiling")]
    pub(crate) view_profiler: RefCell<ViewProfiler>,
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>> LatestView<'a, T, S> {
    pub(crate) fn new(
        base_view: &'a S,
        global_module_cache: &'a GlobalModuleCache<
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
        >,
        runtime_environment: &'a RuntimeEnvironment,
        latest_view: ViewState<'a, T>,
        txn_idx: TxnIndex,
    ) -> Self {
        Self {
            base_view,
            global_module_cache,
            runtime_environment,
            latest_view,
            txn_idx,
            #[cfg(feature = "blockstm-profiling")]
            view_profiler: RefCell::new(ViewProfiler::new()),
        }
    }

    #[cfg(feature = "blockstm-profiling")]
    pub(crate) fn get_profiler_guard(&self, kind: ViewKind) -> ViewProfilerGuard {
        ViewProfilerGuard::new(&self.view_profiler, kind)
    }

    #[cfg(feature = "blockstm-profiling")]
    pub(crate) fn view_profiler_log_info(&self, incarnation: Incarnation, worker_id: usize) {
        if let ViewState::Sync(state) = &self.latest_view {
            assert_eq!(
                incarnation, state.incarnation,
                "Incarnations provided by executor must match"
            );
        }
        self.view_profiler
            .borrow()
            .log_info(self.txn_idx, incarnation, worker_id);
    }

    #[cfg(test)]
    fn get_read_summary(&self) -> HashSet<InputOutputKey<T::Key, T::Tag>> {
        match &self.latest_view {
            ViewState::Sync(state) => state.captured_reads.borrow().get_read_summary(),
            ViewState::Unsync(state) => state.read_set.borrow().get_read_summary(),
        }
    }

    /// Drains the parallel captured reads.
    pub(crate) fn take_parallel_reads(
        &self,
    ) -> CapturedReads<T, ModuleId, CompiledModule, Module, AptosModuleExtension> {
        match &self.latest_view {
            ViewState::Sync(state) => state.captured_reads.take(),
            ViewState::Unsync(_) => {
                unreachable!("Take reads called in sequential setting (not captured)")
            },
        }
    }

    /// Drains the unsync read set.
    pub(crate) fn take_sequential_reads(&self) -> UnsyncReadSet<T, ModuleId> {
        match &self.latest_view {
            ViewState::Sync(_) => {
                unreachable!("Take unsync reads called in parallel setting")
            },
            ViewState::Unsync(state) => state.read_set.take(),
        }
    }

    fn mark_incorrect_use(&self) {
        match &self.latest_view {
            ViewState::Sync(state) => state.captured_reads.borrow_mut().mark_incorrect_use(),
            ViewState::Unsync(state) => *state.incorrect_use.borrow_mut() = true,
        }
    }

    pub fn is_incorrect_use(&self) -> bool {
        match &self.latest_view {
            ViewState::Sync(_) => {
                // Parallel executor accesses captured reads directly and does not use this API.
                true
            },
            // TODO: store incorrect use in UnsyncMap and eliminate this API.
            ViewState::Unsync(state) => *state.incorrect_use.borrow(),
        }
    }

    pub(crate) fn get_raw_base_value(
        &self,
        state_key: &T::Key,
    ) -> PartialVMResult<Option<StateValue>> {
        let ret = self.base_view.get_state_value(state_key).map_err(|e| {
            PartialVMError::new(StatusCode::STORAGE_ERROR).with_message(format!(
                "Unexpected storage error for {:?}: {:?}",
                state_key, e
            ))
        });

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

    fn patch_base_value(
        &self,
        value: &T::Value,
        layout: Option<&MoveTypeLayout>,
    ) -> PartialVMResult<T::Value> {
        let maybe_patched = match (value.as_state_value(), layout) {
            (Some(state_value), Some(layout)) => {
                let res = self.replace_values_with_identifiers(state_value, layout);
                match res {
                    Ok((value, _)) => Some(value),
                    Err(err) => {
                        let log_context =
                            AdapterLogSchema::new(self.base_view.id(), self.txn_idx as usize);
                        alert!(
                            log_context,
                            "[VM, ResourceView] Error during value to id replacement: {}",
                            err
                        );
                        self.mark_incorrect_use();
                        return Err(PartialVMError::new(
                            StatusCode::DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR,
                        )
                        .with_message(format!("{}", err)));
                    },
                }
            },
            (state_value, _) => state_value,
        };
        Ok(TransactionWrite::from_state_value(maybe_patched))
    }

    /// Given a state value, performs deserialization-serialization round-trip
    /// to replace any aggregator / snapshot values.
    fn replace_values_with_identifiers(
        &self,
        state_value: StateValue,
        layout: &MoveTypeLayout,
    ) -> anyhow::Result<(StateValue, HashSet<DelayedFieldID>)> {
        let mapping = TemporaryValueToIdentifierMapping::new(self, self.txn_idx);
        let function_value_extension = self.as_function_value_extension();

        state_value
            .map_bytes(|bytes| {
                // This call will replace all occurrences of aggregator / snapshot
                // values with unique identifiers with the same type layout.
                // The values are stored in aggregators multi-version data structure,
                // see the actual trait implementation for more details.
                let patched_value = ValueSerDeContext::new()
                    .with_delayed_fields_replacement(&mapping)
                    .with_func_args_deserialization(&function_value_extension)
                    .deserialize(bytes.as_ref(), layout)
                    .ok_or_else(|| {
                        anyhow::anyhow!("Failed to deserialize resource during id replacement")
                    })?;

                ValueSerDeContext::new()
                    .with_delayed_fields_serde()
                    .with_func_args_deserialization(&function_value_extension)
                    .serialize(&patched_value, layout)?
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
    ) -> anyhow::Result<(Bytes, HashSet<DelayedFieldID>)> {
        // This call will replace all occurrences of aggregator / snapshot
        // identifiers with values with the same type layout.
        let function_value_extension = self.as_function_value_extension();
        let value = ValueSerDeContext::new()
            .with_func_args_deserialization(&function_value_extension)
            .with_delayed_fields_serde()
            .deserialize(bytes, layout)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Failed to deserialize resource during id replacement: {:?}",
                    bytes
                )
            })?;

        let mapping = TemporaryValueToIdentifierMapping::new(self, self.txn_idx);
        let patched_bytes = ValueSerDeContext::new()
            .with_delayed_fields_replacement(&mapping)
            .with_func_args_deserialization(&function_value_extension)
            .serialize(&value, layout)?
            .ok_or_else(|| anyhow::anyhow!("Failed to serialize resource during id replacement"))?
            .into();
        Ok((patched_bytes, mapping.into_inner()))
    }

    fn get_reads_needing_exchange_sequential(
        &self,
        read_set: &HashSet<T::Key>,
        unsync_map: &UnsyncMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
        delayed_write_set_ids: &HashSet<DelayedFieldID>,
        skip: &HashSet<T::Key>,
    ) -> Result<BTreeMap<T::Key, (StateValueMetadata, u64, Arc<MoveTypeLayout>)>, PanicError> {
        read_set
            .iter()
            .filter_map(|key| {
                if skip.contains(key) {
                    return None;
                }

                match unsync_map.fetch_data(key) {
                    Some(ValueWithLayout::Exchanged(value, Some(layout))) => self
                        .filter_value_for_exchange(
                            value.as_ref(),
                            &layout,
                            delayed_write_set_ids,
                            key,
                        ),
                    Some(ValueWithLayout::Exchanged(_, None)) => None,
                    Some(ValueWithLayout::RawFromStorage(_)) => Some(Err(code_invariant_error(
                        "Cannot exchange value that was not exchanged before",
                    ))),
                    None => None,
                }
            })
            .collect()
    }

    fn get_group_reads_needing_exchange_parallel(
        &self,
        parallel_state: &ParallelState<'a, T>,
        delayed_write_set_ids: &HashSet<DelayedFieldID>,
        skip: &HashSet<T::Key>,
    ) -> PartialVMResult<BTreeMap<T::Key, (StateValueMetadata, u64)>> {
        let reads_with_delayed_fields = parallel_state
            .captured_reads
            .borrow()
            .get_group_read_values_with_delayed_fields(skip)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>();

        reads_with_delayed_fields
            .into_iter()
            .map(|(key, group_read)| -> PartialVMResult<_> {
                let GroupRead { inner_reads, .. } = group_read;

                // TODO[agg_v2](clean-up): Once ids can be extracted without possible failure,
                // the following is just an any call on iterator (same for resource reads).
                let mut resources_needing_delayed_field_exchange = false;
                for data_read in inner_reads.values() {
                    if let DataRead::Versioned(_version, value, Some(layout)) = data_read {
                        let needs_exchange = self
                            .does_value_need_exchange(value, layout.as_ref(), delayed_write_set_ids)
                            .map_err(PartialVMError::from)?;

                        if needs_exchange {
                            resources_needing_delayed_field_exchange = true;
                            break;
                        }
                    }
                }
                if !resources_needing_delayed_field_exchange {
                    return Ok(None);
                }

                match self.get_resource_state_value_metadata(&key)? {
                    Some(metadata) => match parallel_state.read_group_size(&key, self.txn_idx, parallel_state.incarnation)? {
                        GroupReadResult::Size(group_size) => {
                            Ok(Some((key, (metadata, group_size.get()))))
                        },
                        GroupReadResult::Value(_, _) | GroupReadResult::Uninitialized => {
                            Err(code_invariant_error(format!(
                                "Cannot compute metadata op size for the group read {:?}",
                                key
                            ))
                            .into())
                        },
                    },
                    None => Err(code_invariant_error(format!(
                        "Metadata op not present for the group read {:?}",
                        key
                    ))
                    .into()),
                }
            })
            .flat_map(Result::transpose)
            .collect()
    }

    fn get_group_reads_needing_exchange_sequential(
        &self,
        group_read_set: &HashMap<T::Key, HashSet<T::Tag>>,
        unsync_map: &UnsyncMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
        delayed_write_set_ids: &HashSet<DelayedFieldID>,
        skip: &HashSet<T::Key>,
    ) -> PartialVMResult<BTreeMap<T::Key, (StateValueMetadata, u64)>> {
        group_read_set
            .iter()
            .filter(|(key, _tags)| !skip.contains(key))
            .map(|(key, tags)| -> PartialVMResult<_> {
                if let Some(value_vec) = unsync_map.fetch_group_data(key) {
                    // TODO[agg_v2](cleanup) - can we use .any() instead?
                    let mut resources_needing_delayed_field_exchange = false;
                    for (tag, value_with_layout) in value_vec {
                        if tags.contains(&tag) {
                            if let ValueWithLayout::Exchanged(value, Some(layout)) =
                                value_with_layout
                            {
                                let needs_exchange = self.does_value_need_exchange(
                                    &value,
                                    layout.as_ref(),
                                    delayed_write_set_ids,
                                )?;
                                if needs_exchange {
                                    resources_needing_delayed_field_exchange = true;
                                    break;
                                }
                            }
                        }
                    }
                    if !resources_needing_delayed_field_exchange {
                        return Ok(None);
                    }
                    match self.get_resource_state_value_metadata(key)? {
                        Some(metadata) => match unsync_map.get_group_size(key) {
                            GroupReadResult::Size(group_size) => {
                                Ok(Some((key.clone(), (metadata, group_size.get()))))
                            },
                            GroupReadResult::Value(_, _) => {
                                unreachable!(
                                    "get_group_size cannot return GroupReadResult::Value type"
                                )
                            },
                            GroupReadResult::Uninitialized => Err(code_invariant_error(format!(
                                "Sequential cannot find metadata op size for the group read {:?}",
                                key
                            ))
                            .into()),
                        },
                        None => Err(code_invariant_error(format!(
                            "Sequential cannot find metadata op for the group read {:?}",
                            key,
                        ))
                        .into()),
                    }
                } else {
                    Ok(None)
                }
            })
            .flat_map(Result::transpose)
            .collect()
    }

    fn get_resource_state_value_impl(
        &self,
        state_key: &T::Key,
        layout: UnknownOrLayout,
        kind: ReadKind,
    ) -> PartialVMResult<ReadResult> {
        debug_assert!(
            !state_key.is_module_path(),
            "Reading a module {:?} using ResourceView",
            state_key,
        );

        let state = self.latest_view.get_resource_state();

        let mut ret = state.read_cached_data_by_kind(
            self.txn_idx,
            self.latest_view.incarnation(),
            state_key,
            kind.clone(),
            layout.clone(),
            &|value, layout| self.patch_base_value(value, layout),
        );
        if matches!(ret, ReadResult::Uninitialized) {
            let from_storage =
                TransactionWrite::from_state_value(self.get_raw_base_value(state_key)?);
            state.set_base_value(
                state_key.clone(),
                ValueWithLayout::RawFromStorage(Arc::new(from_storage)),
            );

            // In case of concurrent storage fetches, we cannot use our value,
            // but need to fetch it from versioned_map again.
            ret = state.read_cached_data_by_kind(
                self.txn_idx,
                self.latest_view.incarnation(),
                state_key,
                kind,
                layout.clone(),
                &|value, layout| self.patch_base_value(value, layout),
            );
        }

        match ret {
            // ExecutionHalted indicates that the parallel execution is halted.
            // The read should return immediately and log the error.
            // For now we use SPECULATIVE_EXECUTION_ABORT_ERROR as the VM
            // will not log the speculative error,
            // so no actual error will be logged once the execution is halted and
            // the speculative logging is flushed.
            ReadResult::HaltSpeculativeExecution(msg) => Err(PartialVMError::new(
                StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR,
            )
            .with_message(msg)),
            ReadResult::Uninitialized => Err(code_invariant_error(
                "base value must already be recorded in the MV data structure",
            )
            .into()),
            ReadResult::Exists(_) | ReadResult::Metadata(_) | ReadResult::Value(_, _) => Ok(ret),
        }
    }

    fn initialize_mvhashmap_base_group_contents(&self, group_key: &T::Key) -> PartialVMResult<()> {
        let (base_group, metadata_op): (BTreeMap<T::Tag, Bytes>, _) =
            match self.get_raw_base_value(group_key)? {
                Some(state_value) => (
                    bcs::from_bytes(state_value.bytes()).map_err(|e| {
                        PartialVMError::new(StatusCode::UNEXPECTED_DESERIALIZATION_ERROR)
                            .with_message(format!(
                                "Failed to deserialize the resource group at {:?}: {:?}",
                                group_key, e
                            ))
                    })?,
                    TransactionWrite::from_state_value(Some(state_value)),
                ),
                None => (BTreeMap::new(), TransactionWrite::from_state_value(None)),
            };
        let base_group_sentinel_ops = base_group
            .into_iter()
            .map(|(t, bytes)| {
                (
                    t,
                    TransactionWrite::from_state_value(Some(StateValue::new_legacy(bytes))),
                )
            })
            .collect();

        self.latest_view
            .get_resource_group_state()
            .set_raw_group_base_values(group_key.clone(), base_group_sentinel_ops)?;
        self.latest_view.get_resource_state().set_base_value(
            group_key.clone(),
            ValueWithLayout::RawFromStorage(Arc::new(metadata_op)),
        );
        Ok(())
    }
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>> BlockSynchronizationKillSwitch
    for LatestView<'a, T, S>
{
    fn interrupt_requested(&self) -> bool {
        match &self.latest_view {
            ViewState::Sync(state) => state
                .scheduler
                .interrupt_requested(self.txn_idx, state.incarnation),
            ViewState::Unsync(_) => false,
        }
    }
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>> TResourceView for LatestView<'a, T, S> {
    type Key = T::Key;
    type Layout = MoveTypeLayout;

    fn get_resource_state_value(
        &self,
        state_key: &Self::Key,
        maybe_layout: Option<&Self::Layout>,
    ) -> PartialVMResult<Option<StateValue>> {
        #[cfg(feature = "blockstm-profiling")]
        let _view_profiler = self.get_profiler_guard(ViewKind::ResourceView);

        self.get_resource_state_value_impl(
            state_key,
            UnknownOrLayout::Known(maybe_layout),
            ReadKind::Value,
        )
        .map(|res| res.into_value())
    }

    fn get_resource_state_value_metadata(
        &self,
        state_key: &Self::Key,
    ) -> PartialVMResult<Option<StateValueMetadata>> {
        #[cfg(feature = "blockstm-profiling")]
        let _view_profiler = self.get_profiler_guard(ViewKind::ResourceView);

        self.get_resource_state_value_impl(state_key, UnknownOrLayout::Unknown, ReadKind::Metadata)
            .map(|res| {
                if let ReadResult::Metadata(v) = res {
                    v
                } else {
                    unreachable!("Read result must be Metadata kind")
                }
            })
    }

    fn resource_exists(&self, state_key: &Self::Key) -> PartialVMResult<bool> {
        #[cfg(feature = "blockstm-profiling")]
        let _view_profiler = self.get_profiler_guard(ViewKind::ResourceView);

        self.get_resource_state_value_impl(state_key, UnknownOrLayout::Unknown, ReadKind::Exists)
            .map(|res| {
                if let ReadResult::Exists(v) = res {
                    v
                } else {
                    unreachable!("Read result must be Exists kind")
                }
            })
    }
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>> TResourceGroupView for LatestView<'a, T, S> {
    type GroupKey = T::Key;
    type Layout = MoveTypeLayout;
    type ResourceTag = T::Tag;

    fn resource_group_size(
        &self,
        group_key: &Self::GroupKey,
    ) -> PartialVMResult<ResourceGroupSize> {
        #[cfg(feature = "blockstm-profiling")]
        let _view_profiler = self.get_profiler_guard(ViewKind::GroupView);

        let mut group_read = match &self.latest_view {
            ViewState::Sync(state) => state.read_group_size(group_key, self.txn_idx, state.incarnation)?,
            ViewState::Unsync(state) => state.unsync_map.get_group_size(group_key),
        };

        if matches!(group_read, GroupReadResult::Uninitialized) {
            self.initialize_mvhashmap_base_group_contents(group_key)?;

            group_read = match &self.latest_view {
                ViewState::Sync(state) => state.read_group_size(group_key, self.txn_idx, state.incarnation)?,
                ViewState::Unsync(state) => state.unsync_map.get_group_size(group_key),
            }
        };

        Ok(group_read.into_size())
    }

    fn get_resource_from_group(
        &self,
        group_key: &Self::GroupKey,
        resource_tag: &Self::ResourceTag,
        maybe_layout: Option<&Self::Layout>,
    ) -> PartialVMResult<Option<Bytes>> {
        #[cfg(feature = "blockstm-profiling")]
        let _view_profiler = self.get_profiler_guard(ViewKind::GroupView);

        let mut group_read = self
            .latest_view
            .get_resource_group_state()
            .read_cached_group_tagged_data(
                self.txn_idx,
                group_key,
                resource_tag,
                maybe_layout,
                &|value, layout| self.patch_base_value(value, layout),
            )?;

        if matches!(group_read, GroupReadResult::Uninitialized) {
            self.initialize_mvhashmap_base_group_contents(group_key)?;

            group_read = self
                .latest_view
                .get_resource_group_state()
                .read_cached_group_tagged_data(
                    self.txn_idx,
                    group_key,
                    resource_tag,
                    maybe_layout,
                    &|value, layout| self.patch_base_value(value, layout),
                )?;
        };

        Ok(group_read.into_value().0)
    }

    fn release_group_cache(
        &self,
    ) -> Option<HashMap<Self::GroupKey, BTreeMap<Self::ResourceTag, Bytes>>> {
        unimplemented!("Currently resolved by ResourceGroupAdapter");
    }

    fn is_resource_groups_split_in_change_set_capable(&self) -> bool {
        true
    }
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>> StateStorageView for LatestView<'a, T, S> {
    type Key = T::Key;

    fn id(&self) -> StateViewId {
        self.base_view.id()
    }

    fn read_state_value(&self, state_key: &Self::Key) -> Result<(), StateViewError> {
        self.base_view.get_state_value(state_key)?;
        Ok(())
    }

    fn get_usage(&self) -> Result<StateStorageUsage, StateViewError> {
        self.base_view.get_usage()
    }
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>> TAggregatorV1View for LatestView<'a, T, S> {
    type Identifier = T::Key;

    fn get_aggregator_v1_state_value(
        &self,
        state_key: &Self::Identifier,
    ) -> PartialVMResult<Option<StateValue>> {
        #[cfg(feature = "blockstm-profiling")]
        let _view_profiler = self.get_profiler_guard(ViewKind::AggregatorV1View);

        if let ViewState::Sync(parallel_state) = &self.latest_view {
            parallel_state
                .captured_reads
                .borrow_mut()
                .aggregator_v1_reads
                .insert(state_key.clone());
        }

        // TODO: in BlockSTMv2, record the keys here separately in captured reads
        // Then during commit sequential hook, we can also validate AggregatorV1.

        // TODO[agg_v1](cleanup):
        // Integrate aggregators V1. That is, we can lift the u128 value
        // from the state item by passing the right layout here. This can
        // be useful for cross-testing the old and the new flows.
        // self.get_resource_state_value(state_key, Some(&MoveTypeLayout::U128))
        self.get_resource_state_value(state_key, None)
    }
}

impl<'a, T: Transaction, S: TStateView<Key = T::Key>> TDelayedFieldView for LatestView<'a, T, S> {
    type Identifier = DelayedFieldID;
    type ResourceGroupTag = T::Tag;
    type ResourceKey = T::Key;

    fn get_delayed_field_value(
        &self,
        id: &Self::Identifier,
    ) -> Result<DelayedFieldValue, PanicOr<DelayedFieldsSpeculativeError>> {
        #[cfg(feature = "blockstm-profiling")]
        let _view_profiler = self.get_profiler_guard(ViewKind::DelayedFieldView);

        match &self.latest_view {
            ViewState::Sync(state) => get_delayed_field_value_impl(
                &state.captured_reads,
                state.versioned_map.delayed_fields(),
                &state.scheduler,
                id,
                self.txn_idx,
            ),
            ViewState::Unsync(state) => {
                state.read_set.borrow_mut().delayed_field_reads.insert(*id);
                Ok(state.unsync_map.fetch_delayed_field(id).ok_or_else(|| {
                    code_invariant_error(format!("DelayedField {:?} not found in get_delayed_field_value in sequential execution", id))
                })?)
            },
        }
    }

    fn delayed_field_try_add_delta_outcome(
        &self,
        id: &Self::Identifier,
        base_delta: &SignedU128,
        delta: &SignedU128,
        max_value: u128,
    ) -> Result<bool, PanicOr<DelayedFieldsSpeculativeError>> {
        #[cfg(feature = "blockstm-profiling")]
        let _view_profiler = self.get_profiler_guard(ViewKind::DelayedFieldView);

        match &self.latest_view {
            ViewState::Sync(state) => delayed_field_try_add_delta_outcome_impl(
                &state.captured_reads,
                state.versioned_map.delayed_fields(),
                &state.scheduler,
                id,
                base_delta,
                delta,
                max_value,
                self.txn_idx,
            ),
            ViewState::Unsync(state) => {
                // No speculation in sequential execution, just evaluate directly
                let value = state.unsync_map
                    .fetch_delayed_field(id)
                    .ok_or_else(|| {
                        code_invariant_error(format!("DelayedField {:?} not found in delayed_field_try_add_delta_outcome in sequential execution", id))
                    })?
                    .into_aggregator_value()?;
                let math = BoundedMath::new(max_value);
                let before = expect_ok(math.unsigned_add_delta(value, base_delta))?;
                if math.unsigned_add_delta(before, delta).is_err() {
                    Ok(false)
                } else {
                    Ok(true)
                }
            },
        }
    }

    fn generate_delayed_field_id(&self, width: u32) -> Self::Identifier {
        #[cfg(feature = "blockstm-profiling")]
        let _view_profiler = self.get_profiler_guard(ViewKind::DelayedFieldView);

        let index = match &self.latest_view {
            ViewState::Sync(state) => state.counter.fetch_add(1, Ordering::SeqCst),
            ViewState::Unsync(state) => {
                let mut counter = state.counter.borrow_mut();
                let id = *counter;
                *counter += 1;
                id
            },
        };

        (index, width).into()
    }

    fn validate_delayed_field_id(&self, id: &Self::Identifier) -> Result<(), PanicError> {
        #[cfg(feature = "blockstm-profiling")]
        let _view_profiler = self.get_profiler_guard(ViewKind::DelayedFieldView);

        let unique_index = id.extract_unique_index();

        let start_counter = match &self.latest_view {
            ViewState::Sync(state) => state.start_counter,
            ViewState::Unsync(state) => state.start_counter,
        };
        let current_counter = match &self.latest_view {
            ViewState::Sync(state) => state.counter.load(Ordering::SeqCst),
            ViewState::Unsync(state) => *state.counter.borrow(),
        };

        // We read the counter to create an identifier from it, and only after
        // increment. So its value must be < the current value.
        if unique_index < start_counter || unique_index >= current_counter {
            return Err(code_invariant_error(format!(
                "Invalid delayed field id: {:?} with index: {} (started from {} and reached {})",
                id, unique_index, start_counter, current_counter
            )));
        }
        Ok(())
    }

    fn get_reads_needing_exchange(
        &self,
        delayed_write_set_ids: &HashSet<Self::Identifier>,
        skip: &HashSet<Self::ResourceKey>,
    ) -> Result<
        BTreeMap<Self::ResourceKey, (StateValueMetadata, u64, Arc<MoveTypeLayout>)>,
        PanicError,
    > {
        #[cfg(feature = "blockstm-profiling")]
        let _view_profiler = self.get_profiler_guard(ViewKind::DelayedFieldView);

        match &self.latest_view {
            ViewState::Sync(state) => state
                .captured_reads
                .borrow()
                .get_read_values_with_delayed_fields(self, delayed_write_set_ids, skip),
            ViewState::Unsync(state) => {
                let read_set = state.read_set.borrow();
                self.get_reads_needing_exchange_sequential(
                    &read_set.resource_reads,
                    state.unsync_map,
                    delayed_write_set_ids,
                    skip,
                )
            },
        }
    }

    fn get_group_reads_needing_exchange(
        &self,
        delayed_write_set_ids: &HashSet<Self::Identifier>,
        skip: &HashSet<Self::ResourceKey>,
    ) -> PartialVMResult<BTreeMap<Self::ResourceKey, (StateValueMetadata, u64)>> {
        #[cfg(feature = "blockstm-profiling")]
        let _view_profiler = self.get_profiler_guard(ViewKind::DelayedFieldView);

        match &self.latest_view {
            ViewState::Sync(state) => {
                self.get_group_reads_needing_exchange_parallel(state, delayed_write_set_ids, skip)
            },
            ViewState::Unsync(state) => {
                let read_set = state.read_set.borrow();
                self.get_group_reads_needing_exchange_sequential(
                    &read_set.group_reads,
                    state.unsync_map,
                    delayed_write_set_ids,
                    skip,
                )
            },
        }
    }
}

#[cfg(test)]
mod test {
    //
    // TODO[agg_v2](tests): Refactor the tests, and include resource groups in testing
    //
    use super::*;
    use crate::{
        captured_reads::{CapturedReads, DelayedFieldRead, DelayedFieldReadKind},
        proptest_types::types::{KeyType, MockEvent, ValueType},
        scheduler::{DependencyResult, Scheduler, TWaitForDependency},
        view::{delayed_field_try_add_delta_outcome_impl, get_delayed_field_value_impl, ViewState},
    };
    use aptos_aggregator::{
        bounded_math::{BoundedMath, SignedU128},
        delta_math::DeltaHistory,
        types::{DelayedFieldValue, DelayedFieldsSpeculativeError, ReadPosition},
    };
    use aptos_mvhashmap::{
        types::{MVDelayedFieldsError, TxnIndex},
        unsync_map::UnsyncMap,
        versioned_delayed_fields::TVersionedDelayedFieldView,
        MVHashMap,
    };
    use aptos_types::{
        error::PanicOr,
        state_store::{state_value::StateValue, MockStateView},
        transaction::BlockExecutableTransaction,
        write_set::TransactionWrite,
    };
    use aptos_vm_types::resolver::TResourceView;
    use claims::{assert_err_eq, assert_none, assert_ok_eq, assert_some_eq};
    use move_core_types::value::{IdentifierMappingKind, MoveStructLayout, MoveTypeLayout};
    use move_vm_types::{
        delayed_values::{
            delayed_field_id::DelayedFieldID,
            derived_string_snapshot::{bytes_and_width_to_derived_string_struct, to_utf8_bytes},
        },
        values::{Struct, Value},
    };
    use std::{
        cell::RefCell,
        collections::HashMap,
        sync::atomic::{AtomicBool, AtomicU32},
    };
    use test_case::test_case;

    #[derive(Default)]
    pub struct FakeVersionedDelayedFieldView {
        data: HashMap<DelayedFieldID, DelayedFieldValue>,
    }

    impl FakeVersionedDelayedFieldView {
        pub fn set_value(&mut self, id: DelayedFieldID, value: DelayedFieldValue) {
            self.data.insert(id, value);
        }
    }

    impl TVersionedDelayedFieldView<DelayedFieldID> for FakeVersionedDelayedFieldView {
        fn read(
            &self,
            id: &DelayedFieldID,
            _txn_idx: TxnIndex,
        ) -> Result<DelayedFieldValue, PanicOr<MVDelayedFieldsError>> {
            self.data
                .get(id)
                .cloned()
                .ok_or(PanicOr::Or(MVDelayedFieldsError::NotFound))
        }

        fn read_latest_predicted_value(
            &self,
            id: &DelayedFieldID,
            _current_txn_idx: TxnIndex,
            _read_position: ReadPosition,
        ) -> Result<DelayedFieldValue, MVDelayedFieldsError> {
            self.data
                .get(id)
                .cloned()
                .ok_or(MVDelayedFieldsError::NotFound)
        }
    }

    struct FakeWaitForDependency();

    impl TWaitForDependency for FakeWaitForDependency {
        fn wait_for_dependency(
            &self,
            _txn_idx: TxnIndex,
            _dep_txn_idx: TxnIndex,
        ) -> Result<DependencyResult, PanicError> {
            unreachable!();
        }
    }

    #[derive(Clone, Debug)]
    struct TestTransactionType {}

    impl BlockExecutableTransaction for TestTransactionType {
        type Event = MockEvent;
        type Key = KeyType<u32>;
        type Tag = u32;
        type Value = ValueType;

        fn user_txn_bytes_len(&self) -> usize {
            0
        }
    }

    #[test]
    fn test_history_updates() {
        let mut view = FakeVersionedDelayedFieldView::default();
        let captured_reads = RefCell::new(CapturedReads::<
            TestTransactionType,
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
        >::new(false));
        let wait_for = FakeWaitForDependency();
        let id = DelayedFieldID::new_for_test_for_u64(600);
        let max_value = 600;
        let math = BoundedMath::new(max_value);
        let txn_idx = 1;
        let storage_value = 100;
        view.set_value(id, DelayedFieldValue::Aggregator(storage_value));

        let mut base_delta = SignedU128::Positive(0);
        let base_value_ref = &mut base_delta;

        macro_rules! assert_try_add {
            ($delta:expr, $outcome:expr) => {
                assert_ok_eq!(
                    delayed_field_try_add_delta_outcome_impl(
                        &captured_reads,
                        &view,
                        &wait_for,
                        &id,
                        base_value_ref,
                        &$delta,
                        max_value,
                        txn_idx
                    ),
                    $outcome
                );
                if $outcome {
                    *base_value_ref = math.signed_add(base_value_ref, &$delta).unwrap();
                }
            };
        }

        assert_try_add!(SignedU128::Positive(300), true);
        assert_some_eq!(
            captured_reads
                .borrow()
                .get_delayed_field_by_kind(&id, DelayedFieldReadKind::HistoryBounded),
            DelayedFieldRead::HistoryBounded {
                restriction: DeltaHistory {
                    max_achieved_positive_delta: 300,
                    min_achieved_negative_delta: 0,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },
                max_value,
                inner_aggregator_value: storage_value,
            }
        );

        assert_try_add!(SignedU128::Positive(100), true);
        assert_some_eq!(
            captured_reads
                .borrow()
                .get_delayed_field_by_kind(&id, DelayedFieldReadKind::HistoryBounded),
            DelayedFieldRead::HistoryBounded {
                restriction: DeltaHistory {
                    max_achieved_positive_delta: 400,
                    min_achieved_negative_delta: 0,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },
                max_value,
                inner_aggregator_value: storage_value,
            }
        );

        assert_try_add!(SignedU128::Negative(450), true);
        assert_some_eq!(
            captured_reads
                .borrow()
                .get_delayed_field_by_kind(&id, DelayedFieldReadKind::HistoryBounded),
            DelayedFieldRead::HistoryBounded {
                restriction: DeltaHistory {
                    max_achieved_positive_delta: 400,
                    min_achieved_negative_delta: 50,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },
                max_value,
                inner_aggregator_value: storage_value,
            }
        );

        assert_try_add!(SignedU128::Positive(200), true);
        assert_some_eq!(
            captured_reads
                .borrow()
                .get_delayed_field_by_kind(&id, DelayedFieldReadKind::HistoryBounded),
            DelayedFieldRead::HistoryBounded {
                restriction: DeltaHistory {
                    max_achieved_positive_delta: 400,
                    min_achieved_negative_delta: 50,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },
                max_value,
                inner_aggregator_value: storage_value,
            }
        );

        assert_try_add!(SignedU128::Positive(350), true);
        assert_some_eq!(
            captured_reads
                .borrow()
                .get_delayed_field_by_kind(&id, DelayedFieldReadKind::HistoryBounded),
            DelayedFieldRead::HistoryBounded {
                restriction: DeltaHistory {
                    max_achieved_positive_delta: 500,
                    min_achieved_negative_delta: 50,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },
                max_value,
                inner_aggregator_value: storage_value,
            }
        );

        assert_try_add!(SignedU128::Negative(600), true);
        assert_some_eq!(
            captured_reads
                .borrow()
                .get_delayed_field_by_kind(&id, DelayedFieldReadKind::HistoryBounded),
            DelayedFieldRead::HistoryBounded {
                restriction: DeltaHistory {
                    max_achieved_positive_delta: 500,
                    min_achieved_negative_delta: 100,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },
                max_value,
                inner_aggregator_value: storage_value,
            }
        );
    }

    #[test]
    fn test_aggregator_overflows() {
        let mut view = FakeVersionedDelayedFieldView::default();
        let captured_reads = RefCell::new(CapturedReads::<
            TestTransactionType,
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
        >::new(false));
        let wait_for = FakeWaitForDependency();
        let id = DelayedFieldID::new_for_test_for_u64(600);
        let max_value = 600;
        let math = BoundedMath::new(max_value);
        let txn_idx = 1;
        let storage_value = 100;
        view.set_value(id, DelayedFieldValue::Aggregator(storage_value));

        let mut base_delta = SignedU128::Positive(0);
        let base_value_ref = &mut base_delta;

        macro_rules! assert_try_add {
            ($delta:expr, $outcome:expr) => {
                assert_ok_eq!(
                    delayed_field_try_add_delta_outcome_impl(
                        &captured_reads,
                        &view,
                        &wait_for,
                        &id,
                        base_value_ref,
                        &$delta,
                        max_value,
                        txn_idx
                    ),
                    $outcome
                );
                if $outcome {
                    *base_value_ref = math.signed_add(base_value_ref, &$delta).unwrap();
                }
            };
        }

        assert_try_add!(SignedU128::Positive(400), true);
        assert_some_eq!(
            captured_reads
                .borrow()
                .get_delayed_field_by_kind(&id, DelayedFieldReadKind::HistoryBounded),
            DelayedFieldRead::HistoryBounded {
                restriction: DeltaHistory {
                    max_achieved_positive_delta: 400,
                    min_achieved_negative_delta: 0,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },
                max_value,
                inner_aggregator_value: storage_value,
            }
        );

        assert_try_add!(SignedU128::Negative(450), true);
        assert_some_eq!(
            captured_reads
                .borrow()
                .get_delayed_field_by_kind(&id, DelayedFieldReadKind::HistoryBounded),
            DelayedFieldRead::HistoryBounded {
                restriction: DeltaHistory {
                    max_achieved_positive_delta: 400,
                    min_achieved_negative_delta: 50,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },
                max_value,
                inner_aggregator_value: storage_value,
            }
        );

        assert_try_add!(SignedU128::Positive(601), false);
        assert_some_eq!(
            captured_reads
                .borrow()
                .get_delayed_field_by_kind(&id, DelayedFieldReadKind::HistoryBounded),
            DelayedFieldRead::HistoryBounded {
                restriction: DeltaHistory {
                    max_achieved_positive_delta: 400,
                    min_achieved_negative_delta: 50,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },
                max_value,
                inner_aggregator_value: storage_value,
            }
        );

        assert_try_add!(SignedU128::Positive(575), false);
        assert_some_eq!(
            captured_reads
                .borrow()
                .get_delayed_field_by_kind(&id, DelayedFieldReadKind::HistoryBounded),
            DelayedFieldRead::HistoryBounded {
                restriction: DeltaHistory {
                    max_achieved_positive_delta: 400,
                    min_achieved_negative_delta: 50,
                    min_overflow_positive_delta: Some(525),
                    max_underflow_negative_delta: None,
                },
                max_value,
                inner_aggregator_value: storage_value,
            }
        );

        assert_try_add!(SignedU128::Positive(551), false);
        assert_some_eq!(
            captured_reads
                .borrow()
                .get_delayed_field_by_kind(&id, DelayedFieldReadKind::HistoryBounded),
            DelayedFieldRead::HistoryBounded {
                restriction: DeltaHistory {
                    max_achieved_positive_delta: 400,
                    min_achieved_negative_delta: 50,
                    min_overflow_positive_delta: Some(501),
                    max_underflow_negative_delta: None,
                },
                max_value,
                inner_aggregator_value: storage_value,
            }
        );

        assert_try_add!(SignedU128::Positive(570), false);
        assert_some_eq!(
            captured_reads
                .borrow()
                .get_delayed_field_by_kind(&id, DelayedFieldReadKind::HistoryBounded),
            DelayedFieldRead::HistoryBounded {
                restriction: DeltaHistory {
                    max_achieved_positive_delta: 400,
                    min_achieved_negative_delta: 50,
                    min_overflow_positive_delta: Some(501),
                    max_underflow_negative_delta: None,
                },
                max_value,
                inner_aggregator_value: storage_value,
            }
        );
    }

    #[test]
    fn test_aggregator_underflows() {
        let mut view = FakeVersionedDelayedFieldView::default();
        let captured_reads = RefCell::new(CapturedReads::<
            TestTransactionType,
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
        >::new(false));
        let wait_for = FakeWaitForDependency();
        let id = DelayedFieldID::new_for_test_for_u64(600);
        let max_value = 600;
        let math = BoundedMath::new(max_value);
        let txn_idx = 1;
        let storage_value = 200;
        view.set_value(id, DelayedFieldValue::Aggregator(storage_value));

        let mut base_delta = SignedU128::Positive(0);
        let base_value_ref = &mut base_delta;

        macro_rules! assert_try_add {
            ($delta:expr, $outcome:expr) => {
                assert_ok_eq!(
                    delayed_field_try_add_delta_outcome_impl(
                        &captured_reads,
                        &view,
                        &wait_for,
                        &id,
                        base_value_ref,
                        &$delta,
                        max_value,
                        txn_idx
                    ),
                    $outcome
                );
                if $outcome {
                    *base_value_ref = math.signed_add(base_value_ref, &$delta).unwrap();
                }
            };
        }

        assert_try_add!(SignedU128::Positive(300), true);
        assert_some_eq!(
            captured_reads
                .borrow()
                .get_delayed_field_by_kind(&id, DelayedFieldReadKind::HistoryBounded),
            DelayedFieldRead::HistoryBounded {
                restriction: DeltaHistory {
                    max_achieved_positive_delta: 300,
                    min_achieved_negative_delta: 0,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },
                max_value,
                inner_aggregator_value: storage_value,
            }
        );

        assert_try_add!(SignedU128::Negative(650), false);
        assert_some_eq!(
            captured_reads
                .borrow()
                .get_delayed_field_by_kind(&id, DelayedFieldReadKind::HistoryBounded),
            DelayedFieldRead::HistoryBounded {
                restriction: DeltaHistory {
                    max_achieved_positive_delta: 300,
                    min_achieved_negative_delta: 0,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },
                max_value,
                inner_aggregator_value: storage_value,
            }
        );

        assert_try_add!(SignedU128::Negative(550), false);
        assert_some_eq!(
            captured_reads
                .borrow()
                .get_delayed_field_by_kind(&id, DelayedFieldReadKind::HistoryBounded),
            DelayedFieldRead::HistoryBounded {
                restriction: DeltaHistory {
                    max_achieved_positive_delta: 300,
                    min_achieved_negative_delta: 0,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: Some(250),
                },
                max_value,
                inner_aggregator_value: storage_value,
            }
        );

        assert_try_add!(SignedU128::Negative(525), false);
        assert_some_eq!(
            captured_reads
                .borrow()
                .get_delayed_field_by_kind(&id, DelayedFieldReadKind::HistoryBounded),
            DelayedFieldRead::HistoryBounded {
                restriction: DeltaHistory {
                    max_achieved_positive_delta: 300,
                    min_achieved_negative_delta: 0,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: Some(225),
                },
                max_value,
                inner_aggregator_value: storage_value,
            }
        );

        assert_try_add!(SignedU128::Negative(540), false);
        assert_some_eq!(
            captured_reads
                .borrow()
                .get_delayed_field_by_kind(&id, DelayedFieldReadKind::HistoryBounded),
            DelayedFieldRead::HistoryBounded {
                restriction: DeltaHistory {
                    max_achieved_positive_delta: 300,
                    min_achieved_negative_delta: 0,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: Some(225),
                },
                max_value,
                inner_aggregator_value: storage_value,
            }
        );

        assert_try_add!(SignedU128::Negative(501), false);
        assert_some_eq!(
            captured_reads
                .borrow()
                .get_delayed_field_by_kind(&id, DelayedFieldReadKind::HistoryBounded),
            DelayedFieldRead::HistoryBounded {
                restriction: DeltaHistory {
                    max_achieved_positive_delta: 300,
                    min_achieved_negative_delta: 0,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: Some(201),
                },
                max_value,
                inner_aggregator_value: storage_value,
            }
        );
    }

    #[test]
    fn test_read_kind_upgrade_fail() {
        let mut view = FakeVersionedDelayedFieldView::default();
        let captured_reads = RefCell::new(CapturedReads::<
            TestTransactionType,
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
        >::new(false));
        let wait_for = FakeWaitForDependency();
        let id = DelayedFieldID::new_for_test_for_u64(600);
        let max_value = 600;
        let txn_idx = 1;
        let storage_value = 200;
        view.set_value(id, DelayedFieldValue::Aggregator(storage_value));

        assert_ok_eq!(
            delayed_field_try_add_delta_outcome_impl(
                &captured_reads,
                &view,
                &wait_for,
                &id,
                &SignedU128::Positive(0),
                &SignedU128::Positive(300),
                max_value,
                txn_idx
            ),
            true
        );

        assert_some_eq!(
            captured_reads
                .borrow()
                .get_delayed_field_by_kind(&id, DelayedFieldReadKind::HistoryBounded),
            DelayedFieldRead::HistoryBounded {
                restriction: DeltaHistory {
                    max_achieved_positive_delta: 300,
                    min_achieved_negative_delta: 0,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },
                max_value,
                inner_aggregator_value: storage_value,
            }
        );

        view.set_value(id, DelayedFieldValue::Aggregator(400));
        assert_err_eq!(
            get_delayed_field_value_impl(&captured_reads, &view, &wait_for, &id, txn_idx),
            PanicOr::Or(DelayedFieldsSpeculativeError::InconsistentRead),
        );
    }

    fn create_struct_layout(inner: MoveTypeLayout) -> MoveTypeLayout {
        MoveTypeLayout::Struct(MoveStructLayout::new(vec![inner]))
    }

    fn create_vector_layout(inner: MoveTypeLayout) -> MoveTypeLayout {
        MoveTypeLayout::Vector(Box::new(inner))
    }

    fn create_aggregator_layout(inner: MoveTypeLayout) -> MoveTypeLayout {
        MoveTypeLayout::Struct(MoveStructLayout::new(vec![
            MoveTypeLayout::Native(IdentifierMappingKind::Aggregator, Box::new(inner.clone())),
            inner,
        ]))
    }

    fn create_aggregator_storage_layout(inner: MoveTypeLayout) -> MoveTypeLayout {
        MoveTypeLayout::Struct(MoveStructLayout::new(vec![inner.clone(), inner.clone()]))
    }

    fn create_aggregator_layout_u64() -> MoveTypeLayout {
        create_aggregator_layout(MoveTypeLayout::U64)
    }

    fn create_snapshot_storage_layout(inner: MoveTypeLayout) -> MoveTypeLayout {
        MoveTypeLayout::Struct(MoveStructLayout::new(vec![inner]))
    }

    fn create_snapshot_layout(inner: MoveTypeLayout) -> MoveTypeLayout {
        MoveTypeLayout::Struct(MoveStructLayout::new(vec![MoveTypeLayout::Native(
            IdentifierMappingKind::Snapshot,
            Box::new(inner),
        )]))
    }

    fn create_derived_string_layout() -> MoveTypeLayout {
        MoveTypeLayout::Native(
            IdentifierMappingKind::DerivedString,
            Box::new(MoveTypeLayout::Struct(MoveStructLayout::new(vec![
                create_string_layout(),
                create_vector_layout(MoveTypeLayout::U8),
            ]))),
        )
    }

    fn create_derived_string_storage_layout() -> MoveTypeLayout {
        MoveTypeLayout::Struct(MoveStructLayout::new(vec![
            create_string_layout(),
            create_vector_layout(MoveTypeLayout::U8),
        ]))
    }

    fn create_string_layout() -> MoveTypeLayout {
        MoveTypeLayout::Struct(MoveStructLayout::Runtime(vec![MoveTypeLayout::Vector(
            Box::new(MoveTypeLayout::U8),
        )]))
    }

    fn create_aggregator_value_u64(value: u64, max_value: u64) -> Value {
        Value::struct_(Struct::pack(vec![Value::u64(value), Value::u64(max_value)]))
    }

    fn create_snapshot_value(value: Value) -> Value {
        Value::struct_(Struct::pack(vec![value]))
    }

    fn create_derived_value(value: impl ToString, width: usize) -> Value {
        bytes_and_width_to_derived_string_struct(to_utf8_bytes(value), width).unwrap()
    }

    fn create_struct_value(inner: Value) -> Value {
        Value::struct_(Struct::pack(vec![inner]))
    }

    fn create_vector_value(inner: Vec<Value>) -> Value {
        Value::vector_for_testing_only(inner)
    }

    fn create_state_value(value: &Value, layout: &MoveTypeLayout) -> StateValue {
        StateValue::new_legacy(
            ValueSerDeContext::new()
                .serialize(value, layout)
                .unwrap()
                .unwrap()
                .into(),
        )
    }

    #[test]
    fn test_id_value_exchange() {
        let unsync_map = UnsyncMap::new();
        let counter = RefCell::new(5);
        let base_view = MockStateView::empty();
        let start_counter = 5;
        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let global_module_cache = GlobalModuleCache::empty();

        let latest_view = LatestView::<TestTransactionType, MockStateView<KeyType<u32>>>::new(
            &base_view,
            &global_module_cache,
            &runtime_environment,
            ViewState::Unsync(SequentialState::new(&unsync_map, start_counter, &counter)),
            1,
        );

        // Test id -- value exchange for a value that does not contain delayed fields
        let layout = MoveTypeLayout::Struct(MoveStructLayout::new(vec![
            MoveTypeLayout::U64,
            MoveTypeLayout::U64,
            MoveTypeLayout::U64,
        ]));
        let value = Value::struct_(Struct::pack(vec![
            Value::u64(1),
            Value::u64(2),
            Value::u64(3),
        ]));
        let state_value = create_state_value(&value, &layout);
        let (patched_state_value, identifiers) = latest_view
            .replace_values_with_identifiers(state_value.clone(), &layout)
            .unwrap();
        assert_eq!(state_value, patched_state_value);
        assert!(
            identifiers.is_empty(),
            "No identifiers should have been replaced in this case"
        );
        let (final_state_value, identifiers) = latest_view
            .replace_identifiers_with_values(patched_state_value.bytes(), &layout)
            .unwrap();
        assert_eq!(state_value, StateValue::from(final_state_value.to_vec()));
        assert!(
            identifiers.is_empty(),
            "No identifiers should have been replaced in this case"
        );

        /*
            layout = Struct {
                agg: Aggregator<u64>
            }
        */
        let storage_layout =
            create_struct_layout(create_aggregator_storage_layout(MoveTypeLayout::U64));
        let value = create_struct_value(create_aggregator_value_u64(25, 30));
        let state_value = create_state_value(&value, &storage_layout);

        let layout = create_struct_layout(create_aggregator_layout_u64());
        let (patched_state_value, identifiers) = latest_view
            .replace_values_with_identifiers(state_value.clone(), &layout)
            .unwrap();
        assert_eq!(
            identifiers.len(),
            1,
            "One identifier should have been replaced in this case"
        );
        assert!(
            identifiers.contains(&DelayedFieldID::new_with_width(5, 8)),
            "The value 25 should have been replaced in the identifier 5"
        );
        let (final_state_value, identifiers) = latest_view
            .replace_identifiers_with_values(patched_state_value.bytes(), &layout)
            .unwrap();
        assert_eq!(state_value, StateValue::from(final_state_value.to_vec()));
        assert_eq!(
            identifiers.len(),
            1,
            "One identifier should have been replaced in this case"
        );

        /*
            layout = Struct {
                aggregators: vec![Aggregator<u64>]
            }
        */
        let storage_layout = create_struct_layout(create_vector_layout(
            create_aggregator_storage_layout(MoveTypeLayout::U64),
        ));
        let value = create_struct_value(create_vector_value(vec![
            create_aggregator_value_u64(20, 50),
            create_aggregator_value_u64(35, 65),
            create_aggregator_value_u64(0, 20),
        ]));
        let state_value = create_state_value(&value, &storage_layout);

        let layout = create_struct_layout(create_vector_layout(create_aggregator_layout_u64()));
        let (patched_state_value, identifiers) = latest_view
            .replace_values_with_identifiers(state_value.clone(), &layout)
            .unwrap();
        assert_eq!(
            identifiers.len(),
            3,
            "Three identifiers should have been replaced in this case"
        );
        assert_eq!(
            counter,
            RefCell::new(9),
            "The counter should have been updated to 9"
        );
        let patched_value =
            Value::struct_(Struct::pack(vec![Value::vector_for_testing_only(vec![
                Value::struct_(Struct::pack(vec![
                    Value::u64(DelayedFieldID::new_with_width(6, 8).as_u64()),
                    Value::u64(50),
                ])),
                Value::struct_(Struct::pack(vec![
                    Value::u64(DelayedFieldID::new_with_width(7, 8).as_u64()),
                    Value::u64(65),
                ])),
                Value::struct_(Struct::pack(vec![
                    Value::u64(DelayedFieldID::new_with_width(8, 8).as_u64()),
                    Value::u64(20),
                ])),
            ])]));
        assert_eq!(
            patched_state_value,
            create_state_value(&patched_value, &storage_layout),
        );
        let (final_state_value, identifiers) = latest_view
            .replace_identifiers_with_values(patched_state_value.bytes(), &layout)
            .unwrap();
        assert_eq!(state_value, StateValue::from(final_state_value.to_vec()));
        assert_eq!(
            identifiers.len(),
            3,
            "Three identifiers should have been replaced in this case"
        );

        /*
            layout = Struct {
                aggregators: vec![AggregatorSnapshot<u128>]
            }
        */
        let storage_layout = create_struct_layout(create_vector_layout(
            create_snapshot_storage_layout(MoveTypeLayout::U128),
        ));
        let value = create_struct_value(create_vector_value(vec![
            create_snapshot_value(Value::u128(20)),
            create_snapshot_value(Value::u128(35)),
            create_snapshot_value(Value::u128(0)),
        ]));
        let state_value = create_state_value(&value, &storage_layout);

        let layout = create_struct_layout(create_vector_layout(create_snapshot_layout(
            MoveTypeLayout::U128,
        )));
        let (patched_state_value, identifiers) = latest_view
            .replace_values_with_identifiers(state_value.clone(), &layout)
            .unwrap();
        assert_eq!(
            identifiers.len(),
            3,
            "Three identifiers should have been replaced in this case"
        );
        assert_eq!(
            counter,
            RefCell::new(12),
            "The counter should have been updated to 12"
        );
        let patched_value =
            Value::struct_(Struct::pack(vec![Value::vector_for_testing_only(vec![
                create_snapshot_value(Value::u128(
                    DelayedFieldID::new_with_width(9, 16).as_u64() as u128
                )),
                create_snapshot_value(Value::u128(
                    DelayedFieldID::new_with_width(10, 16).as_u64() as u128
                )),
                create_snapshot_value(Value::u128(
                    DelayedFieldID::new_with_width(11, 16).as_u64() as u128
                )),
            ])]));
        assert_eq!(
            patched_state_value,
            create_state_value(&patched_value, &storage_layout),
        );
        let (final_state_value, identifiers2) = latest_view
            .replace_identifiers_with_values(patched_state_value.bytes(), &layout)
            .unwrap();
        assert_eq!(state_value, StateValue::from(final_state_value.to_vec()));
        assert_eq!(
            identifiers2.len(),
            3,
            "Three identifiers should have been replaced in this case"
        );
        assert_eq!(identifiers, identifiers2);

        /*
            layout = Struct {
                snap: vec![DerivedStringSnapshot]
            }
        */
        let storage_layout =
            create_struct_layout(create_vector_layout(create_derived_string_storage_layout()));
        let value = create_struct_value(create_vector_value(vec![
            create_derived_value("hello", 60),
            create_derived_value("ab", 55),
            create_derived_value("c", 50),
        ]));
        let state_value = create_state_value(&value, &storage_layout);

        let layout = create_struct_layout(create_vector_layout(create_derived_string_layout()));
        let (patched_state_value, identifiers) = latest_view
            .replace_values_with_identifiers(state_value.clone(), &layout)
            .unwrap();
        assert_eq!(
            identifiers.len(),
            3,
            "Three identifiers should have been replaced in this case"
        );
        assert_eq!(
            counter,
            RefCell::new(15),
            "The counter should have been updated to 15"
        );

        let patched_value =
            Value::struct_(Struct::pack(vec![Value::vector_for_testing_only(vec![
                DelayedFieldID::new_with_width(12, 60)
                    .into_derived_string_struct()
                    .unwrap(),
                DelayedFieldID::new_with_width(13, 55)
                    .into_derived_string_struct()
                    .unwrap(),
                DelayedFieldID::new_with_width(14, 50)
                    .into_derived_string_struct()
                    .unwrap(),
            ])]));
        assert_eq!(
            patched_state_value,
            create_state_value(&patched_value, &storage_layout),
        );
        let (final_state_value, identifiers2) = latest_view
            .replace_identifiers_with_values(patched_state_value.bytes(), &layout)
            .unwrap();
        assert_eq!(state_value, StateValue::from(final_state_value.to_vec()));
        assert_eq!(
            identifiers2.len(),
            3,
            "Three identifiers should have been replaced in this case"
        );
        assert_eq!(identifiers, identifiers2);
    }

    struct Holder {
        unsync_map: UnsyncMap<KeyType<u32>, u32, ValueType, DelayedFieldID>,
        counter: RefCell<u32>,
        base_view: MockStateView<KeyType<u32>>,
        empty_global_module_cache:
            GlobalModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension>,
        runtime_environment: RuntimeEnvironment,
        skip_module_validation: AtomicBool,
    }

    impl Holder {
        fn new(data: HashMap<KeyType<u32>, StateValue>, start_counter: u32) -> Self {
            let unsync_map = UnsyncMap::new();
            let counter = RefCell::new(start_counter);
            let base_view = MockStateView::new(data);
            let runtime_environment = RuntimeEnvironment::new(vec![]);
            let skip_module_validation = AtomicBool::new(true);
            Self {
                unsync_map,
                counter,
                base_view,
                empty_global_module_cache: GlobalModuleCache::empty(),
                runtime_environment,
                skip_module_validation,
            }
        }
    }

    fn create_sequential_latest_view<'a>(
        h: &'a Holder,
    ) -> LatestView<'a, TestTransactionType, MockStateView<KeyType<u32>>> {
        let sequential_state: SequentialState<'a, TestTransactionType> =
            SequentialState::new(&h.unsync_map, *h.counter.borrow(), &h.counter);

        LatestView::<'a, TestTransactionType, MockStateView<KeyType<u32>>>::new(
            &h.base_view,
            &h.empty_global_module_cache,
            &h.runtime_environment,
            ViewState::Unsync(sequential_state),
            1,
        )
    }

    struct ComparisonHolder {
        start_counter: u32,
        holder: Holder,
        counter: AtomicU32,
        base_view: MockStateView<KeyType<u32>>,
        runtime_environment: RuntimeEnvironment,
        versioned_map: MVHashMap<KeyType<u32>, u32, ValueType, DelayedFieldID>,
        scheduler: Scheduler,
    }

    impl ComparisonHolder {
        fn new(data: HashMap<KeyType<u32>, StateValue>, start_counter: u32) -> Self {
            let holder = Holder::new(data.clone(), start_counter);
            let counter = AtomicU32::new(start_counter);
            let base_view = MockStateView::new(data);
            let versioned_map = MVHashMap::new();
            let scheduler = Scheduler::new(30);
            let runtime_environment = RuntimeEnvironment::new(vec![]);

            Self {
                start_counter,
                holder,
                counter,
                base_view,
                runtime_environment,
                versioned_map,
                scheduler,
            }
        }

        fn new_view(&self) -> ViewsComparison<'_> {
            let latest_view_seq = create_sequential_latest_view(&self.holder);
            let latest_view_par =
                LatestView::<TestTransactionType, MockStateView<KeyType<u32>>>::new(
                    &self.base_view,
                    &self.holder.empty_global_module_cache,
                    &self.runtime_environment,
                    ViewState::Sync(ParallelState::new(
                        &self.versioned_map,
                        SchedulerWrapper::V1(&self.scheduler, &self.holder.skip_module_validation),
                        self.start_counter,
                        &self.counter,
                        0,
                    )),
                    1,
                );

            ViewsComparison {
                latest_view_seq,
                latest_view_par,
            }
        }
    }

    struct ViewsComparison<'a> {
        latest_view_seq: LatestView<'a, TestTransactionType, MockStateView<KeyType<u32>>>,
        latest_view_par: LatestView<'a, TestTransactionType, MockStateView<KeyType<u32>>>,
    }

    impl<'a> ViewsComparison<'a> {
        fn assert_res_eq<T, E>(&self, res_seq: Result<T, E>, res_par: Result<T, E>) -> Result<T, E>
        where
            T: std::fmt::Debug + PartialEq,
            E: std::fmt::Debug,
        {
            assert_eq!(res_seq.is_ok(), res_par.is_ok());
            if let Ok(res_seq) = res_seq {
                assert_ok_eq!(&res_par, &res_seq);
            }

            assert_eq!(
                self.latest_view_par.get_read_summary(),
                self.latest_view_seq.get_read_summary()
            );

            res_par
        }

        fn get_resource_state_value(
            &self,
            state_key: &KeyType<u32>,
            maybe_layout: Option<&MoveTypeLayout>,
        ) -> PartialVMResult<Option<StateValue>> {
            let seq = self
                .latest_view_seq
                .get_resource_state_value(state_key, maybe_layout);
            let par = self
                .latest_view_par
                .get_resource_state_value(state_key, maybe_layout);

            self.assert_res_eq(seq, par)
        }

        fn resource_exists(&self, state_key: &KeyType<u32>) -> PartialVMResult<bool> {
            let seq = self.latest_view_seq.resource_exists(state_key);
            let par = self.latest_view_par.resource_exists(state_key);

            self.assert_res_eq(seq, par)
        }

        fn get_resource_state_value_metadata(
            &self,
            state_key: &KeyType<u32>,
        ) -> PartialVMResult<Option<StateValueMetadata>> {
            let seq = self
                .latest_view_seq
                .get_resource_state_value_metadata(state_key);
            let par = self
                .latest_view_par
                .get_resource_state_value_metadata(state_key);

            self.assert_res_eq(seq, par)
        }

        fn get_reads_needing_exchange(
            &self,
            delayed_write_set_ids: &HashSet<DelayedFieldID>,
            skip: &HashSet<KeyType<u32>>,
        ) -> Result<
            BTreeMap<KeyType<u32>, (StateValueMetadata, u64, Arc<MoveTypeLayout>)>,
            PanicError,
        > {
            let seq = self
                .latest_view_seq
                .get_reads_needing_exchange(delayed_write_set_ids, skip);
            let par = self
                .latest_view_par
                .get_reads_needing_exchange(delayed_write_set_ids, skip);

            self.assert_res_eq(
                seq.as_ref().map(|m| {
                    m.iter()
                        .map(|(k, (metadata, size, layout))| (*k, (metadata, size, layout.clone())))
                        .collect::<BTreeMap<_, _>>()
                }),
                par.as_ref().map(|m| {
                    m.iter()
                        .map(|(k, (metadata, size, layout))| (*k, (metadata, size, layout.clone())))
                        .collect::<BTreeMap<_, _>>()
                }),
            )
            .unwrap();
            par
        }

        fn get_delayed_field_value(
            &self,
            id: &DelayedFieldID,
        ) -> Result<DelayedFieldValue, PanicOr<DelayedFieldsSpeculativeError>> {
            let seq = self.latest_view_seq.get_delayed_field_value(id);
            let par = self.latest_view_par.get_delayed_field_value(id);

            self.assert_res_eq(seq, par)
        }
    }

    #[test]
    fn test_missing_same() {
        let holder = ComparisonHolder::new(HashMap::new(), 1000);
        let views = holder.new_view();

        assert_ok_eq!(
            views.get_resource_state_value(&KeyType::<u32>(1,), None),
            None
        );

        assert_ok_eq!(views.resource_exists(&KeyType::<u32>(1)), false);

        assert_ok_eq!(
            views.get_resource_state_value_metadata(&KeyType::<u32>(1)),
            None,
        );
    }

    #[test]
    fn test_non_value_reads_not_recorded() {
        let state_value = create_state_value(&Value::u64(12321), &MoveTypeLayout::U64);
        let data = HashMap::from([(KeyType::<u32>(1), state_value.clone())]);

        let holder = ComparisonHolder::new(data, 1000);
        let views = holder.new_view();

        assert_ok_eq!(views.resource_exists(&KeyType::<u32>(1)), true,);
        assert!(views
            .get_resource_state_value_metadata(&KeyType::<u32>(1))
            .unwrap()
            .is_some(),);

        assert_eq!(views.latest_view_par.get_read_summary(), HashSet::new());
        assert_eq!(views.latest_view_seq.get_read_summary(), HashSet::new());
    }

    fn assert_fetch_eq<V: TransactionWrite>(
        fetched: Option<ValueWithLayout<V>>,
        expected_maybe_write: Option<V>,
        expected_maybe_layout: Option<MoveTypeLayout>,
    ) {
        match fetched {
            Some(ValueWithLayout::Exchanged(write, layout)) => {
                let expected_write = expected_maybe_write.unwrap();
                assert_eq!(write.write_op_kind(), expected_write.write_op_kind());
                assert_eq!(write.bytes(), expected_write.bytes());
                assert_eq!(write.as_state_value(), expected_write.as_state_value());
                assert_eq!(
                    layout.as_ref().map(|v| v.as_ref()),
                    expected_maybe_layout.as_ref()
                );
            },
            Some(ValueWithLayout::RawFromStorage(_)) => panic!("Unexpected RawFromStorage"),
            None => {
                assert_none!(expected_maybe_write);
                assert_none!(expected_maybe_layout);
            },
        }
    }

    #[test]
    fn test_regular_read_operations() {
        let state_value = create_state_value(&Value::u64(12321), &MoveTypeLayout::U64);
        let data = HashMap::from([(KeyType::<u32>(1), state_value.clone())]);

        let holder = ComparisonHolder::new(data, 1000);
        let views = holder.new_view();

        assert_ok_eq!(
            views.get_resource_state_value(&KeyType::<u32>(1), None),
            Some(state_value.clone())
        );

        assert_fetch_eq(
            holder
                .holder
                .unsync_map
                .fetch_data(&KeyType::<u32>(1)),
            Some(TransactionWrite::from_state_value(Some(state_value))),
            None,
        );
    }

    #[test_case(Some(true))]
    #[test_case(Some(false))]
    #[test_case(None)]
    fn test_aggregator_read_operations(check_metadata: Option<bool>) {
        let storage_layout =
            create_struct_layout(create_aggregator_storage_layout(MoveTypeLayout::U64));
        let value = create_struct_value(create_aggregator_value_u64(25, 30));
        let state_value = create_state_value(&value, &storage_layout);
        let data = HashMap::from([(KeyType::<u32>(1), state_value.clone())]);

        let start_counter = 1000;
        let id = DelayedFieldID::new_with_width(start_counter, 8);

        let holder = ComparisonHolder::new(data, start_counter);
        let views = holder.new_view();

        let patched_value = create_struct_value(create_aggregator_value_u64(id.as_u64(), 30));
        let patched_state_value = create_state_value(&patched_value, &storage_layout);

        match check_metadata {
            Some(true) => {
                views
                    .get_resource_state_value_metadata(&KeyType::<u32>(1))
                    .unwrap();
            },
            Some(false) => {
                assert_ok_eq!(views.resource_exists(&KeyType::<u32>(1)), true,);
            },
            None => {},
        };

        let layout = create_struct_layout(create_aggregator_layout_u64());
        assert_ok_eq!(
            views.get_resource_state_value(&KeyType::<u32>(1), Some(&layout)),
            Some(patched_state_value.clone())
        );
        assert!(views
            .get_reads_needing_exchange(&HashSet::from([id]), &HashSet::new())
            .unwrap()
            .contains_key(&KeyType(1)));
        assert_fetch_eq(
            holder
                .holder
                .unsync_map
                .fetch_data(&KeyType::<u32>(1)),
            Some(TransactionWrite::from_state_value(Some(
                patched_state_value,
            ))),
            Some(layout),
        );
    }

    #[test]
    fn test_read_operations() {
        let state_value_3 = create_state_value(&Value::u64(12321), &MoveTypeLayout::U64);
        let mut data = HashMap::new();
        data.insert(KeyType::<u32>(3), state_value_3.clone());
        let storage_layout =
            create_struct_layout(create_aggregator_storage_layout(MoveTypeLayout::U64));
        let value = create_struct_value(create_aggregator_value_u64(25, 30));
        let state_value_4 = create_state_value(&value, &storage_layout);
        data.insert(KeyType::<u32>(4), state_value_4);

        let start_counter = 1000;
        let id = DelayedFieldID::new_with_width(start_counter, 8);
        let holder = ComparisonHolder::new(data, start_counter);
        let views = holder.new_view();

        assert_eq!(
            views
                .get_resource_state_value(&KeyType::<u32>(1), None)
                .unwrap(),
            None
        );
        let layout = create_struct_layout(create_aggregator_layout_u64());
        assert_eq!(
            views
                .get_resource_state_value(&KeyType::<u32>(2), Some(&layout))
                .unwrap(),
            None
        );
        assert_eq!(
            views
                .get_resource_state_value(&KeyType::<u32>(3), None)
                .unwrap(),
            Some(state_value_3.clone())
        );

        let patched_value = create_struct_value(create_aggregator_value_u64(id.as_u64(), 30));
        let state_value_4 = create_state_value(&patched_value, &storage_layout);
        assert_eq!(
            views
                .get_resource_state_value(&KeyType::<u32>(4), Some(&layout))
                .unwrap(),
            Some(state_value_4.clone())
        );

        // When we throw exception, it is not required read summaries to match, as they will not be used
        // assert_err_eq!(
        //     views.get_delayed_field_value(&DelayedFieldID::new_for_test_for_u64(1005)),
        //     PanicOr::Or(DelayedFieldsSpeculativeError::NotFound(DelayedFieldID::new_for_test_for_u64(1005))),
        // );

        assert_ok_eq!(
            views.get_delayed_field_value(&id),
            DelayedFieldValue::Aggregator(25),
        );

        let captured_reads = views.latest_view_par.take_parallel_reads();
        assert!(captured_reads.validate_data_reads(holder.versioned_map.data(), 1));
        // TODO(aggr_v2): what's up with this test case?
        let _read_set_with_delayed_fields = captured_reads.get_read_values_with_delayed_fields(
            &views.latest_view_par,
            &HashSet::new(),
            &HashSet::new(),
        );

        // TODO[agg_v2](test): This prints
        // read: (KeyType(4, false), Versioned(Err(StorageVersion), Some(Struct(Runtime([Struct(Runtime([Tagged(IdentifierMapping(Aggregator), U64), U64]))])))))
        // read: (KeyType(2, false), Versioned(Err(StorageVersion), Some(Struct(Runtime([Struct(Runtime([Tagged(IdentifierMapping(Aggregator), U64), U64]))])))))
        // for read in read_set_with_delayed_fields {
        //     println!("read: {:?}", read);
        // }

        // TODO[agg_v2](test): This assertion fails.
        // let data_read = DataRead::Versioned(Ok((1,0)), Arc::new(TransactionWrite::from_state_value(Some(state_value_4))), Some(Arc::new(layout)));
        // assert!(read_set_with_delayed_fields.any(|x| x == (&KeyType::<u32>(4, false), &data_read)));
    }
}
