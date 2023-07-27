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
    btree_map::Entry::{Occupied, Vacant},
    BTreeMap,
};

/// A change set produced by the VM. Just like VMOutput, this type should
/// be used inside the VM. For storage backends, use ChangeSet.
#[derive(Debug, Clone)]
pub struct VMChangeSet {
    // Changes to the data in the global state.
    resource_write_set: BTreeMap<StateKey, WriteOp>,
    // Changes to the code in the global state.
    module_write_set: BTreeMap<StateKey, WriteOp>,
    // Aggregator changes: writes and deltas.
    aggregator_write_set: BTreeMap<StateKey, WriteOp>,
    delta_change_set: DeltaChangeSet,
    // Events produced during the execution of a transaction.
    events: Vec<ContractEvent>,
}

// Useful macro to squash a pair of write ops and ignore the result if
// it is a no-op.
macro_rules! squash_write_ops {
    ($entry:ident, $next_write_op:ident) => {
        // Squashing creation and deletion is a no-op. In that case, we
        // have to remove the old write op from the write set.
        let noop = !WriteOp::squash($entry.get_mut(), $next_write_op).map_err(|e| {
            VMStatus::error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                err_msg(format!("Error while squashing two write ops: {}.", e)),
            )
        })?;
        if noop {
            $entry.remove();
        }
    };
}

impl VMChangeSet {
    /// Returns an empty change set.
    pub fn empty() -> Self {
        Self {
            resource_write_set: BTreeMap::new(),
            module_write_set: BTreeMap::new(),
            aggregator_write_set: BTreeMap::new(),
            delta_change_set: DeltaChangeSet::empty(),
            events: vec![],
        }
    }

