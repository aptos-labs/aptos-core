// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::check_change_set::CheckChangeSet;
use aptos_aggregator::delta_change_set::{deserialize, serialize, DeltaChangeSet};
use aptos_state_view::StateView;
use aptos_types::{
    contract_event::ContractEvent,
    state_store::state_key::{StateKey, StateKeyInner},
    transaction::ChangeSet as StorageChangeSet,
    write_set::{WriteOp, WriteSetMut},
};
use move_binary_format::errors::Location;
use move_core_types::vm_status::{err_msg, StatusCode, VMStatus};
use std::collections::{
    btree_map,
    btree_map::{
        Entry,
        Entry::{Occupied, Vacant},
    },
    BTreeMap,
};

/// Encapsulates any changes to the state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateChange<V>(BTreeMap<StateKey, V>);

impl<V: Clone> StateChange<V> {
    pub fn empty() -> Self {
        Self(BTreeMap::new())
    }

    pub fn new(changes: impl IntoIterator<Item = (StateKey, V)>) -> Self {
        Self(changes.into_iter().collect())
    }

    #[inline]
    pub fn insert(&mut self, state_key: StateKey, value: V) {
        self.0.insert(state_key, value);
    }

    #[inline]
    pub fn remove(&mut self, state_key: &StateKey) -> Option<V> {
        self.0.remove(state_key)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn get(&self, state_key: &StateKey) -> Option<&V> {
        self.0.get(state_key)
    }

    #[inline]
    pub fn get_mut(&mut self, state_key: &StateKey) -> Option<&mut V> {
        self.0.get_mut(state_key)
    }

    #[inline]
    pub fn entry(&mut self, key: StateKey) -> Entry<StateKey, V> {
        self.0.entry(key)
    }

    #[inline]
    pub fn iter(&self) -> btree_map::Iter<'_, StateKey, V> {
        self.0.iter()
    }
}

impl<V> IntoIterator for StateChange<V> {
    type IntoIter = btree_map::IntoIter<StateKey, V>;
    type Item = (StateKey, V);

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// A change set produced by the VM. Just like VMOutput, this type should
/// be used inside the VM. For storage backends, use ChangeSet.
#[derive(Debug, Clone)]
pub struct VMChangeSet {
    // Changes to the data in the global state.
    resource_write_set: StateChange<WriteOp>,
    // Changes to the code in the global state.
    module_write_set: StateChange<WriteOp>,
    // Aggregator changes: writes and deltas.
    aggregator_write_set: StateChange<WriteOp>,
    delta_change_set: DeltaChangeSet,
    // Events produced during the execution of a transaction.
    events: Vec<ContractEvent>,
}

impl VMChangeSet {
    /// Returns an empty change set.
    pub fn empty() -> Self {
        Self {
            resource_write_set: StateChange::empty(),
            module_write_set: StateChange::empty(),
            aggregator_write_set: StateChange::empty(),
            delta_change_set: DeltaChangeSet::empty(),
            events: vec![],
        }
    }

