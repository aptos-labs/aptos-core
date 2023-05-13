// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::state_store::state_key::StateKey;
use std::collections::{
    btree_map::{Entry, IntoIter, Iter},
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
