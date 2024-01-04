// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    abstract_write_op::{
        AbstractResourceWriteOp, GroupWrite, InPlaceDelayedFieldChangeOp,
        ResourceGroupInPlaceDelayedFieldChangeOp, WriteWithDelayedFieldsOp,
    },
    check_change_set::CheckChangeSet,
};
use aptos_aggregator::{
    delayed_change::DelayedChange,
    delta_change_set::{serialize, DeltaOp},
    resolver::AggregatorV1Resolver,
    types::{code_invariant_error, DelayedFieldID},
};
use aptos_types::{
    aggregator::PanicError,
    contract_event::ContractEvent,
    state_store::{
        state_key::{StateKey, StateKeyInner},
        state_value::StateValueMetadata,
    },
    transaction::ChangeSet as StorageChangeSet,
    write_set::{TransactionWrite, WriteOp, WriteOpSize, WriteSetMut},
};
use move_binary_format::errors::{Location, PartialVMError, PartialVMResult, VMResult};
use move_core_types::{value::MoveTypeLayout, vm_status::StatusCode};
use std::{
    collections::{
        btree_map::Entry::{Occupied, Vacant},
        BTreeMap,
    },
    hash::Hash,
    sync::Arc,
};

/// A change set produced by the VM.
///
/// **WARNING**: Just like VMOutput, this type should only be used inside the
/// VM. For storage backends, use `ChangeSet`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VMChangeSet {
    resource_write_set: BTreeMap<StateKey, AbstractResourceWriteOp>,
    module_write_set: BTreeMap<StateKey, WriteOp>,
    events: Vec<(ContractEvent, Option<MoveTypeLayout>)>,

    // Changes separated out from the writes, for better concurrency,
    // materialized back into resources when transaction output is computed.
    delayed_field_change_set: BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,

    // TODO[agg_v1](cleanup) deprecate aggregator_v1 fields.
    aggregator_v1_write_set: BTreeMap<StateKey, WriteOp>,
    aggregator_v1_delta_set: BTreeMap<StateKey, DeltaOp>,
}

macro_rules! squash_writes_pair {
    ($write_entry:ident, $additional_write:ident) => {
        // Squashing creation and deletion is a no-op. In that case, we
        // have to remove the old write op from the write set.
        let noop = !WriteOp::squash($write_entry.get_mut(), $additional_write).map_err(|e| {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message(format!("Error while squashing two write ops: {}.", e))
        })?;
        if noop {
            $write_entry.remove();
        }
    };
}

impl VMChangeSet {
    pub fn empty() -> Self {
        Self {
            resource_write_set: BTreeMap::new(),
            module_write_set: BTreeMap::new(),
            events: vec![],
            delayed_field_change_set: BTreeMap::new(),
            aggregator_v1_write_set: BTreeMap::new(),
            aggregator_v1_delta_set: BTreeMap::new(),
        }
    }

    pub fn new(
        resource_write_set: BTreeMap<StateKey, AbstractResourceWriteOp>,
        module_write_set: BTreeMap<StateKey, WriteOp>,
        events: Vec<(ContractEvent, Option<MoveTypeLayout>)>,
        delayed_field_change_set: BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
        aggregator_v1_write_set: BTreeMap<StateKey, WriteOp>,
        aggregator_v1_delta_set: BTreeMap<StateKey, DeltaOp>,
        checker: &dyn CheckChangeSet,
    ) -> PartialVMResult<Self> {
        let change_set = Self {
            resource_write_set,
            module_write_set,
            events,
            delayed_field_change_set,
            aggregator_v1_write_set,
            aggregator_v1_delta_set,
        };
        // Returns an error if structure of the change set is not valid,
        // e.g. the size in bytes is too large.
        checker.check_change_set(&change_set)?;
        Ok(change_set)
    }

