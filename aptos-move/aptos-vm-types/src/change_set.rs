// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::write_change_set::WriteChangeSet;
use aptos_aggregator::delta_change_set::{deserialize, serialize, DeltaChangeSet};
use aptos_types::{
    contract_event::ContractEvent, state_store::state_key::StateKey, write_set::WriteOp,
};
use move_core_types::vm_status::VMStatus;
use std::collections::{
    btree_map::{
        Entry,
        Entry::{Occupied, Vacant},
        IntoIter, Iter, Keys,
    },
    BTreeMap,
};

/// Container to hold arbitrary changes to the global state produced by the VM.
#[derive(Clone, Debug)]
pub struct ChangeSet<T> {
    inner: BTreeMap<StateKey, T>,
}

impl<T> ChangeSet<T> {
    pub fn new(items: impl IntoIterator<Item = (StateKey, T)>) -> Self {
        Self {
            inner: items.into_iter().collect(),
        }
    }

    pub fn empty() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[inline]
    pub fn insert(&mut self, change: (StateKey, T)) {
        self.inner.insert(change.0, change.1);
    }

    #[inline]
    pub fn get(&self, key: &StateKey) -> Option<&T> {
        self.inner.get(key)
    }

    #[inline]
    pub fn get_mut(&mut self, key: &StateKey) -> Option<&mut T> {
        self.inner.get_mut(key)
    }

    #[inline]
    pub fn entry(&mut self, key: StateKey) -> Entry<'_, StateKey, T> {
        self.inner.entry(key)
    }

    #[inline]
    pub fn extend<I: IntoIterator<Item = (StateKey, T)>>(&mut self, iter: I) {
        self.inner.extend(iter)
    }

    #[inline]
    pub fn keys(&self) -> Keys<'_, StateKey, T> {
        self.inner.keys()
    }

    #[inline]
    pub fn remove(&mut self, key: &StateKey) -> Option<T> {
        self.inner.remove(key)
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_, StateKey, T> {
        self.inner.iter()
    }

    #[inline]
    pub fn into_iter(self) -> IntoIter<StateKey, T> {
        self.inner.into_iter()
    }
}

pub trait SizeChecker {
    fn check_writes(&self, writes: &WriteChangeSet) -> Result<(), VMStatus>;
    fn check_events(&self, events: &[ContractEvent]) -> Result<(), VMStatus>;
}

#[derive(Debug)]
pub struct AptosChangeSet {
    writes: WriteChangeSet,
    deltas: DeltaChangeSet,
    events: Vec<ContractEvent>,
}

impl AptosChangeSet {
    pub fn new(
        writes: WriteChangeSet,
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

    pub fn writes(&self) -> &WriteChangeSet {
        &self.writes
    }

    pub fn deltas(&self) -> &DeltaChangeSet {
        &self.deltas
    }

    pub fn events(&self) -> &[ContractEvent] {
        &self.events
    }

    pub fn into_inner(self) -> (WriteChangeSet, DeltaChangeSet, Vec<ContractEvent>) {
        (self.writes, self.deltas, self.events)
    }

    fn squash_delta_change_set(&mut self, deltas: DeltaChangeSet) -> anyhow::Result<()> {
        use WriteOp::*;
        for (key, mut op) in deltas.into_iter() {
            if let Some(r) = self.writes.get_mut(&key) {
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
                match self.deltas.entry(key) {
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

        Ok(())
    }

    fn squash_write_change_set(&mut self, writes: WriteChangeSet) -> anyhow::Result<()> {
        for (key, write) in writes.into_iter() {
            match self.writes.entry(key) {
                Occupied(mut entry) => {
                    if !WriteOp::squash(entry.get_mut(), write)? {
                        entry.remove();
                    }
                },
                Vacant(entry) => {
                    self.deltas.remove(entry.key());
                    entry.insert(write);
                },
            }
        }
        Ok(())
    }

    fn squash_events(&mut self, events: Vec<ContractEvent>) -> anyhow::Result<()> {
        self.events.extend(events);
        Ok(())
    }

    pub fn squash(&mut self, change_set: Self) -> anyhow::Result<()> {
        let (writes, deltas, events) = change_set.into_inner();
        self.squash_delta_change_set(deltas)?;
        self.squash_write_change_set(writes)?;
        self.squash_events(events)?;
        Ok(())
    }
}
