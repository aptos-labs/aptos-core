// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    delta::DeltaOp,
    remote_cache::StateViewWithRemoteCache,
    write::{squash_writes, WriteOp},
};
use anyhow::bail;
use aptos_types::{
    contract_event::ContractEvent,
    state_store::state_key::StateKey,
    write_set::{WriteSet, WriteSetMut},
};
use move_binary_format::errors::Location;
use move_core_types::vm_status::VMStatus;
use std::collections::{
    btree_map::{
        Entry,
        Entry::{Occupied, Vacant},
        IntoIter, Iter,
    },
    BTreeMap,
};

/// Container to hold arbitrary changes to the global state.
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
    pub fn remove(&mut self, key: &StateKey) -> Option<T> {
        self.inner.remove(key)
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_, StateKey, T> {
        self.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a ChangeSet<T> {
    type IntoIter = Iter<'a, StateKey, T>;
    type Item = (&'a StateKey, &'a T);

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl<T> IntoIterator for ChangeSet<T> {
    type IntoIter = IntoIter<StateKey, T>;
    type Item = (StateKey, T);

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

/// Syntactic sugar for deltas and writes.
pub type DeltaChangeSet = ChangeSet<DeltaOp>;
pub type WriteChangeSet = ChangeSet<WriteOp>;

impl WriteChangeSet {
    /// Converts the set of writes produced by the VM into storage-friendly
    /// write set containing blobs. Returns an error if serialization of one
    /// of the writes fails.
    pub fn into_write_set(self) -> anyhow::Result<WriteSet, VMStatus> {
        let mut write_set_mut = WriteSetMut::default();
        for (key, write) in self {
            let write_op = write
                .into_write_op()
                .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
            write_set_mut.insert((key, write_op));
        }
        let write_set = write_set_mut.freeze().expect("freeze cannot fail");
        Ok(write_set)
    }

    /// Merges two set of writes. Returns an error if an error occurred
    /// while squashing the writes.
    pub fn merge_writes(
        &mut self,
        writes: impl IntoIterator<Item = (StateKey, WriteOp)>,
    ) -> anyhow::Result<()> {
        for (key, write) in writes {
            match self.entry(key) {
                // The write is overwriting the previous one.
                Occupied(mut entry) => {
                    if !(squash_writes(entry.get_mut(), write)?) {
                        // No-op.
                        entry.remove();
                    }
                },
                // Record a new write.
                Vacant(entry) => {
                    entry.insert(write);
                },
            }
        }
        Ok(())
    }

    /// Materializes a set of deltas and merges it with a set of writes. If materialization
    /// was not successful, an error is returned.
    pub fn merge_deltas(
        &mut self,
        deltas: DeltaChangeSet,
        view: &impl StateViewWithRemoteCache,
    ) -> anyhow::Result<(), VMStatus> {
        // Make sure we assert state keys are indeed disjoint.
        assert!(self.inner.keys().all(|k| !deltas.inner.contains_key(k)));
        let materialized_deltas = deltas.try_materialize(view)?;

        // It is safe to simply extend writes because writes and deltas have different
        // state keys within a single transaction.
        self.extend(materialized_deltas);
        Ok(())
    }
}

impl DeltaChangeSet {
    /// Materializes a set of deltas into writes, returning an error when delta application
    /// fails.
    pub fn try_materialize(
        self,
        state_view: &impl StateViewWithRemoteCache,
    ) -> anyhow::Result<WriteChangeSet, VMStatus> {
        let mut materialized_set = WriteChangeSet::empty();
        for (state_key, delta) in self {
            let write = delta.try_materialize(state_view, &state_key)?;
            materialized_set.insert((state_key, write));
        }

        // All deltas are applied successfully.
        Ok(materialized_set)
    }
}

#[derive(Debug)]
pub struct AptosChangeSet {
    writes: WriteChangeSet,
    deltas: DeltaChangeSet,
    events: Vec<ContractEvent>,
}

impl AptosChangeSet {
    pub fn new(writes: WriteChangeSet, deltas: DeltaChangeSet, events: Vec<ContractEvent>) -> Self {
        Self {
            writes,
            deltas,
            events,
        }
    }

    pub fn writes(&self) -> &WriteChangeSet {
        &self.writes
    }

    pub fn deltas(&self) -> &DeltaChangeSet {
        &self.deltas
    }

    pub fn into_inner(self) -> (WriteChangeSet, DeltaChangeSet, Vec<ContractEvent>) {
        (self.writes, self.deltas, self.events)
    }

    pub fn squash(self, other: Self) -> anyhow::Result<Self> {
        // Unpack the change sets.
        let (mut writes, mut deltas, mut events) = self.into_inner();
        let (other_writes, other_deltas, other_events) = other.into_inner();

        extend_with_writes(&mut writes, &mut deltas, other_writes)?;
        extend_with_deltas(&mut writes, &mut deltas, other_deltas)?;
        events.extend(other_events);

        Ok(Self::new(writes, deltas, events))
    }
}

fn extend_with_deltas(
    writes: &mut WriteChangeSet,
    deltas: &mut DeltaChangeSet,
    other_deltas: DeltaChangeSet,
) -> anyhow::Result<()> {
    for (key, mut delta) in other_deltas.into_iter() {
        if let Some(write) = writes.get_mut(&key) {
            // Delta can only be applied to aggregators!
            if let WriteOp::AggregatorWrite(op) = write {
                match op {
                    Some(v) => {
                        *v = delta.apply_to(*v)?;
                    },
                    None => {
                        bail!(format!("Failed to apply aggregator delta {:?} because the value is already deleted", delta));
                    },
                }
            }
        } else {
            match deltas.entry(key) {
                Occupied(entry) => {
                    // In this case, we need to merge the new incoming delta to the existing
                    // delta, ensuring the strict ordering.
                    delta.merge_onto(*entry.get())?;
                    *entry.into_mut() = delta;
                },
                Vacant(entry) => {
                    entry.insert(delta);
                },
            }
        }
    }
    Ok(())
}

fn extend_with_writes(
    writes: &mut WriteChangeSet,
    deltas: &mut DeltaChangeSet,
    other_writes: WriteChangeSet,
) -> anyhow::Result<()> {
    for (key, other_write) in other_writes.into_iter() {
        match writes.entry(key) {
            Occupied(mut entry) => {
                if !(squash_writes(entry.get_mut(), other_write)?) {
                    entry.remove();
                }
            },
            Vacant(entry) => {
                deltas.remove(entry.key());
                entry.insert(other_write);
            },
        }
    }
    Ok(())
}
