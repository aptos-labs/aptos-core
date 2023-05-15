// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::write_change_set::WriteChangeSet;
use aptos_aggregator::delta_change_set::{deserialize, serialize, DeltaChangeSet};
use aptos_types::{
    contract_event::ContractEvent,
    state_store::state_key::{StateKey, StateKeyInner},
    transaction::ChangeSet as StorageChangeSet,
    write_set::WriteOp,
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
    fn check_writes(
        &self,
        resource_writes: &WriteChangeSet,
        module_writes: &WriteChangeSet,
    ) -> Result<(), VMStatus>;
    fn check_events(&self, events: &[ContractEvent]) -> Result<(), VMStatus>;
}

#[derive(Debug)]
pub struct AptosChangeSet {
    resource_writes: WriteChangeSet,
    module_writes: WriteChangeSet,
    aggregator_writes: WriteChangeSet,
    deltas: DeltaChangeSet,
    events: Vec<ContractEvent>,
}

impl AptosChangeSet {
    pub fn new(
        resource_writes: WriteChangeSet,
        module_writes: WriteChangeSet,
        aggregator_writes: WriteChangeSet,
        deltas: DeltaChangeSet,
        events: Vec<ContractEvent>,
        checker: &dyn SizeChecker,
    ) -> anyhow::Result<Self, VMStatus> {
        // TODO: Check aggregator writes?
        checker.check_writes(&resource_writes, &module_writes)?;
        checker.check_events(&events)?;
        let change_set = Self {
            resource_writes,
            module_writes,
            aggregator_writes,
            deltas,
            events,
        };
        Ok(change_set)
    }

    pub fn from_change_set(change_set: StorageChangeSet) -> Self {
        let (write_set, events) = change_set.into_inner();

        let mut resource_writes = WriteChangeSet::empty();
        let mut module_writes = WriteChangeSet::empty();
        for (state_key, write_op) in write_set {
            if let StateKeyInner::AccessPath(ap) = state_key.inner() {
                if ap.is_code() {
                    module_writes.insert((state_key, write_op));
                    continue;
                }
            }
            // Aggregator writes should never be included ina change set!
            resource_writes.insert((state_key, write_op));
        }
        Self {
            resource_writes,
            module_writes,
            aggregator_writes: WriteChangeSet::empty(),
            deltas: DeltaChangeSet::empty(),
            events,
        }
    }

    pub fn resource_writes(&self) -> &WriteChangeSet {
        &self.resource_writes
    }

    pub fn module_writes(&self) -> &WriteChangeSet {
        &self.module_writes
    }

    pub fn aggregator_writes(&self) -> &WriteChangeSet {
        &self.aggregator_writes
    }

    pub fn deltas(&self) -> &DeltaChangeSet {
        &self.deltas
    }

    pub fn events(&self) -> &[ContractEvent] {
        &self.events
    }

    pub fn into_inner(
        self,
    ) -> (
        WriteChangeSet,
        WriteChangeSet,
        WriteChangeSet,
        DeltaChangeSet,
        Vec<ContractEvent>,
    ) {
        (
            self.resource_writes,
            self.module_writes,
            self.aggregator_writes,
            self.deltas,
            self.events,
        )
    }

    fn squash_delta_change_set(&mut self, deltas: DeltaChangeSet) -> anyhow::Result<()> {
        use WriteOp::*;
        for (key, mut op) in deltas.into_iter() {
            if let Some(r) = self.aggregator_writes.get_mut(&key) {
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

    fn squash_module_write_change_set(
        &mut self,
        module_writes: WriteChangeSet,
    ) -> anyhow::Result<()> {
        for (key, write) in module_writes.into_iter() {
            match self.module_writes.entry(key) {
                Occupied(mut entry) => {
                    if !WriteOp::squash(entry.get_mut(), write)? {
                        entry.remove();
                    }
                },
                Vacant(entry) => {
                    entry.insert(write);
                },
            }
        }
        Ok(())
    }

    fn squash_resource_write_change_set(
        &mut self,
        resource_writes: WriteChangeSet,
    ) -> anyhow::Result<()> {
        for (key, write) in resource_writes.into_iter() {
            match self.resource_writes.entry(key) {
                Occupied(mut entry) => {
                    if !WriteOp::squash(entry.get_mut(), write)? {
                        entry.remove();
                    }
                },
                Vacant(entry) => {
                    entry.insert(write);
                },
            }
        }
        Ok(())
    }

    fn squash_aggregator_write_change_set(
        &mut self,
        aggregator_writes: WriteChangeSet,
    ) -> anyhow::Result<()> {
        for (key, write) in aggregator_writes.into_iter() {
            match self.aggregator_writes.entry(key) {
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
        let (resource_writes, module_writes, aggregator_writes, deltas, events) =
            change_set.into_inner();
        self.squash_delta_change_set(deltas)?;
        self.squash_resource_write_change_set(resource_writes)?;
        self.squash_module_write_change_set(module_writes)?;
        self.squash_aggregator_write_change_set(aggregator_writes)?;
        self.squash_events(events)?;
        Ok(())
    }
}