    // TODO[agg_v2](cleanup) see if we can remove in favor of `new`.
    pub fn new_expanded(
        resource_write_set: BTreeMap<StateKey, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
        resource_group_write_set: BTreeMap<StateKey, GroupWrite>,
        module_write_set: BTreeMap<StateKey, WriteOp>,
        aggregator_v1_write_set: BTreeMap<StateKey, WriteOp>,
        aggregator_v1_delta_set: BTreeMap<StateKey, DeltaOp>,
        delayed_field_change_set: BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
        reads_needing_delayed_field_exchange: BTreeMap<StateKey, (WriteOp, Arc<MoveTypeLayout>)>,
        group_reads_needing_delayed_field_exchange: BTreeMap<StateKey, (WriteOp, u64)>,
        events: Vec<(ContractEvent, Option<MoveTypeLayout>)>,
        checker: &dyn CheckChangeSet,
    ) -> PartialVMResult<Self> {
        Self::new(
            resource_write_set
                .into_iter()
                .map::<PartialVMResult<_>, _>(|(k, (w, l))| {
                    Ok((
                        k,
                        AbstractResourceWriteOp::from_resource_write_with_maybe_layout(w, l),
                    ))
                })
                .chain(
                    resource_group_write_set
                        .into_iter()
                        .map(|(k, w)| Ok((k, AbstractResourceWriteOp::WriteResourceGroup(w)))),
                )
                .chain(
                    reads_needing_delayed_field_exchange
                        .into_iter()
                        .map(|(k, (w, layout))| {
                            Ok((
                                k,
                                AbstractResourceWriteOp::InPlaceDelayedFieldChange(
                                    InPlaceDelayedFieldChangeOp {
                                        layout,
                                        materialized_size: WriteOpSize::from(&w)
                                            .write_len()
                                            .ok_or_else(|| {
                                                PartialVMError::new(
                                                    StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR,
                                                )
                                                .with_message(
                                                    "Read with exchange cannot be a delete."
                                                        .to_string(),
                                                )
                                            })?,
                                        metadata: w.into_metadata(),
                                    },
                                ),
                            ))
                        }),
                )
                .chain(group_reads_needing_delayed_field_exchange.into_iter().map(
                    |(k, (metadata_op, materialized_size))| {
                        Ok((
                            k,
                            AbstractResourceWriteOp::ResourceGroupInPlaceDelayedFieldChange(
                                ResourceGroupInPlaceDelayedFieldChangeOp {
                                    metadata_op,
                                    materialized_size,
                                },
                            ),
                        ))
                    },
                ))
                .try_fold::<_, _, PartialVMResult<BTreeMap<_, _>>>(
                    BTreeMap::new(),
                    |mut acc, element| {
                        let (key, value) = element?;
                        if acc.insert(key, value).is_some() {
                            Err(PartialVMError::new(
                                StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR,
                            )
                            .with_message(
                                "Found duplicate key across resource change sets.".to_string(),
                            ))
                        } else {
                            Ok(acc)
                        }
                    },
                )?,
            module_write_set,
            events,
            delayed_field_change_set,
            aggregator_v1_write_set,
            aggregator_v1_delta_set,
            checker,
        )
    }

    /// Builds a new change set from the storage representation.
    ///
    /// **WARNING**: Has complexity O(#write_ops) because we need to iterate
    /// over blobs and split them into resources or modules. Only used to
    /// support transactions with write-set payload.
    ///
    /// Note: does not separate out individual resource group updates.
    pub fn try_from_storage_change_set(
        change_set: StorageChangeSet,
        checker: &dyn CheckChangeSet,
        // Pass in within which resolver context are we creating this change set.
        // Used to eagerly reject changes created in an incompatible way.
        is_delayed_field_optimization_capable: bool,
    ) -> VMResult<Self> {
        assert!(
            !is_delayed_field_optimization_capable,
            "try_from_storage_change_set can only be called in non-is_delayed_field_optimization_capable context, as it doesn't support delayed field changes (type layout) and resource groups");

        let (write_set, events) = change_set.into_inner();

        // There should be no aggregator writes if we have a change set from
        // storage.
        let mut resource_write_set = BTreeMap::new();
        let mut module_write_set = BTreeMap::new();

        for (state_key, write_op) in write_set {
            if matches!(state_key.inner(), StateKeyInner::AccessPath(ap) if ap.is_code()) {
                module_write_set.insert(state_key, write_op);
            } else {
                // TODO[agg_v1](fix) While everything else must be a resource, first
                // version of aggregators is implemented as a table item. Revisit when
                // we split MVHashMap into data and aggregators.

                // We can set layout to None, as we are not in the is_delayed_field_optimization_capable context
                resource_write_set.insert(state_key, AbstractResourceWriteOp::Write(write_op));
            }
        }

        // We can set layout to None, as we are not in the is_delayed_field_optimization_capable context
        let events = events.into_iter().map(|event| (event, None)).collect();
        let change_set = Self {
            // TODO[agg_v2](fix): do we use same or different capable flag for resource groups?
            // We should skip unpacking resource groups, as we are not in the is_delayed_field_optimization_capable
            // context (i.e. if dynamic_change_set_optimizations_enabled is disabled)
            resource_write_set,
            module_write_set,
            delayed_field_change_set: BTreeMap::new(),
            aggregator_v1_write_set: BTreeMap::new(),
            aggregator_v1_delta_set: BTreeMap::new(),
            events,
        };
        checker
            .check_change_set(&change_set)
            .map_err(|e| e.finish(Location::Undefined))?;
        Ok(change_set)
    }

