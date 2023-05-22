// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::check_change_set::CheckChangeSet;
use aptos_aggregator::delta_change_set::{deserialize, serialize, DeltaChangeSet};
use aptos_types::{
    contract_event::ContractEvent,
    write_set::{WriteOp, WriteSet},
};
use move_core_types::vm_status::VMStatus;
use std::collections::btree_map::Entry::{Occupied, Vacant};

#[derive(Debug, Clone)]
pub struct VMChangeSet {
    write_set: WriteSet,
    delta_change_set: DeltaChangeSet,
    events: Vec<ContractEvent>,
}

impl VMChangeSet {
    pub fn new(
        write_set: WriteSet,
        delta_change_set: DeltaChangeSet,
        events: Vec<ContractEvent>,
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<Self, VMStatus> {
        let change_set = Self {
            write_set,
            delta_change_set,
            events,
        };
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

    pub fn into_inner(self) -> (WriteSet, DeltaChangeSet, Vec<ContractEvent>) {
        (self.write_set, self.delta_change_set, self.events)
    }

    pub fn squash_delta_change_set(
        self,
        other: DeltaChangeSet,
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<Self, VMStatus> {
        use WriteOp::*;

        let (write_set, mut delta_set, events) = self.into_inner();
        let mut write_set = write_set.into_mut();

        let delta_ops = delta_set.as_inner_mut();
        let write_ops = write_set.as_inner_mut();

        for (key, mut op) in other.into_iter() {
            if let Some(r) = write_ops.get_mut(&key) {
                match r {
                    Creation(data)
                    | Modification(data)
                    | CreationWithMetadata { data, .. }
                    | ModificationWithMetadata { data, .. } => {
                        let val: u128 = deserialize(data);
                        *data = serialize(&op.apply_to(val)?);
                    },
                    Deletion | DeletionWithMetadata { .. } => {
                        anyhow::bail!("Failed to apply Aggregator delta -- value already deleted");
                    },
                }
            } else {
                match delta_ops.entry(key) {
                    Occupied(entry) => {
                        // In this case, we need to merge the new incoming `op` to the existing
                        // delta, ensuring the strict ordering.
                        op.merge_onto(*entry.get())?;
                        *entry.into_mut() = op;
                    },
                    Vacant(entry) => {
                        entry.insert(op);
                    },
                }
            }
        }

        let write_set = write_set.freeze()?;
        Self::new(write_set, delta_set, events, checker)
    }

    pub fn squash_write_set(
        self,
        other_write_set: WriteSet,
        other_events: Vec<ContractEvent>,
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<Self, VMStatus> {
        let (write_set, mut delta, mut events) = self.into_inner();
        let mut write_set = write_set.into_mut();
        let write_ops = write_set.as_inner_mut();

        for (key, op) in other_write_set.into_iter() {
            match write_ops.entry(key) {
                Occupied(mut entry) => {
                    if !WriteOp::squash(entry.get_mut(), op)? {
                        entry.remove();
                    }
                },
                Vacant(entry) => {
                    delta.remove(entry.key());
                    entry.insert(op);
                },
            }
        }

        events.extend(other_events);

        let write_set = write_set.freeze()?;
        Self::new(write_set, delta, events, checker)
    }

    pub fn squash(
        self,
        other: Self,
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<Self, VMStatus> {
        let (writes, deltas, events) = other.into_inner();
        self.squash_write_set(writes, events, checker)?
            .squash_delta_change_set(deltas, checker)
    }
}