    /// Returns a new change set, and checks that it is well-formed.
    pub fn new(
        resource_write_set: StateChange<WriteOp>,
        module_write_set: StateChange<WriteOp>,
        aggregator_write_set: StateChange<WriteOp>,
        delta_change_set: DeltaChangeSet,
        events: Vec<ContractEvent>,
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<Self, VMStatus> {
        // Check that writes and deltas have disjoint key set.
        let disjoint = delta_change_set
            .iter()
            .all(|(k, _)| aggregator_write_set.get(k).is_none());
        if !disjoint {
            return Err(VMStatus::error(
                StatusCode::DATA_FORMAT_ERROR,
                err_msg("DeltaChangeSet and WriteSet are not disjoint."),
            ));
        }

        let change_set = Self {
            resource_write_set,
            module_write_set,
            aggregator_write_set,
            delta_change_set,
            events,
        };

        // Check the newly-formed change set.
        checker.check_change_set(&change_set)?;
        Ok(change_set)
    }

    /// Returns a new change set, built from the storage representation. Note
    /// that this is an expensive operation because it re-splits resources from
    /// modules and is only used to support transactions with write set payload.
    pub fn try_from_storage_change_set(
        change_set: StorageChangeSet,
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<Self, VMStatus> {
        let (write_set, events) = change_set.into_inner();

        // There should be no aggregator writes if we have a change set from
        // storage.
        // TODO: We should also assert that there is no Aggregator V1 table item.
        let mut resource_write_set = StateChange::empty();
        let mut module_write_set = StateChange::empty();

        // Go over write ops to split them into resources and modules.
        for (state_key, write_op) in write_set {
            if matches!(state_key.inner(), StateKeyInner::AccessPath(ap) if ap.is_code()) {
                module_write_set.insert(state_key, write_op);
            } else {
                // Everything else is a resource.
                resource_write_set.insert(state_key, write_op);
            }
        }

        let change_set = Self {
            resource_write_set,
            module_write_set,
            aggregator_write_set: StateChange::empty(),
            delta_change_set: DeltaChangeSet::empty(),
            events,
        };
        checker.check_change_set(&change_set)?;
        Ok(change_set)
    }

    pub fn try_into_storage_change_set(self) -> anyhow::Result<StorageChangeSet, VMStatus> {
        let Self {
            resource_write_set,
            module_write_set,
            aggregator_write_set,
            delta_change_set,
            events,
        } = self;

        // Only change set without deltas can be converted.
        if !delta_change_set.is_empty() {
            return Err(VMStatus::error(
                // TODO: Is invariant violation a good thing here?
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                err_msg(
                    "Cannot convert to VMChangeSet with non-materialized deltas into ChangeSet",
                ),
            ));
        }

        let mut write_set_mut = WriteSetMut::default();
        write_set_mut.extend(resource_write_set);
        write_set_mut.extend(module_write_set);

        // TODO: Only aggregator V1 is a separate state item.
        write_set_mut.extend(aggregator_write_set);

        let write_set = write_set_mut.freeze().map_err(|_| {
            VMStatus::error(
                StatusCode::DATA_FORMAT_ERROR,
                err_msg("Failed to freeze write sets when converting VMChangeSet to ChangeSet"),
            )
        })?;
        Ok(StorageChangeSet::new(write_set, events))
    }

    /// A useful method to iterate over all writes.
    pub fn write_set_iter(&self) -> impl Iterator<Item = (&StateKey, &WriteOp)> {
        self.resource_write_set
            .iter()
            .chain(self.module_write_set.iter())
            // TODO: With Aggregator V2, we should not include aggregator write set.
            .chain(self.aggregator_write_set.iter())
    }

    /// Returns all changes made to data.
    pub fn resource_write_set(&self) -> &StateChange<WriteOp> {
        &self.resource_write_set
    }

    /// Returns all changes made to code.
    pub fn module_write_set(&self) -> &StateChange<WriteOp> {
        &self.module_write_set
    }

    /// Returns all changes made to aggregators.
    pub fn aggregator_write_set(&self) -> &StateChange<WriteOp> {
        &self.aggregator_write_set
    }

    /// Returns all the pending changes (i.e. deltas) made to aggregators
    /// and not yet applied.
    pub fn delta_change_set(&self) -> &DeltaChangeSet {
        &self.delta_change_set
    }

    /// Returns all events emitted.
    pub fn events(&self) -> &[ContractEvent] {
        &self.events
    }

    /// Materializes this change set: all deltas are converted into writes and
    /// are combined with existing write set. In case of materialization fails,
    /// an error is returned.
    pub fn try_materialize(self, state_view: &impl StateView) -> anyhow::Result<Self, VMStatus> {
        let Self {
            resource_write_set,
            module_write_set,
            mut aggregator_write_set,
            delta_change_set,
            events,
        } = self;

        // Try to materialize deltas and add them to the write set.
        let delta_writes = delta_change_set.try_materialize(state_view)?;
        delta_writes
            .into_iter()
            .for_each(|(state_key, write_op)| aggregator_write_set.insert(state_key, write_op));

        Ok(Self {
            resource_write_set,
            module_write_set,
            aggregator_write_set,
            delta_change_set: DeltaChangeSet::empty(),
            events,
        })
    }

    /// Squashes `next` change set on top of this change set. The squashed
    /// change set is then checked using the `checker`.
    pub fn squash(
        self,
        next: Self,
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<Self, VMStatus> {
        use WriteOp::*;

        // First, obtain write sets, delta change sets and events of this and other
        // change sets.
        let Self {
            resource_write_set: next_resource_write_set,
            module_write_set: next_module_write_set,
            aggregator_write_set: next_aggregator_write_set,
            delta_change_set: next_delta_change_set,
            events: next_events,
        } = next;
        let Self {
            mut resource_write_set,
            mut module_write_set,
            mut aggregator_write_set,
            mut delta_change_set,
            mut events,
        } = self;

        // We are modifying current sets, so grab a mutable reference for them.
        let delta_ops = delta_change_set.as_inner_mut();

        // First, squash incoming deltas.
        for (key, next_delta_op) in next_delta_change_set.into_iter() {
            if let Some(write_op) = aggregator_write_set.get_mut(&key) {
                // In this case, delta follows a write op.
                match write_op {
                    Creation(data)
                    | Modification(data)
                    | CreationWithMetadata { data, .. }
                    | ModificationWithMetadata { data, .. } => {
                        // Apply delta on top of creation or modification.
                        let base: u128 = deserialize(data);
                        let value = next_delta_op
                            .apply_to(base)
                            .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
                        *data = serialize(&value);
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
                match delta_ops.entry(key) {
                    Occupied(entry) => {
                        // In this case, we need to merge the new incoming delta
                        // to the existing delta, ensuring the strict ordering.
                        entry
                            .into_mut()
                            .merge_with_next_delta(next_delta_op)
                            .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
                    },
                    Vacant(entry) => {
                        // We see this delta for the first time, so simply add it
                        // to the set.
                        entry.insert(next_delta_op);
                    },
                }
            }
        }

        // Next, squash write ops.
        for (key, next_write_op) in next_aggregator_write_set.into_iter() {
            match aggregator_write_set.entry(key) {
                Occupied(mut entry) => {
                    // Squashing creation and deletion is a no-op. In that case, we
                    // have to remove the old write op from the write set.
                    let noop = !WriteOp::squash(entry.get_mut(), next_write_op).map_err(|e| {
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
                    // This is a new write op. It can overwrite a delta so we
                    // have to make sure we remove such a delta from the set in
                    // this case.
                    let removed_delta = delta_change_set.remove(entry.key());

                    // We cannot create after modification with a delta!
                    if removed_delta.is_some() && next_write_op.is_creation() {
                        return Err(VMStatus::error(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                            err_msg("Cannot create a resource after modification with a delta."),
                        ));
                    }

                    entry.insert(next_write_op);
                },
            }
        }
        for (key, next_write_op) in next_resource_write_set.into_iter() {
            match resource_write_set.entry(key) {
                Occupied(mut entry) => {
                    // Squashing creation and deletion is a no-op. In that case, we
                    // have to remove the old write op from the write set.
                    let noop = !WriteOp::squash(entry.get_mut(), next_write_op).map_err(|e| {
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
                    entry.insert(next_write_op);
                },
            }
        }
        for (key, next_write_op) in next_module_write_set.into_iter() {
            match module_write_set.entry(key) {
                Occupied(mut entry) => {
                    // Squashing creation and deletion is a no-op. In that case, we
                    // have to remove the old write op from the write set.
                    let noop = !WriteOp::squash(entry.get_mut(), next_write_op).map_err(|e| {
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
                    entry.insert(next_write_op);
                },
            }
        }

        // Squash events.
        events.extend(next_events);

        Self::new(
            resource_write_set,
            module_write_set,
            aggregator_write_set,
            delta_change_set,
            events,
            checker,
        )
    }
}
