// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::check_change_set::CheckChangeSet;
use aptos_aggregator::delta_change_set::{deserialize, serialize, DeltaChangeSet};
use aptos_state_view::StateView;
use aptos_types::{
    contract_event::ContractEvent,
    write_set::{WriteOp, WriteSet},
};
use move_binary_format::errors::Location;
use move_core_types::vm_status::{StatusCode, VMStatus};
use std::collections::btree_map::Entry::{Occupied, Vacant};

/// A change set produced by the VM. Just like VMOutput, this type should
/// be used inside the VM. For storage backends, use ChangeSet.
#[derive(Debug, Clone)]
pub struct VMChangeSet {
    write_set: WriteSet,
    delta_change_set: DeltaChangeSet,
    events: Vec<ContractEvent>,
}

impl VMChangeSet {
    /// Returns an empty change set.
    pub fn empty() -> Self {
        Self {
            write_set: WriteSet::default(),
            delta_change_set: DeltaChangeSet::empty(),
            events: vec![],
        }
    }

    /// Returns a new change set, and checks that it is well-formed.
    pub fn new(
        write_set: WriteSet,
        delta_change_set: DeltaChangeSet,
        events: Vec<ContractEvent>,
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<Self, VMStatus> {
        // Check that writes and deltas have disjoint key set.
        let disjoint = delta_change_set
            .iter()
            .all(|(k, _)| write_set.get(k).is_some());
        if !disjoint {
            return Err(VMStatus::Error(
                StatusCode::DATA_FORMAT_ERROR,
                Some("DeltaChangeSet and WriteSet are not disjoint.".to_string()),
            ));
        }

        let change_set = Self {
            write_set,
            delta_change_set,
            events,
        };

        // Check the newly-formed change set.
        checker.check_change_set(&change_set)?;
        Ok(change_set)
    }

    pub fn write_set(&self) -> &WriteSet {
        &self.write_set
    }

    pub fn delta_change_set(&self) -> &DeltaChangeSet {
        &self.delta_change_set
    }

    pub fn events(&self) -> &[ContractEvent] {
        &self.events
    }

    pub fn unpack(self) -> (WriteSet, DeltaChangeSet, Vec<ContractEvent>) {
        (self.write_set, self.delta_change_set, self.events)
    }

    /// Materializes this change set: all deltas are converted into writes and
    /// are combined with existing write set. In case of materialization fails,
    /// an error is returned.
    pub fn try_materialize(self, state_view: &impl StateView) -> anyhow::Result<Self, VMStatus> {
        let (write_set, delta_change_set, events) = self.unpack();

        // Try to materialize deltas and add them to the write set.
        let mut write_set_mut = write_set.into_mut();
        let delta_writes = delta_change_set.take_materialized(state_view)?;
        delta_writes
            .into_iter()
            .for_each(|item| write_set_mut.insert(item));

        let write_set = write_set_mut.freeze().map_err(|_| {
            VMStatus::Error(
                StatusCode::DATA_FORMAT_ERROR,
                Some(
                    "Failed to freeze write set when converting VMOutput to TransactionOutput"
                        .to_string(),
                ),
            )
        })?;

        Ok(Self {
            write_set,
            delta_change_set: DeltaChangeSet::empty(),
            events,
        })
    }

    pub fn squash(
        self,
        other: Self,
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<Self, VMStatus> {
        use WriteOp::*;

        let (other_write_set, other_delta_change_set, other_events) = other.unpack();
        let (write_set, mut delta_change_set, mut events) = self.unpack();
        let mut write_set_mut = write_set.into_mut();

        let delta_ops = delta_change_set.as_inner_mut();
        let write_ops = write_set_mut.as_inner_mut();

        // First, squash deltas.
        for (key, mut delta_op) in other_delta_change_set.into_iter() {
            if let Some(write_op) = write_ops.get_mut(&key) {
                match write_op {
                    Creation(data)
                    | Modification(data)
                    | CreationWithMetadata { data, .. }
                    | ModificationWithMetadata { data, .. } => {
                        let base: u128 = deserialize(data);
                        let value = delta_op
                            .apply_to(base)
                            .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
                        *data = serialize(&value);
                    },
                    Deletion | DeletionWithMetadata { .. } => {
                        return Err(VMStatus::Error(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                            Some("Cannot squash delta which was already deleted.".to_string()),
                        ));
                    },
                }
            } else {
                match delta_ops.entry(key) {
                    Occupied(entry) => {
                        // In this case, we need to merge the new incoming `delta_op`
                        // to the existing delta, ensuring the strict ordering.
                        delta_op
                            .merge_onto(*entry.get())
                            .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
                        *entry.into_mut() = delta_op;
                    },
                    Vacant(entry) => {
                        entry.insert(delta_op);
                    },
                }
            }
        }

        // Next, squash write ops.
        for (key, write_op) in other_write_set.into_iter() {
            match write_ops.entry(key) {
                Occupied(mut entry) => {
                    // Squashing creation and deletion is a no-op. In that case, we
                    // have to remove the old write op from the write set.
                    let noop = !WriteOp::squash(entry.get_mut(), write_op).map_err(|e| {
                        VMStatus::Error(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                            Some(format!("Error while squashing two write ops: {}.", e)),
                        )
                    })?;
                    if noop {
                        entry.remove();
                    }
                },
                Vacant(entry) => {
                    delta_change_set.remove(entry.key());
                    entry.insert(write_op);
                },
            }
        }

        let write_set = write_set_mut.freeze().map_err(|_| {
            VMStatus::Error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                Some("Error when freezing squashed write sets.".to_string()),
            )
        })?;

        // Squash events.
        events.extend(other_events);

        Self::new(write_set, delta_change_set, events, checker)
    }
}
