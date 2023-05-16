// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{op::Op, write_change_set::WriteChangeSet};
use aptos_aggregator::delta_change_set::{deserialize, serialize, DeltaChangeSet};
use aptos_types::{
    contract_event::ContractEvent,
    state_store::state_key::{StateKey, StateKeyInner},
    transaction::ChangeSet as StorageChangeSet,
    write_set::{WriteSet, WriteSetMut},
};
use move_core_types::vm_status::{StatusCode, VMStatus};
use move_vm_types::types::Store;
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
}

impl<T> IntoIterator for ChangeSet<T> {
    type IntoIter = IntoIter<StateKey, T>;
    type Item = (StateKey, T);

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

pub trait SizeChecker<T: Store> {
    fn check_writes(
        &self,
        resource_writes: &WriteChangeSet<T>,
        module_writes: &WriteChangeSet<T>,
    ) -> Result<(), VMStatus>;
    fn check_events(&self, events: &[ContractEvent]) -> Result<(), VMStatus>;
}

#[derive(Debug)]
pub struct AptosChangeSet {
    resource_writes: WriteChangeSet<Vec<u8>>,
    module_writes: WriteChangeSet<Vec<u8>>,
    aggregator_writes: WriteChangeSet<Vec<u8>>,
    deltas: DeltaChangeSet,
    events: Vec<ContractEvent>,
}

impl AptosChangeSet {
    pub fn new(
        resource_writes: WriteChangeSet<Vec<u8>>,
        module_writes: WriteChangeSet<Vec<u8>>,
        aggregator_writes: WriteChangeSet<Vec<u8>>,
        deltas: DeltaChangeSet,
        events: Vec<ContractEvent>,
        checker: &dyn SizeChecker<Vec<u8>>,
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
                    module_writes.insert((state_key, Op::from_write_op(write_op)));
                    continue;
                }
            }
            // Aggregator writes should never be included ina change set!
            resource_writes.insert((state_key, Op::from_write_op(write_op)));
        }
        Self {
            resource_writes,
            module_writes,
            aggregator_writes: WriteChangeSet::empty(),
            deltas: DeltaChangeSet::empty(),
            events,
        }
    }

    pub fn resource_writes(&self) -> &WriteChangeSet<Vec<u8>> {
        &self.resource_writes
    }

    pub fn module_writes(&self) -> &WriteChangeSet<Vec<u8>> {
        &self.module_writes
    }

    pub fn aggregator_writes(&self) -> &WriteChangeSet<Vec<u8>> {
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
        WriteChangeSet<Vec<u8>>,
        WriteChangeSet<Vec<u8>>,
        WriteChangeSet<Vec<u8>>,
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

    /// Converts VM-friendly change set into storage-friendly change set on bytes.
    /// Note that deltas ARE IGNORED during this conversion.
    pub fn into_change_set(self) -> anyhow::Result<StorageChangeSet, VMStatus> {
        let (resource_writes, module_writes, aggregator_writes, _deltas, events) =
            self.into_inner();
        let write_set = into_write_set(resource_writes, module_writes, aggregator_writes)?;
        Ok(StorageChangeSet::new_unchecked(write_set, events))
    }

    fn squash_delta_change_set(&mut self, deltas: DeltaChangeSet) -> anyhow::Result<()> {
        for (key, mut op) in deltas.into_iter() {
            if let Some(r) = self.aggregator_writes.get_mut(&key) {
                match r {
                    Op::Creation(data)
                    | Op::Modification(data)
                    | Op::CreationWithMetadata { data, .. }
                    | Op::ModificationWithMetadata { data, .. } => {
                        let val: u128 = deserialize(data);
                        *data = serialize(&op.apply_to(val)?);
                    },
                    Op::Deletion | Op::DeletionWithMetadata { .. } => {
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
        module_writes: WriteChangeSet<Vec<u8>>,
    ) -> anyhow::Result<()> {
        for (key, write) in module_writes.into_iter() {
            match self.module_writes.entry(key) {
                Occupied(mut entry) => {
                    if !Op::squash(entry.get_mut(), write)? {
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
        resource_writes: WriteChangeSet<Vec<u8>>,
    ) -> anyhow::Result<()> {
        for (key, write) in resource_writes.into_iter() {
            match self.resource_writes.entry(key) {
                Occupied(mut entry) => {
                    if !Op::squash(entry.get_mut(), write)? {
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
        aggregator_writes: WriteChangeSet<Vec<u8>>,
    ) -> anyhow::Result<()> {
        for (key, write) in aggregator_writes.into_iter() {
            match self.aggregator_writes.entry(key) {
                Occupied(mut entry) => {
                    if !Op::squash(entry.get_mut(), write)? {
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

        // TODO: There seems to be quite a bit of duplication here. Let's refactor this
        // when we support new types for resources and modules.
        self.squash_resource_write_change_set(resource_writes)?;
        self.squash_module_write_change_set(module_writes)?;
        self.squash_aggregator_write_change_set(aggregator_writes)?;

        self.squash_events(events)?;
        Ok(())
    }
}

/// Utility to merge writes into a single storage-friendly write set.
pub(crate) fn into_write_set(
    resource_writes: WriteChangeSet<Vec<u8>>,
    module_writes: WriteChangeSet<Vec<u8>>,
    aggregator_writes: WriteChangeSet<Vec<u8>>,
) -> anyhow::Result<WriteSet, VMStatus> {
    // Convert to write sets.
    let resource_write_set = resource_writes.into_write_set()?;
    let module_write_set = module_writes.into_write_set()?;
    let aggregator_write_set = aggregator_writes.into_write_set()?;

    // Combine all write sets together
    let combined_write_sets = resource_write_set.into_iter().chain(
        module_write_set
            .into_iter()
            .chain(aggregator_write_set.into_iter()),
    );
    WriteSetMut::new(combined_write_sets)
        .freeze()
        .map_err(|_| VMStatus::Error(StatusCode::DATA_FORMAT_ERROR, None))
}
