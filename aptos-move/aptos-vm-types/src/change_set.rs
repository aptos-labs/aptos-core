// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::btree_map::{Entry, IntoIter, Iter};
use std::collections::btree_map::Entry::{Occupied, Vacant};
use std::collections::BTreeMap;
use aptos_types::contract_event::ContractEvent;
use aptos_types::state_store::state_key::StateKey;
use aptos_types::transaction::CheckChangeSet;
use aptos_types::write_set::{WriteSet, WriteSetMut};
use move_binary_format::errors::{Location, PartialVMResult};
use move_core_types::vm_status::VMStatus;
use crate::delta::DeltaOp;
use crate::remote_cache::StateViewWithRemoteCache;
use crate::write::{AptosWrite, Op};

/// Container to hold arbitrary changes to the global state.
#[derive(Clone, Debug, PartialEq, Eq)]
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

    pub fn insert(&mut self, change: (StateKey, T)) {
        self.inner.insert(change.0, change.1);
    }

    pub fn get(&self, key: &StateKey) -> Option<&T> {
        self.inner.get(key)
    }

    pub fn get_mut(&mut self, key: &StateKey) -> Option<&mut T> {
        self.inner.get_mut(key)
    }

    pub fn entry(&mut self, key: StateKey) -> Entry<'_, StateKey, T> {
        self.inner.entry(key)
    }

    pub fn remove(&mut self, key: &StateKey) -> Option<T> {
        self.inner.remove(key)
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_, StateKey, T> {
        self.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a ChangeSet<T> {
    type Item = (&'a StateKey, &'a T);
    type IntoIter = Iter<'a, StateKey, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl<T> IntoIterator for ChangeSet<T> {
    type Item = (StateKey, T);
    type IntoIter = IntoIter<StateKey, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

/// Contains set of changes produced by the VM to the global state. Includes both
/// writes (i.e. resource creation, modification and deletion) and deltas (partial)
/// updates.
#[derive(Debug)]
pub struct AptosChangeSet {
    deltas: ChangeSet<DeltaOp>,
    writes: ChangeSet<Op<AptosWrite>>,
    events: Vec<ContractEvent>,
}

impl AptosChangeSet {
    pub fn new(writes: ChangeSet<Op<AptosWrite>>, deltas: ChangeSet<DeltaOp>, events: Vec<ContractEvent>) -> Self {
        Self {
            writes,
            deltas,
            events,
        }
    }

    pub fn writes(&self) -> &ChangeSet<Op<AptosWrite>> {
        &self.writes
    }

    pub fn deltas(&self) -> &ChangeSet<DeltaOp> {
        &self.deltas
    }

    pub fn check(&self, checker: &dyn CheckChangeSet) -> Result<(), VMStatus> {
        Ok(())
        // TODO: change the arg type of checker to `AptosChangeSet`!
        // checker.check_change_set(&self)
    }

    pub fn into_inner(self) -> (ChangeSet<Op<AptosWrite>>, ChangeSet<DeltaOp>, Vec<ContractEvent>) {
        (self.writes, self.deltas, self.events)
    }

    pub fn into_write_set(
        writes: ChangeSet<Op<AptosWrite>>,
    ) -> anyhow::Result<WriteSet, VMStatus> {
        let mut write_set_mut = WriteSetMut::default();
        for (key, write) in writes {
            write_set_mut.insert((key, write.into_write_op().map_err(|e| e.finish(Location::Undefined).into_vm_status())?));
        }

        // TODO: revisit!
        let write_set = write_set_mut.freeze().expect("change this");
        Ok(write_set)
    }

    pub fn try_materialize_deltas(
        deltas: ChangeSet<DeltaOp>,
        state_view: &impl StateViewWithRemoteCache,
    ) -> anyhow::Result<ChangeSet<Op<AptosWrite>>, VMStatus> {
        let mut materialized_writes = ChangeSet::empty();
        for (state_key, delta_op) in deltas {
            let write = delta_op.try_materialize(state_view, &state_key)?;
            materialized_writes.insert((state_key, write));
        }

        // All deltas are applied successfully.
        Ok(materialized_writes)
    }

    fn extend_with_deltas(
        writes: &mut ChangeSet<Op<AptosWrite>>,
        deltas: &mut ChangeSet<DeltaOp>,
        other_deltas: ChangeSet<DeltaOp>,
    ) -> anyhow::Result<()> {
        for (key, mut delta_op) in other_deltas.into_iter() {
            if let Some(r) = writes.get_mut(&key) {
                match r {
                    Op::Creation(write)
                    | Op::Modification(write) => {
                        let value: u128 = write.as_aggregator_value()?;
                        *write = AptosWrite::AggregatorValue(delta_op.apply_to(value)?);
                    },
                    Op::Deletion => {
                        anyhow::bail!(format!(
                            "Failed to apply aggregator delta {:?} because the value is already deleted", delta_op,
                        ));
                    },
                }
            } else {
                match deltas.entry(key) {
                    Occupied(entry) => {
                        // In this case, we need to merge the new incoming `op` to the existing
                        // delta, ensuring the strict ordering.
                        delta_op.merge_onto(*entry.get())?;
                        *entry.into_mut() = delta_op;
                    },
                    Vacant(entry) => {
                        entry.insert(delta_op);
                    },
                }
            }
        }
        Ok(())
    }

    pub fn extend_with_writes(
        writes: &mut ChangeSet<Op<AptosWrite>>,
        deltas: &mut ChangeSet<DeltaOp>,
        other_writes: ChangeSet<Op<AptosWrite>>,
    ) -> anyhow::Result<()> {
        for (key, other_op) in other_writes.into_iter() {
            match writes.entry(key) {
                Occupied(mut entry) => {
                    let mut op = entry.get_mut();
                    if !op.squash(other_op)? {
                        entry.remove();
                    }
                },
                Vacant(entry) => {
                    deltas.remove(entry.key());
                    entry.insert(other_op);
                },
            }
        }
        Ok(())
    }

    pub fn squash(self, other: Self) -> anyhow::Result<Self> {
        let (mut writes, mut deltas, mut events) = self.into_inner();
        let (other_writes, other_deltas, other_events) = other.into_inner();
        Self::extend_with_writes(&mut writes, &mut deltas, other_writes)?;
        // TODO: check writes here?
        Self::extend_with_deltas(&mut writes, &mut deltas, other_deltas)?;
        // TODO: check writes here?
        events.extend(other_events);
        // TODO: check events here?
        Ok(Self::new(writes, deltas, events))
    }
}