    /// Converts VM-native change set into its storage representation with fully
    /// serialized changes. The conversion fails if:
    /// - deltas are not materialized.
    /// - resource group writes are not (combined &) converted to resource writes.
    pub fn try_into_storage_change_set(self) -> Result<StorageChangeSet, PanicError> {
        // Converting VMChangeSet into TransactionOutput (i.e. storage change set), can
        // be done here only if dynamic_change_set_optimizations have not been used/produced
        // data into the output.
        // If they (DelayedField or ResourceGroup) have added data into the write set, translation
        // into output is more complicated, and needs to be done within BlockExecutor context
        // that knows how to deal with it.
        let Self {
            resource_write_set,
            module_write_set,
            aggregator_v1_write_set,
            aggregator_v1_delta_set,
            delayed_field_change_set,
            events,
        } = self;

        if !aggregator_v1_delta_set.is_empty() {
            return Err(code_invariant_error(
                "Cannot convert from VMChangeSet with non-materialized Aggregator V1 deltas to ChangeSet.",
            ));
        }
        if !delayed_field_change_set.is_empty() {
            return Err(code_invariant_error(
                "Cannot convert from VMChangeSet with non-materialized Delayed Field changes to ChangeSet.",
            ));
        }

        let mut write_set_mut = WriteSetMut::default();
        write_set_mut.extend(
            resource_write_set
                .into_iter()
                .map(|(k, v)| {
                    Ok((
                        k,
                        v.try_into_concrete_write().ok_or_else(|| {
                            code_invariant_error(
                                "Cannot convert from VMChangeSet with non-materialized write set",
                            )
                        })?,
                    ))
                })
                .collect::<Result<Vec<_>, _>>()?,
        );
        write_set_mut.extend(module_write_set);
        write_set_mut.extend(aggregator_v1_write_set);

        let events = events.into_iter().map(|(e, _)| e).collect();
        let write_set = write_set_mut
            .freeze()
            .expect("Freezing a WriteSet does not fail.");
        Ok(StorageChangeSet::new(write_set, events))
    }

    pub fn concrete_write_set_iter(&self) -> impl Iterator<Item = (&StateKey, Option<&WriteOp>)> {
        self.resource_write_set()
            .iter()
            .map(|(k, v)| (k, v.try_as_concrete_write()))
            .chain(
                self.module_write_set()
                    .iter()
                    .chain(self.aggregator_v1_write_set().iter())
                    .map(|(k, v)| (k, Some(v))),
            )
    }

    pub fn write_set_size_iter(&self) -> impl Iterator<Item = (&StateKey, WriteOpSize)> {
        self.resource_write_set()
            .iter()
            .map(|(k, v)| (k, v.materialized_size()))
            .chain(
                self.module_write_set()
                    .iter()
                    .chain(self.aggregator_v1_write_set().iter())
                    .map(|(k, v)| (k, WriteOpSize::from(v))),
            )
    }

    pub fn num_write_ops(&self) -> usize {
        self.resource_write_set().len()
            + self.module_write_set().len()
            + self.aggregator_v1_write_set().len()
    }

    /// Deposit amount is inserted into metadata at a different time than the WriteOp is created.
    /// So this method is needed to be able to update metadata generically across different variants.
    pub fn write_set_iter_mut(
        &mut self,
    ) -> impl Iterator<Item = (&StateKey, WriteOpSize, &mut StateValueMetadata)> {
        self.resource_write_set
            .iter_mut()
            .map(|(k, v)| (k, v.materialized_size(), v.get_metadata_mut()))
            .chain(
                self.module_write_set
                    .iter_mut()
                    .chain(self.aggregator_v1_write_set.iter_mut())
                    .map(|(k, v)| (k, WriteOpSize::from(v as &WriteOp), v.get_metadata_mut())),
            )
    }

