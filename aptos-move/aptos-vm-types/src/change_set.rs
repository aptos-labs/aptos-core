// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::check_change_set::CheckChangeSet;
use aptos_aggregator::{
    delayed_change::DelayedChange,
    delta_change_set::{serialize, DeltaOp},
    resolver::AggregatorV1Resolver,
    types::{code_invariant_error, DelayedFieldID},
};
use aptos_types::{
    contract_event::ContractEvent,
    state_store::state_key::{StateKey, StateKeyInner},
    transaction::ChangeSet as StorageChangeSet,
    write_set::{TransactionWrite, WriteOp, WriteSetMut},
};
use claims::assert_none;
use move_binary_format::errors::{Location, PartialVMError};
use move_core_types::{
    language_storage::StructTag,
    value::MoveTypeLayout,
    vm_status::{err_msg, StatusCode, VMStatus},
};
use std::{
    collections::{
        btree_map::Entry::{Occupied, Vacant},
        BTreeMap,
    },
    hash::Hash,
    sync::Arc,
};

#[derive(PartialEq, Eq, Clone, Debug)]
/// Describes an update to a resource group granularly, with WriteOps to affected
/// member resources of the group, as well as a separate WriteOp for metadata and size.
pub struct GroupWrite {
    /// Op of the correct kind (creation / modification / deletion) and metadata, and
    /// the size of the group after the updates encoded in the bytes (no bytes for
    /// deletion). Relevant during block execution, where the information read to
    /// derive metadata_op will be validated during parallel execution to make sure
    /// it is correct, and the bytes will be replaced after the transaction is committed
    /// with correct serialized group update to obtain storage WriteOp.
    metadata_op: WriteOp,
    /// Updates to individual group members. WriteOps are 'legacy', i.e. no metadata.
    /// If the metadata_op is a deletion, all (correct) inner_ops should be deletions,
    /// and if metadata_op is a creation, then there may not be a creation inner op.
    /// Not vice versa, e.g. for deleted inner ops, other untouched resources may still
    /// exist in the group. Note: During parallel block execution, due to speculative
    /// reads, this invariant may be violated (and lead to speculation error if observed)
    /// but guaranteed to fail validation and lead to correct re-execution in that case.
    inner_ops: BTreeMap<StructTag, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
}

impl GroupWrite {
    /// Creates a group write and ensures that the format is correct: in particular,
    /// sets the bytes of a non-deletion metadata_op by serializing the provided size,
    /// and ensures inner ops do not contain any metadata.
    pub fn new(
        mut metadata_op: WriteOp,
        group_size: u64,
        inner_ops: BTreeMap<StructTag, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
    ) -> Self {
        for (v, _layout) in inner_ops.values() {
            assert_none!(v.metadata());
        }

        let encoded_group_size = bcs::to_bytes(&group_size)
            .expect("Must serialize u64 successfully")
            .into();
        metadata_op.set_bytes(encoded_group_size);

        Self {
            metadata_op,
            inner_ops,
        }
    }

    /// Utility method that extracts the serialized group size from metadata_op. Returns
    /// None if group is being deleted, otherwise asserts on deserializing the size.
    pub fn encoded_group_size(&self) -> Option<u64> {
        self.metadata_op
            .bytes()
            .map(|b| bcs::from_bytes::<u64>(b).expect("Must be serialized group size"))
    }

    // TODO: refactor storage fee & refund interfaces to operate on metadata directly,
    // as that would avoid providing &mut to the whole metadata op in here, including
    // bytes that are not raw bytes (encoding group size) and must not be modified.
    pub fn metadata_op_mut(&mut self) -> &mut WriteOp {
        &mut self.metadata_op
    }

    pub fn metadata_op(&self) -> &WriteOp {
        &self.metadata_op
    }

    pub fn inner_ops(&self) -> &BTreeMap<StructTag, (WriteOp, Option<Arc<MoveTypeLayout>>)> {
        &self.inner_ops
    }
}