    /// Returns a new change set, and checks that it is well-formed.
    pub fn new(
        resource_write_set: BTreeMap<StateKey, WriteOp>,
        module_write_set: BTreeMap<StateKey, WriteOp>,
        aggregator_write_set: BTreeMap<StateKey, WriteOp>,
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
                err_msg("Aggregator delta and write sets are not disjoint."),
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

    /// Builds a new change set from the storage representation.
    ///
    /// WARNING: Has complexity O(#write_ops) because we need to
    /// iterate over blobs and split them into resources or modules.
    /// Only used to support transactions with write-set payload.
    pub fn try_from_storage_change_set(
        change_set: StorageChangeSet,
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<Self, VMStatus> {
        let (write_set, events) = change_set.into_inner();

        // There should be no aggregator writes if we have a change set from
        // storage.
        let mut resource_write_set = BTreeMap::new();
        let mut module_write_set = BTreeMap::new();

        // Go over write ops to split them into resources and modules.
        for (state_key, write_op) in write_set {
            if matches!(state_key.inner(), StateKeyInner::AccessPath(ap) if ap.is_code()) {
                module_write_set.insert(state_key, write_op);
            } else {
                // Everything else must be a resource.
                // TODO(aggregator): Aggregator V1 is a table item, and so fits into
                // this category. In practice this should never happen, but we might
                // want to have an assert here before Aggregator V2 lands.
                resource_write_set.insert(state_key, write_op);
            }
        }

        let change_set = Self {
            resource_write_set,
            module_write_set,
            aggregator_write_set: BTreeMap::new(),
            delta_change_set: DeltaChangeSet::empty(),
            events,
        };
        checker.check_change_set(&change_set)?;
        Ok(change_set)
    }

    /// Converts VM-native change set into its storage representation with fully
    /// serialized changes.
    /// At this points, if deltas are not materialized the conversion fails.
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
                StatusCode::DATA_FORMAT_ERROR,
                err_msg(
                    "Cannot convert from VMChangeSet with non-materialized deltas to ChangeSet.",
                ),
            ));
        }

        // Create a write set which has all the changes combined together.
        let mut write_set_mut = WriteSetMut::default();
        write_set_mut.extend(resource_write_set);
        write_set_mut.extend(module_write_set);

        // TODO(aggregator): Aggregator V1 is a state item, and has to be added
        // here.
        // When we move to different key representation for aggregators,
        // this has to be conditional on the table item (supply) until we
        // fully transition to the new version.
        write_set_mut.extend(aggregator_write_set);

        let write_set = write_set_mut.freeze().map_err(|_| {
            VMStatus::error(
                StatusCode::DATA_FORMAT_ERROR,
                err_msg("Failed to freeze write sets when converting VMChangeSet to ChangeSet"),
            )
        })?;
        Ok(StorageChangeSet::new(write_set, events))
    }

    // TODO(aggregator): With Aggregator V2, we would have to revisit this
    // because we no longer iterate over state items.
    pub fn write_set_iter(&self) -> impl Iterator<Item = (&StateKey, &WriteOp)> {
        self.resource_write_set
            .iter()
            .chain(self.module_write_set.iter())
            // TODO: With Aggregator V2, we should not include aggregator write set.
            .chain(self.aggregator_write_set.iter())
    }

    pub fn resource_write_set(&self) -> &BTreeMap<StateKey, WriteOp> {
        &self.resource_write_set
    }

    pub fn module_write_set(&self) -> &BTreeMap<StateKey, WriteOp> {
        &self.module_write_set
    }

    pub fn aggregator_write_set(&self) -> &BTreeMap<StateKey, WriteOp> {
        &self.aggregator_write_set
    }

    pub fn delta_change_set(&self) -> &DeltaChangeSet {
        &self.delta_change_set
    }

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
        delta_writes.into_iter().for_each(|(state_key, write_op)| {
            assert!(aggregator_write_set.insert(state_key, write_op).is_none())
        });

        Ok(Self {
            resource_write_set,
            module_write_set,
            aggregator_write_set,
            delta_change_set: DeltaChangeSet::empty(),
            events,
        })
    }

    fn squash_additional_aggregator_changes(
        delta_change_set: &mut DeltaChangeSet,
        aggregator_write_set: &mut BTreeMap<StateKey, WriteOp>,
        additional_delta_change_set: DeltaChangeSet,
        additional_aggregator_write_set: BTreeMap<StateKey, WriteOp>,
    ) -> anyhow::Result<(), VMStatus> {
        use WriteOp::*;

        // DeltaChangeSet has to be grabbed as mutable.
        let delta_ops = delta_change_set.as_inner_mut();

        // First, squash deltas.
        for (key, additional_delta_op) in additional_delta_change_set.into_iter() {
            if let Some(write_op) = aggregator_write_set.get_mut(&key) {
                // In this case, delta follows a write op.
                match write_op {
                    Creation(data)
                    | Modification(data)
                    | CreationWithMetadata { data, .. }
                    | ModificationWithMetadata { data, .. } => {
                        // Apply delta on top of creation or modification.
                        let base: u128 = deserialize(data);
                        let value = additional_delta_op
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
                            .merge_with_next_delta(additional_delta_op)
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
        for (key, additional_write_op) in additional_aggregator_write_set.into_iter() {
            match aggregator_write_set.entry(key) {
                Occupied(mut entry) => {
                    squash_write_ops!(entry, additional_write_op);
                },
                Vacant(entry) => {
                    // This is a new write op. It can overwrite a delta so we
                    // have to make sure we remove such a delta from the set in
                    // this case.
                    let removed_delta = delta_change_set.remove(entry.key());

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

    fn squash_additional_writes(
        write_set: &mut BTreeMap<StateKey, WriteOp>,
        additional_write_set: BTreeMap<StateKey, WriteOp>,
    ) -> anyhow::Result<(), VMStatus> {
        for (key, additional_write_op) in additional_write_set.into_iter() {
            match write_set.entry(key) {
                Occupied(mut entry) => {
                    squash_write_ops!(entry, additional_write_op);
                },
                Vacant(entry) => {
                    entry.insert(additional_write_op);
                },
            }
        }
        Ok(())
    }

    pub fn squash_additional_change_set(
        self,
        additional_change_set: Self,
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<Self, VMStatus> {
        // First, obtain write sets, delta change sets and events of this and
        // additional change sets.
        let Self {
            resource_write_set: additional_resource_write_set,
            module_write_set: additional_module_write_set,
            aggregator_write_set: additional_aggregator_write_set,
            delta_change_set: additional_delta_change_set,
            events: additional_events,
        } = additional_change_set;
        let Self {
            mut resource_write_set,
            mut module_write_set,
            mut aggregator_write_set,
            mut delta_change_set,
            mut events,
        } = self;

        Self::squash_additional_aggregator_changes(
            &mut delta_change_set,
            &mut aggregator_write_set,
            additional_delta_change_set,
            additional_aggregator_write_set,
        )?;
        Self::squash_additional_writes(&mut resource_write_set, additional_resource_write_set)?;
        Self::squash_additional_writes(&mut module_write_set, additional_module_write_set)?;
        events.extend(additional_events);

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
