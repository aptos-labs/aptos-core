// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::btree_map::{IntoIter, Iter};
use std::collections::BTreeMap;
use crate::state_store::state_key::StateKey;
use crate::vm_types::delta::DeltaOp;
use crate::vm_types::write::{AptosWrite, Op};

/// Container to hold arbitrary changes to the global state.
#[derive(Debug)]
pub struct ChangeSet<T> {
    inner: BTreeMap<StateKey, T>,
}

impl<T> ChangeSet<T> {
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
}

impl AptosChangeSet {
    pub fn get_deltas(&self) -> &ChangeSet<DeltaOp> {
        &self.deltas
    }

    pub fn get_writes(&self) -> &ChangeSet<Op<AptosWrite>> {
        &self.writes
    }
}