    pub fn resource_write_set(&self) -> &BTreeMap<StateKey, AbstractResourceWriteOp> {
        &self.resource_write_set
    }

    pub fn module_write_set(&self) -> &BTreeMap<StateKey, WriteOp> {
        &self.module_write_set
    }

    // Called by `into_transaction_output_with_materialized_writes` only.
    pub(crate) fn extend_aggregator_v1_write_set(
        &mut self,
        additional_aggregator_writes: impl Iterator<Item = (StateKey, WriteOp)>,
    ) {
        self.aggregator_v1_write_set
            .extend(additional_aggregator_writes)
    }

    // Called by `into_transaction_output_with_materialized_writes` only.
    pub(crate) fn extend_resource_write_set(
        &mut self,
        patched_resource_writes: impl Iterator<Item = (StateKey, WriteOp)>,
    ) -> Result<(), PanicError> {
        for (key, new_write) in patched_resource_writes {
            let abstract_write = self.resource_write_set.get_mut(&key).ok_or_else(|| {
                code_invariant_error(format!(
                    "Cannot patch a resource which does not exist, for: {:?}.",
                    key
                ))
            })?;

            if let AbstractResourceWriteOp::Write(w) = &abstract_write {
                return Err(code_invariant_error(format!(
                    "Trying to patch the value that is already materialized: {:?}: {:?} into {:?}.",
                    key, w, new_write
                )));
            }

            let new_length = WriteOpSize::from(&new_write).write_len();
            let old_length = abstract_write.materialized_size().write_len();
            if new_length != old_length {
                return Err(code_invariant_error(format!(
                    "Trying to patch the value that changed size during materialization: {:?}: {:?} into {:?}. \nValues {:?} into {:?}.", key, old_length, new_length, abstract_write, new_write,
                )));
            }

            *abstract_write = AbstractResourceWriteOp::Write(new_write);
        }
        Ok(())
    }

    /// The events are set to the input events.
    pub(crate) fn set_events(&mut self, patched_events: impl Iterator<Item = ContractEvent>) {
        self.events = patched_events
            .map(|event| (event, None))
            .collect::<Vec<_>>();
    }