/// A change set produced by the VM.
///
/// **WARNING**: Just like VMOutput, this type should only be used inside the
/// VM. For storage backends, use `ChangeSet`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VMChangeSet {
    resource_write_set: BTreeMap<StateKey, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
    // Prior to adding a dedicated write-set for resource groups, all resource group
    // updates are merged into a single WriteOp included in the resource_write_set.
    resource_group_write_set: BTreeMap<StateKey, GroupWrite>,
    module_write_set: BTreeMap<StateKey, WriteOp>,
    aggregator_v1_write_set: BTreeMap<StateKey, WriteOp>,
    aggregator_v1_delta_set: BTreeMap<StateKey, DeltaOp>,
    delayed_field_change_set: BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
    events: Vec<(ContractEvent, Option<MoveTypeLayout>)>,
}

macro_rules! squash_writes_pair {
    ($write_entry:ident, $additional_write:ident) => {
        // Squashing creation and deletion is a no-op. In that case, we
        // have to remove the old write op from the write set.
        let noop = !WriteOp::squash($write_entry.get_mut(), $additional_write).map_err(|e| {
            VMStatus::error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                err_msg(format!("Error while squashing two write ops: {}.", e)),
            )
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
            resource_group_write_set: BTreeMap::new(),
            module_write_set: BTreeMap::new(),
            aggregator_v1_write_set: BTreeMap::new(),
            aggregator_v1_delta_set: BTreeMap::new(),
            delayed_field_change_set: BTreeMap::new(),
            events: vec![],
        }
    }

    pub fn new(
        resource_write_set: BTreeMap<StateKey, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
        resource_group_write_set: BTreeMap<StateKey, GroupWrite>,
        module_write_set: BTreeMap<StateKey, WriteOp>,
        aggregator_v1_write_set: BTreeMap<StateKey, WriteOp>,
        aggregator_v1_delta_set: BTreeMap<StateKey, DeltaOp>,
        delayed_field_change_set: BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
        events: Vec<(ContractEvent, Option<MoveTypeLayout>)>,
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<Self, VMStatus> {
        let change_set = Self {
            resource_write_set,
            resource_group_write_set,
            module_write_set,
            aggregator_v1_write_set,
            aggregator_v1_delta_set,
            delayed_field_change_set,
            events,
        };

        // Returns an error if structure of the change set is not valid,
        // e.g. the size in bytes is too large.
        checker.check_change_set(&change_set)?;
        Ok(change_set)
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
        is_delayed_field_optimization_enabled: bool,
    ) -> anyhow::Result<Self, VMStatus> {
        assert!(
            !is_delayed_field_optimization_enabled,
            "try_from_storage_change_set can only be called in non-is_delayed_field_optimization_enabled context, as it doesn't support delayed field changes (type layout) and resource groups");

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

                // We can set layout to None, as we are not in the is_delayed_field_optimization_enabled context
                resource_write_set.insert(state_key, (write_op, None));
            }
        }

        // We can set layout to None, as we are not in the is_delayed_field_optimization_enabled context
        let events = events.into_iter().map(|event| (event, None)).collect();
        let change_set = Self {
            resource_write_set,
            // TODO[agg_v2](fix): do we use same or different capable flag for resource groups?
            // We should skip unpacking resource groups, as we are not in the is_delayed_field_optimization_enabled
            // context (i.e. if dynamic_change_set_optimizations_enabled is disabled)
            resource_group_write_set: BTreeMap::new(),
            module_write_set,
            aggregator_v1_write_set: BTreeMap::new(),
            aggregator_v1_delta_set: BTreeMap::new(),
            delayed_field_change_set: BTreeMap::new(),
            events,
        };
        checker.check_change_set(&change_set)?;
        Ok(change_set)
    }

    pub(crate) fn into_storage_change_set_unchecked(self) -> StorageChangeSet {
        // Converting VMChangeSet into TransactionOutput (i.e. storage change set), can
        // be done here only if dynamic_change_set_optimizations have not been used/produced
        // data into the output.
        // If they (DelayedField or ResourceGroup) have added data into the write set, translation
        // into output is more complicated, and needs to be done within BlockExecutor context
        // that knows how to deal with it.
        assert!(self.delayed_field_change_set().is_empty());
        assert!(self.resource_group_write_set().is_empty());

        let Self {
            resource_write_set,
            resource_group_write_set: _,
            module_write_set,
            aggregator_v1_write_set,
            aggregator_v1_delta_set: _,
            delayed_field_change_set: _,
            events,
        } = self;

        let mut write_set_mut = WriteSetMut::default();
        write_set_mut.extend(resource_write_set.into_iter().map(|(k, (v, _))| (k, v)));
        write_set_mut.extend(module_write_set);
        write_set_mut.extend(aggregator_v1_write_set);

        let events = events.into_iter().map(|(e, _)| e).collect();
        let write_set = write_set_mut
            .freeze()
            .expect("Freezing a WriteSet does not fail.");
        StorageChangeSet::new(write_set, events)
    }

    /// Converts VM-native change set into its storage representation with fully
    /// serialized changes. The conversion fails if:
    /// - deltas are not materialized.
    /// - resource group writes are not (combined &) converted to resource writes.
    pub fn try_into_storage_change_set(self) -> anyhow::Result<StorageChangeSet, VMStatus> {
        if !self.aggregator_v1_delta_set.is_empty() {
            return Err(VMStatus::error(
                StatusCode::DATA_FORMAT_ERROR,
                err_msg(
                    "Cannot convert from VMChangeSet with non-materialized Aggregator V1 deltas to ChangeSet.",
                ),
            ));
        }
        if !self.delayed_field_change_set.is_empty() {
            return Err(VMStatus::error(
                StatusCode::DATA_FORMAT_ERROR,
                err_msg(
                    "Cannot convert from VMChangeSet with non-materialized Delayed Field changes to ChangeSet.",
                ),
            ));
        }
        if !self.resource_group_write_set.is_empty() {
            return Err(VMStatus::error(
                StatusCode::DATA_FORMAT_ERROR,
                err_msg(
                    "Cannot convert from VMChangeSet with non-combined resource group changes.",
                ),
            ));
        }
        Ok(self.into_storage_change_set_unchecked())
    }

    pub fn write_set_iter(&self) -> impl Iterator<Item = (&StateKey, &WriteOp)> {
        self.resource_write_set()
            .iter()
            .map(|(k, (v, _))| (k, v))
            .chain(self.module_write_set().iter())
            .chain(self.aggregator_v1_write_set().iter())
    }

    pub fn num_write_ops(&self) -> usize {
        self.resource_write_set().len()
            + self.module_write_set().len()
            + self.aggregator_v1_write_set().len()
    }

    pub fn write_set_iter_mut(&mut self) -> impl Iterator<Item = (&StateKey, &mut WriteOp)> {
        self.resource_write_set
            .iter_mut()
            .map(|(k, (v, _))| (k, v))
            .chain(self.module_write_set.iter_mut())
            .chain(self.aggregator_v1_write_set.iter_mut())
    }

    pub fn group_write_set_iter_mut(
        &mut self,
    ) -> impl Iterator<Item = (&StateKey, &mut GroupWrite)> {
        self.resource_group_write_set.iter_mut()
    }

    pub fn resource_write_set(
        &self,
    ) -> &BTreeMap<StateKey, (WriteOp, Option<Arc<MoveTypeLayout>>)> {
        &self.resource_write_set
    }

    pub fn resource_group_write_set(&self) -> &BTreeMap<StateKey, GroupWrite> {
        &self.resource_group_write_set
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
        finalized_group_writes: impl Iterator<Item = (StateKey, WriteOp)>,
    ) {
        self.resource_write_set.extend(
            patched_resource_writes
                .chain(finalized_group_writes)
                .map(|(k, v)| (k, (v, None))),
        );
    }

    /// The events are set to the input events.
    pub(crate) fn set_events(&mut self, patched_events: impl Iterator<Item = ContractEvent>) {
        self.events = patched_events
            .map(|event| (event, None))
            .collect::<Vec<_>>();
    }

    pub(crate) fn drain_delayed_field_change_set(
        &mut self,
    ) -> impl Iterator<Item = (DelayedFieldID, DelayedChange<DelayedFieldID>)> + '_ {
        std::mem::take(&mut self.delayed_field_change_set).into_iter()
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
        self,
        resolver: &impl AggregatorV1Resolver,
    ) -> anyhow::Result<Self, VMStatus> {
        let Self {
            resource_write_set,
            resource_group_write_set,
            module_write_set,
            mut aggregator_v1_write_set,
            aggregator_v1_delta_set,
            delayed_field_change_set,
            events,
        } = self;

        let into_write =
            |(state_key, delta): (StateKey, DeltaOp)| -> anyhow::Result<(StateKey, WriteOp), VMStatus> {
                // Materialization is needed when committing a transaction, so
                // we need precise mode to compute the true value of an
                // aggregator.
                let write = resolver.try_convert_aggregator_v1_delta_into_write_op(&state_key, &delta)?;
                Ok((state_key, write))
            };

        let materialized_aggregator_delta_set = aggregator_v1_delta_set
            .into_iter()
            .map(into_write)
            .collect::<anyhow::Result<BTreeMap<StateKey, WriteOp>, VMStatus>>()?;
        aggregator_v1_write_set.extend(materialized_aggregator_delta_set);

        Ok(Self {
            resource_write_set,
            resource_group_write_set,
            module_write_set,
            aggregator_v1_write_set,
            aggregator_v1_delta_set: BTreeMap::new(),
            delayed_field_change_set,
            events,
        })
    }

    fn squash_additional_aggregator_v1_changes(
        aggregator_v1_write_set: &mut BTreeMap<StateKey, WriteOp>,
        aggregator_v1_delta_set: &mut BTreeMap<StateKey, DeltaOp>,
        additional_aggregator_v1_write_set: BTreeMap<StateKey, WriteOp>,
        additional_aggregator_v1_delta_set: BTreeMap<StateKey, DeltaOp>,
    ) -> anyhow::Result<(), VMStatus> {
        use WriteOp::*;

        // First, squash deltas.
        for (state_key, additional_delta_op) in additional_aggregator_v1_delta_set {
            if let Some(write_op) = aggregator_v1_write_set.get_mut(&state_key) {
                // In this case, delta follows a write op.
                match write_op {
                    Creation(data)
                    | Modification(data)
                    | CreationWithMetadata { data, .. }
                    | ModificationWithMetadata { data, .. } => {
                        // Apply delta on top of creation or modification.
                        // TODO[agg_v1](cleanup): This will not be needed anymore once aggregator
                        // change sets carry non-serialized information.
                        let base: u128 = bcs::from_bytes(data)
                            .expect("Deserializing into an aggregator value always succeeds");
                        let value = additional_delta_op
                            .apply_to(base)
                            .map_err(PartialVMError::from)
                            .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
                        *data = serialize(&value).into();
                    },
                    Deletion | DeletionWithMetadata { .. } => {
                        // This case (applying a delta to deleted item) should
                        // never happen. Let's still return an error instead of
                        // panicking.
                        return Err(VMStatus::error(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                            err_msg("Cannot squash delta which was already deleted."),
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
                            .map_err(PartialVMError::from)
                            .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
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
                        return Err(VMStatus::error(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                            err_msg("Cannot create a resource after modification with a delta."),
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
    ) -> anyhow::Result<(), VMStatus> {
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
            change_set.insert(
                id,
                merged_change
                    .map_err(PartialVMError::from)
                    .map_err(|e| e.finish(Location::Undefined).into_vm_status())?,
            );
        }
        Ok(())
    }

    fn squash_additional_module_writes(
        write_set: &mut BTreeMap<StateKey, WriteOp>,
        additional_write_set: BTreeMap<StateKey, WriteOp>,
    ) -> anyhow::Result<(), VMStatus> {
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

    fn squash_group_writes(
        write_set: &mut BTreeMap<StateKey, GroupWrite>,
        additional_write_set: BTreeMap<StateKey, GroupWrite>,
    ) -> anyhow::Result<(), VMStatus> {
        for (key, additional_update) in additional_write_set.into_iter() {
            match write_set.entry(key) {
                Occupied(mut group_entry) => {
                    let GroupWrite {
                        metadata_op: additional_metadata_op,
                        inner_ops: additional_inner_ops,
                    } = additional_update;

                    // Squashing creation and deletion is a no-op. In that case, we have to
                    // remove the old GroupWrite from the group write set.
                    let noop = !WriteOp::squash(
                        &mut group_entry.get_mut().metadata_op,
                        additional_metadata_op,
                    )
                    .map_err(|e| {
                        VMStatus::error(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                            err_msg(format!(
                                "Error while squashing two group write metadata ops: {}.",
                                e
                            )),
                        )
                    })?;
                    if noop {
                        group_entry.remove();
                    } else {
                        Self::squash_additional_resource_writes(
                            &mut group_entry.get_mut().inner_ops,
                            additional_inner_ops,
                        )?;
                    }
                },
                Vacant(entry) => {
                    entry.insert(additional_update);
                },
            }
        }
        Ok(())
    }

    fn squash_additional_resource_writes<
        K: Hash + Eq + PartialEq + Ord + Clone + std::fmt::Debug,
    >(
        write_set: &mut BTreeMap<K, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
        additional_write_set: BTreeMap<K, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
    ) -> anyhow::Result<(), VMStatus> {
        for (key, additional_entry) in additional_write_set.into_iter() {
            match write_set.entry(key.clone()) {
                Occupied(mut entry) => {
                    // Squash entry and addtional entries if type layouts match
                    let (additional_write_op, additional_type_layout) = additional_entry;
                    let (write_op, type_layout) = entry.get_mut();
                    if *type_layout != additional_type_layout {
                        return Err(VMStatus::error(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                            err_msg(format!(
                                "Cannot squash two writes with different type layouts.
                                key: {:?}, type_layout: {:?}, additional_type_layout: {:?}",
                                key, type_layout, additional_type_layout
                            )),
                        ));
                    }
                    let noop = !WriteOp::squash(write_op, additional_write_op).map_err(|e| {
                        VMStatus::error(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                            err_msg(format!("Error while squashing two write ops: {}.", e)),
                        )
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

    pub fn squash_additional_change_set(
        &mut self,
        additional_change_set: Self,
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<(), VMStatus> {
        let Self {
            resource_write_set: additional_resource_write_set,
            resource_group_write_set: additional_resource_group_write_set,
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
        Self::squash_additional_delayed_field_changes(
            &mut self.delayed_field_change_set,
            additional_delayed_field_change_set,
        )?;
        Self::squash_additional_resource_writes(
            &mut self.resource_write_set,
            additional_resource_write_set,
        )?;
        Self::squash_group_writes(
            &mut self.resource_group_write_set,
            additional_resource_group_write_set,
        )?;
        Self::squash_additional_module_writes(
            &mut self.module_write_set,
            additional_module_write_set,
        )?;
        self.events.extend(additional_events);

        checker.check_change_set(self)
    }

    pub fn has_creation(&self) -> bool {
        use WriteOp::*;
        self.write_set_iter()
            .any(|(_key, op)| matches!(op, Creation(..) | CreationWithMetadata { .. }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::utils::{
        mock_tag_0, mock_tag_1, mock_tag_2, raw_metadata, write_op_with_metadata,
    };
    use bytes::Bytes;
    use claims::{assert_err, assert_ok, assert_some_eq};
    use test_case::test_case;

    macro_rules! assert_group_write_size {
        ($op:expr, $s:expr, $exp:expr) => {{
            let group_write = GroupWrite::new($op, $s, BTreeMap::new());
            assert_eq!(group_write.encoded_group_size(), $exp);
        }};
    }

    #[test]
    fn test_group_write_size() {
        // Deletions should lead to size 0.
        assert_group_write_size!(WriteOp::Deletion, 0, None);
        assert_group_write_size!(
            WriteOp::DeletionWithMetadata {
                metadata: raw_metadata(10)
            },
            0,
            None
        );

        let sizes = [20, 100, 45279432, 5];
        assert_group_write_size!(WriteOp::Creation(Bytes::new()), sizes[0], Some(sizes[0]));
        assert_group_write_size!(
            WriteOp::CreationWithMetadata {
                data: Bytes::new(),
                metadata: raw_metadata(20)
            },
            sizes[1],
            Some(sizes[1])
        );
        assert_group_write_size!(
            WriteOp::Modification(Bytes::new()),
            sizes[2],
            Some(sizes[2])
        );
        assert_group_write_size!(
            WriteOp::ModificationWithMetadata {
                data: Bytes::new(),
                metadata: raw_metadata(30)
            },
            sizes[3],
            Some(sizes[3])
        );
    }

    #[test]
    fn test_squash_groups_one_empty() {
        let key_1 = StateKey::raw(vec![1]);
        let key_2 = StateKey::raw(vec![2]);

        let mut base_update = BTreeMap::new();
        base_update.insert(key_1.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(0, 100),
            inner_ops: BTreeMap::new(),
        });
        let mut additional_update = BTreeMap::new();
        additional_update.insert(key_2.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(0, 200),
            inner_ops: BTreeMap::new(),
        });

        assert_ok!(VMChangeSet::squash_group_writes(
            &mut base_update,
            additional_update
        ));

        assert_eq!(base_update.len(), 2);
        assert_some_eq!(
            base_update.get(&key_1).unwrap().metadata_op.metadata(),
            &raw_metadata(100)
        );
        assert_some_eq!(
            base_update.get(&key_2).unwrap().metadata_op.metadata(),
            &raw_metadata(200)
        );
    }

    #[test_case(0, 1)] // create, modify
    #[test_case(1, 1)] // modify, modify
    #[test_case(1, 2)] // modify, delete
    #[test_case(2, 0)] // delete, create
    fn test_squash_groups_mergeable_metadata(base_type_idx: u8, additional_type_idx: u8) {
        let key = StateKey::raw(vec![0]);

        let mut base_update = BTreeMap::new();
        let mut additional_update = BTreeMap::new();
        base_update.insert(key.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(base_type_idx, 100),
            inner_ops: BTreeMap::new(),
        });
        additional_update.insert(key.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(additional_type_idx, 200),
            inner_ops: BTreeMap::new(),
        });

        assert_ok!(VMChangeSet::squash_group_writes(
            &mut base_update,
            additional_update
        ));

        assert_eq!(base_update.len(), 1);
        assert_some_eq!(
            base_update.get(&key).unwrap().metadata_op.metadata(),
            // take the original metadata
            &raw_metadata(100)
        );
    }

    #[test_case(0, 0)] // create, create
    #[test_case(1, 0)] // modify, create
    #[test_case(2, 1)] // delete, modify
    #[test_case(2, 2)] // delete, delete
    fn test_squash_groups_error(base_type_idx: u8, additional_type_idx: u8) {
        let key = StateKey::raw(vec![0]);

        let mut base_update = BTreeMap::new();
        let mut additional_update = BTreeMap::new();
        base_update.insert(key.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(base_type_idx, 100),
            inner_ops: BTreeMap::new(),
        });
        additional_update.insert(key.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(additional_type_idx, 200),
            inner_ops: BTreeMap::new(),
        });

        assert_err!(VMChangeSet::squash_group_writes(
            &mut base_update,
            additional_update
        ));
    }

    #[test]
    fn test_squash_groups_noop() {
        let key = StateKey::raw(vec![0]);

        let mut base_update = BTreeMap::new();
        let mut additional_update = BTreeMap::new();
        base_update.insert(key.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(0, 100), // create
            inner_ops: BTreeMap::new(),
        });
        additional_update.insert(key.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(2, 200), // delete
            inner_ops: BTreeMap::new(),
        });

        assert_ok!(VMChangeSet::squash_group_writes(
            &mut base_update,
            additional_update
        ));
        assert!(base_update.is_empty(), "Must become a no-op");
    }

    #[test]
    fn test_inner_ops() {
        let key_1 = StateKey::raw(vec![1]);
        let key_2 = StateKey::raw(vec![2]);

        let mut base_update = BTreeMap::new();
        let mut additional_update = BTreeMap::new();
        // TODO: Harcoding type layout to None. Test with layout = Some(..)
        base_update.insert(key_1.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(1, 100),
            inner_ops: BTreeMap::from([
                (mock_tag_0(), (WriteOp::Creation(vec![100].into()), None)),
                (mock_tag_2(), (WriteOp::Modification(vec![2].into()), None)),
            ]),
        });
        additional_update.insert(key_1.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(1, 200),
            inner_ops: BTreeMap::from([
                (mock_tag_0(), (WriteOp::Modification(vec![0].into()), None)),
                (mock_tag_1(), (WriteOp::Modification(vec![1].into()), None)),
            ]),
        });

        base_update.insert(key_2.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(1, 100),
            inner_ops: BTreeMap::from([
                (mock_tag_0(), (WriteOp::Deletion, None)),
                (mock_tag_1(), (WriteOp::Modification(vec![2].into()), None)),
                (mock_tag_2(), (WriteOp::Creation(vec![2].into()), None)),
            ]),
        });
        additional_update.insert(key_2.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(1, 200),
            inner_ops: BTreeMap::from([
                (mock_tag_0(), (WriteOp::Creation(vec![0].into()), None)),
                (mock_tag_1(), (WriteOp::Deletion, None)),
                (mock_tag_2(), (WriteOp::Deletion, None)),
            ]),
        });

        assert_ok!(VMChangeSet::squash_group_writes(
            &mut base_update,
            additional_update
        ));
        assert_eq!(base_update.len(), 2);
        let inner_ops_1 = &base_update.get(&key_1).unwrap().inner_ops;
        assert_eq!(inner_ops_1.len(), 3);
        assert_some_eq!(
            inner_ops_1.get(&mock_tag_0()),
            &(WriteOp::Creation(vec![0].into()), None)
        );
        assert_some_eq!(
            inner_ops_1.get(&mock_tag_1()),
            &(WriteOp::Modification(vec![1].into()), None)
        );
        assert_some_eq!(
            inner_ops_1.get(&mock_tag_2()),
            &(WriteOp::Modification(vec![2].into()), None)
        );
        let inner_ops_2 = &base_update.get(&key_2).unwrap().inner_ops;
        assert_eq!(inner_ops_2.len(), 2);
        assert_some_eq!(
            inner_ops_2.get(&mock_tag_0()),
            &(WriteOp::Modification(vec![0].into()), None)
        );
        assert_some_eq!(inner_ops_2.get(&mock_tag_1()), &(WriteOp::Deletion, None));

        let additional_update = BTreeMap::from([(key_2.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(1, 200),
            inner_ops: BTreeMap::from([(mock_tag_1(), (WriteOp::Deletion, None))]),
        })]);
        assert_err!(VMChangeSet::squash_group_writes(
            &mut base_update,
            additional_update
        ));
    }
}
