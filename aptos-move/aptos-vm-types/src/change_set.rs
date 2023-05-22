// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::delta_change_set::{deserialize, serialize, DeltaChangeSet};
use aptos_types::{
    contract_event::ContractEvent, write_set::WriteOp,
};
use move_core_types::vm_status::VMStatus;
use std::collections::{
    btree_map::{
        Entry::{Occupied, Vacant},
    },
};
use aptos_types::write_set::WriteSet;

pub trait SizeChecker {
    fn check_writes(&self, writes: &WriteSet) -> Result<(), VMStatus>;
    fn check_events(&self, events: &[ContractEvent]) -> Result<(), VMStatus>;
}

#[derive(Debug, Clone)]
pub struct AptosChangeSet {
    writes: WriteSet,
    deltas: DeltaChangeSet,
    events: Vec<ContractEvent>,
}

impl AptosChangeSet {
    pub fn new(
        writes: WriteSet,
        deltas: DeltaChangeSet,
        events: Vec<ContractEvent>,
        checker: &dyn SizeChecker,
    ) -> anyhow::Result<Self, VMStatus> {
        checker.check_writes(&writes)?;
        checker.check_events(&events)?;
        let change_set = Self {
            writes,
            deltas,
            events,
        };
        Ok(change_set)
    }

    pub fn writes(&self) -> &WriteSet {
        &self.writes
    }

    pub fn deltas(&self) -> &DeltaChangeSet {
        &self.deltas
    }

    pub fn events(&self) -> &[ContractEvent] {
        &self.events
    }

    pub fn into_inner(self) -> (WriteSet, DeltaChangeSet, Vec<ContractEvent>) {
        (self.writes, self.deltas, self.events)
    }

    pub fn squash_delta_change_set(self, other: DeltaChangeSet) -> anyhow::Result<Self> {
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

        Ok(Self {
            writes: write_set.freeze()?,
            deltas: delta_set,
            events,
        })
    }

    pub fn squash_write_set(self, other_write_set: WriteSet, other_events: Vec<ContractEvent>) -> anyhow::Result<Self> {
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

        Ok(Self {
            writes: write_set.freeze()?,
            deltas: delta,
            events,
        })
    }

    pub fn squash(self, other: Self) -> anyhow::Result<Self> {
        let (writes, deltas, events) = other.into_inner();
        self.squash_write_set(writes, events)?
            .squash_delta_change_set(deltas)
    }
}