    pub(crate) fn drain_delayed_field_change_set(
        &mut self,
    ) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        std::mem::take(&mut self.delayed_field_change_set)
    }

    pub(crate) fn drain_aggregator_v1_delta_set(&mut self) -> BTreeMap<StateKey, DeltaOp> {
        std::mem::take(&mut self.aggregator_v1_delta_set)
    }

    pub fn aggregator_v1_write_set(&self) -> &BTreeMap<StateKey, WriteOp> {
        &self.aggregator_v1_write_set
    }

    pub fn aggregator_v1_delta_set(&self) -> &BTreeMap<StateKey, DeltaOp> {
        &self.aggregator_v1_delta_set
    }

    pub fn delayed_field_change_set(
        &self,
    ) -> &BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        &self.delayed_field_change_set
    }

    pub fn events(&self) -> &[(ContractEvent, Option<MoveTypeLayout>)] {
        &self.events
    }

    /// Materializes this change set: all aggregator v1 deltas are converted into writes and
    /// are combined with existing aggregator writes. The aggregator v2 changeset is not touched.
    pub fn try_materialize_aggregator_v1_delta_set(
        &mut self,
        resolver: &impl AggregatorV1Resolver,
    ) -> VMResult<()> {
        let into_write =
            |(state_key, delta): (StateKey, DeltaOp)| -> VMResult<(StateKey, WriteOp)> {
                // Materialization is needed when committing a transaction, so
                // we need precise mode to compute the true value of an
                // aggregator.
                let write =
                    resolver.try_convert_aggregator_v1_delta_into_write_op(&state_key, &delta)?;
                Ok((state_key, write))
            };

        let aggregator_v1_delta_set = std::mem::take(&mut self.aggregator_v1_delta_set);
        let materialized_aggregator_delta_set = aggregator_v1_delta_set
            .into_iter()
            .map(into_write)
            .collect::<VMResult<BTreeMap<StateKey, WriteOp>>>()?;
        self.aggregator_v1_write_set
            .extend(materialized_aggregator_delta_set);
        Ok(())
    }

    fn squash_additional_aggregator_v1_changes(
        aggregator_v1_write_set: &mut BTreeMap<StateKey, WriteOp>,
        aggregator_v1_delta_set: &mut BTreeMap<StateKey, DeltaOp>,
        additional_aggregator_v1_write_set: BTreeMap<StateKey, WriteOp>,
        additional_aggregator_v1_delta_set: BTreeMap<StateKey, DeltaOp>,
    ) -> PartialVMResult<()> {
        use WriteOp::*;

        // First, squash deltas.
        for (state_key, additional_delta_op) in additional_aggregator_v1_delta_set {
            if let Some(write_op) = aggregator_v1_write_set.get_mut(&state_key) {
                // In this case, delta follows a write op.
                match write_op {
                    Creation { data, .. } | Modification { data, .. } => {
                        // Apply delta on top of creation or modification.
                        // TODO[agg_v1](cleanup): This will not be needed anymore once aggregator
                        // change sets carry non-serialized information.
                        let base: u128 = bcs::from_bytes(data)
                            .expect("Deserializing into an aggregator value always succeeds");
                        let value = additional_delta_op
                            .apply_to(base)
                            .map_err(PartialVMError::from)?;
                        *data = serialize(&value).into();
                    },
                    Deletion { .. } => {
                        // This case (applying a delta to deleted item) should
                        // never happen. Let's still return an error instead of
                        // panicking.
                        return Err(PartialVMError::new(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                        )
                        .with_message(
                            "Cannot squash delta which was already deleted.".to_string(),
                        ));
                    },
                }
            } else {
                // Otherwise, this is a either a new delta or an additional delta
                // for the same state key.
                match aggregator_v1_delta_set.entry(state_key) {
                    Occupied(entry) => {
                        // In this case, we need to merge the new incoming delta
                        // to the existing delta, ensuring the strict ordering.
                        entry
                            .into_mut()
                            .merge_with_next_delta(additional_delta_op)
                            .map_err(PartialVMError::from)?;
                    },
                    Vacant(entry) => {
                        // We see this delta for the first time, so simply add it
                        // to the set.
                        entry.insert(additional_delta_op);
                    },
                }
            }
        }

        // Next, squash write ops.
        for (state_key, additional_write_op) in additional_aggregator_v1_write_set {
            match aggregator_v1_write_set.entry(state_key) {
                Occupied(mut entry) => {
                    squash_writes_pair!(entry, additional_write_op);
                },
                Vacant(entry) => {
                    // This is a new write op. It can overwrite a delta so we
                    // have to make sure we remove such a delta from the set in
                    // this case.
                    let removed_delta = aggregator_v1_delta_set.remove(entry.key());

                    // We cannot create after modification with a delta!
                    if removed_delta.is_some() && additional_write_op.is_creation() {
                        return Err(PartialVMError::new(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                        )
                        .with_message(
                            "Cannot create a resource after modification with a delta.".to_string(),
                        ));
                    }

                    entry.insert(additional_write_op);
                },
            }
        }

        Ok(())
    }

    fn squash_additional_delayed_field_changes(
        change_set: &mut BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
        additional_change_set: BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
    ) -> PartialVMResult<()> {
        let merged_changes = additional_change_set
            .into_iter()
            .map(|(id, additional_change)| {
                let prev_change =
                    if let Some(dependent_id) = additional_change.get_merge_dependent_id() {
                        if change_set.contains_key(&id) {
                            return (
                                id,
                                Err(code_invariant_error(format!(
                                "Aggregator change set contains both {:?} and its dependent {:?}",
                                id, dependent_id
                            ))
                                .into()),
                            );
                        }
                        change_set.get(&dependent_id)
                    } else {
                        change_set.get(&id)
                    };
                (
                    id,
                    DelayedChange::merge_two_changes(prev_change, &additional_change),
                )
            })
            .collect::<Vec<_>>();

        for (id, merged_change) in merged_changes.into_iter() {
            change_set.insert(id, merged_change.map_err(PartialVMError::from)?);
        }
        Ok(())
    }

    fn squash_additional_module_writes(
        write_set: &mut BTreeMap<StateKey, WriteOp>,
        additional_write_set: BTreeMap<StateKey, WriteOp>,
    ) -> PartialVMResult<()> {
        for (key, additional_write_op) in additional_write_set.into_iter() {
            match write_set.entry(key) {
                Occupied(mut entry) => {
                    squash_writes_pair!(entry, additional_write_op);
                },
                Vacant(entry) => {
                    entry.insert(additional_write_op);
                },
            }
        }
        Ok(())
    }

    fn squash_additional_resource_write_ops<
        K: Hash + Eq + PartialEq + Ord + Clone + std::fmt::Debug,
    >(
        write_set: &mut BTreeMap<K, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
        additional_write_set: BTreeMap<K, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
    ) -> Result<(), PanicError> {
        for (key, additional_entry) in additional_write_set.into_iter() {
            match write_set.entry(key.clone()) {
                Occupied(mut entry) => {
                    // Squash entry and additional entries if type layouts match.
                    let (additional_write_op, additional_type_layout) = additional_entry;
                    let (write_op, type_layout) = entry.get_mut();
                    if *type_layout != additional_type_layout {
                        return Err(code_invariant_error(format!(
                            "Cannot squash two writes with different type layouts.
                            key: {:?}, type_layout: {:?}, additional_type_layout: {:?}",
                            key, type_layout, additional_type_layout
                        )));
                    }
                    let noop = !WriteOp::squash(write_op, additional_write_op).map_err(|e| {
                        code_invariant_error(format!("Error while squashing two write ops: {}.", e))
                    })?;
                    if noop {
                        entry.remove();
                    }
                },
                Vacant(entry) => {
                    entry.insert(additional_entry);
                },
            }
        }
        Ok(())
    }

    // pub(crate) only for testing
    pub(crate) fn squash_additional_resource_writes(
        write_set: &mut BTreeMap<StateKey, AbstractResourceWriteOp>,
        additional_write_set: BTreeMap<StateKey, AbstractResourceWriteOp>,
    ) -> Result<(), PanicError> {
        use AbstractResourceWriteOp::*;
        for (key, additional_entry) in additional_write_set.into_iter() {
            match write_set.entry(key.clone()) {
                Vacant(entry) => {
                    entry.insert(additional_entry);
                },
                Occupied(mut entry) => {
                    let (to_delete, to_overwrite) = match (entry.get_mut(), &additional_entry) {
                        (Write(write_op), Write(additional_write_op)) => {
                            let to_delete = !WriteOp::squash(write_op, additional_write_op.clone())
                                .map_err(|e| {
                                    code_invariant_error(format!(
                                        "Error while squashing two write ops: {}.",
                                        e
                                    ))
                                })?;
                            (to_delete, false)
                        },
                        (
                            WriteWithDelayedFields(WriteWithDelayedFieldsOp {
                                write_op,
                                layout,
                                materialized_size,
                            }),
                            WriteWithDelayedFields(WriteWithDelayedFieldsOp {
                                write_op: additional_write_op,
                                layout: additional_layout,
                                materialized_size: additional_materialized_size,
                            }),
                        ) => {
                            if layout != additional_layout {
                                return Err(code_invariant_error(format!(
                                    "Cannot squash two writes with different type layouts.
                                    key: {:?}, type_layout: {:?}, additional_type_layout: {:?}",
                                    key, layout, additional_layout
                                )));
                            }
                            let to_delete = !WriteOp::squash(write_op, additional_write_op.clone())
                                .map_err(|e| {
                                    code_invariant_error(format!(
                                        "Error while squashing two write ops: {}.",
                                        e
                                    ))
                                })?;
                            *materialized_size = *additional_materialized_size;
                            (to_delete, false)
                        },
                        (
                            WriteResourceGroup(group),
                            WriteResourceGroup(GroupWrite {
                                metadata_op: additional_metadata_op,
                                inner_ops: additional_inner_ops,
                                maybe_group_op_size: additional_maybe_group_op_size,
                            }),
                        ) => {
                            // Squashing creation and deletion is a no-op. In that case, we have to
                            // remove the old GroupWrite from the group write set.
                            let to_delete = !WriteOp::squash(
                                &mut group.metadata_op,
                                additional_metadata_op.clone(),
                            )
                            .map_err(|e| {
                                code_invariant_error(format!(
                                    "Error while squashing two group write metadata ops: {}.",
                                    e
                                ))
                            })?;
                            if to_delete {
                                (true, false)
                            } else {
                                Self::squash_additional_resource_write_ops(
                                    &mut group.inner_ops,
                                    additional_inner_ops.clone(),
                                )?;

                                group.maybe_group_op_size = *additional_maybe_group_op_size;
                                (false, false)
                            }
                        },
                        (
                            WriteWithDelayedFields(WriteWithDelayedFieldsOp {
                                materialized_size,
                                ..
                            }),
                            InPlaceDelayedFieldChange(InPlaceDelayedFieldChangeOp {
                                materialized_size: additional_materialized_size,
                                ..
                            }),
                        )
                        | (
                            WriteResourceGroup(GroupWrite {
                                maybe_group_op_size: materialized_size,
                                ..
                            }),
                            ResourceGroupInPlaceDelayedFieldChange(
                                ResourceGroupInPlaceDelayedFieldChangeOp {
                                    materialized_size: additional_materialized_size,
                                    ..
                                },
                            ),
                        ) => {
                            // newer read should've read the original write and contain all info from it,
                            // but could have additional delayed field writes, that change the size.
                            *materialized_size = Some(*additional_materialized_size);
                            (false, false)
                        },
                        // If previous value is a read, newer value overwrites it
                        (
                            InPlaceDelayedFieldChange(_),
                            WriteWithDelayedFields(_) | InPlaceDelayedFieldChange(_),
                        )
                        | (
                            ResourceGroupInPlaceDelayedFieldChange(_),
                            WriteResourceGroup(_) | ResourceGroupInPlaceDelayedFieldChange(_),
                        ) => (false, true),
                        (
                            Write(_),
                            WriteWithDelayedFields(_)
                            | WriteResourceGroup(_)
                            | InPlaceDelayedFieldChange(_)
                            | ResourceGroupInPlaceDelayedFieldChange(_),
                        )
                        | (
                            WriteWithDelayedFields(_),
                            Write(_)
                            | WriteResourceGroup(_)
                            | ResourceGroupInPlaceDelayedFieldChange(_),
                        )
                        | (
                            WriteResourceGroup(_),
                            Write(_) | WriteWithDelayedFields(_) | InPlaceDelayedFieldChange(_),
                        )
                        | (
                            InPlaceDelayedFieldChange(_),
                            Write(_)
                            | WriteResourceGroup(_)
                            | ResourceGroupInPlaceDelayedFieldChange(_),
                        )
                        | (
                            ResourceGroupInPlaceDelayedFieldChange(_),
                            Write(_) | WriteWithDelayedFields(_) | InPlaceDelayedFieldChange(_),
                        ) => {
                            return Err(code_invariant_error(format!(
                                "Trying to squash incompatible writes: {:?}: {:?} into {:?}.",
                                entry.key(),
                                entry.get(),
                                additional_entry
                            )));
                        },
                    };

                    if to_overwrite {
                        entry.insert(additional_entry);
                    } else if to_delete {
                        entry.remove();
                    }
                },
            }
        }
        Ok(())
    }

    pub fn squash_additional_change_set(
        &mut self,
        additional_change_set: Self,
        checker: &dyn CheckChangeSet,
    ) -> PartialVMResult<()> {
        let Self {
            resource_write_set: additional_resource_write_set,
            module_write_set: additional_module_write_set,
            aggregator_v1_write_set: additional_aggregator_write_set,
            aggregator_v1_delta_set: additional_aggregator_delta_set,
            delayed_field_change_set: additional_delayed_field_change_set,
            events: additional_events,
        } = additional_change_set;

        Self::squash_additional_aggregator_v1_changes(
            &mut self.aggregator_v1_write_set,
            &mut self.aggregator_v1_delta_set,
            additional_aggregator_write_set,
            additional_aggregator_delta_set,
        )?;
        Self::squash_additional_resource_writes(
            &mut self.resource_write_set,
            additional_resource_write_set,
        )?;
        Self::squash_additional_module_writes(
            &mut self.module_write_set,
            additional_module_write_set,
        )?;
        Self::squash_additional_delayed_field_changes(
            &mut self.delayed_field_change_set,
            additional_delayed_field_change_set,
        )?;
        self.events.extend(additional_events);

        checker.check_change_set(self)
    }

    pub fn has_creation(&self) -> bool {
        self.write_set_size_iter()
            .any(|(_key, op_size)| matches!(op_size, WriteOpSize::Creation { .. }))
    }
}

// Tests are in test_change_set.rs.
