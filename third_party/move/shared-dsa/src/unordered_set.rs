// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use rustc_hash::{FxBuildHasher, FxHashSet};
use std::{borrow::Borrow, fmt, hash::Hash, iter::FromIterator};

/// A fast, non-cryptographic, non-hash-DoS-resistant hash set.
/// Iteration over elements is not exposed to avoid any reliance on
/// non-deterministic ordering.
#[derive(Clone)]
pub struct UnorderedSet<K> {
    inner: FxHashSet<K>,
}

// ---------------------------------------------------------------------------
// Methods that require no bounds on K
// ---------------------------------------------------------------------------

impl<K> UnorderedSet<K> {
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: FxHashSet::default(),
        }
    }

    /// Creates an empty set pre-allocated to hold at least `capacity` elements
    /// without reallocation. The load factor is handled internally, so pass the
    /// number of elements you expect, not an inflated value.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: FxHashSet::with_capacity_and_hasher(capacity, FxBuildHasher),
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
    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

// ---------------------------------------------------------------------------
// Methods that require K: Hash + Eq
// ---------------------------------------------------------------------------

impl<K: Hash + Eq> UnorderedSet<K> {
    #[inline]
    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.contains(value)
    }

    #[inline]
    pub fn get<Q>(&self, value: &Q) -> Option<&K>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.get(value)
    }

    /// Inserts a value. Returns `true` if the value was newly inserted, or
    /// `false` if it was already present.
    #[inline]
    pub fn insert(&mut self, value: K) -> bool {
        self.inner.insert(value)
    }

    #[inline]
    pub fn replace(&mut self, value: K) -> Option<K> {
        self.inner.replace(value)
    }

    /// Removes a value. Returns `true` if the value was present.
    #[inline]
    pub fn remove<Q>(&mut self, value: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.remove(value)
    }

    #[inline]
    pub fn take<Q>(&mut self, value: &Q) -> Option<K>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.take(value)
    }

    /// Reserves space for at least `additional` more elements. Pass the number
    /// of elements you expect to add, not an inflated value — the load factor
    /// is handled internally.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional);
    }

    #[inline]
    pub fn is_subset(&self, other: &UnorderedSet<K>) -> bool {
        self.inner.is_subset(&other.inner)
    }

    #[inline]
    pub fn is_superset(&self, other: &UnorderedSet<K>) -> bool {
        self.inner.is_superset(&other.inner)
    }

    #[inline]
    pub fn is_disjoint(&self, other: &UnorderedSet<K>) -> bool {
        self.inner.is_disjoint(&other.inner)
    }
}

// ---------------------------------------------------------------------------
// Trait implementations
// ---------------------------------------------------------------------------

impl<K> Default for UnorderedSet<K> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<K> fmt::Debug for UnorderedSet<K> {
    /// Only shows the length to avoid exposing arbitrary iteration order.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnorderedSet")
            .field("len", &self.inner.len())
            .finish()
    }
}

impl<K: Hash + Eq> PartialEq for UnorderedSet<K> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<K: Hash + Eq> Eq for UnorderedSet<K> {}

impl<K: Hash + Eq> FromIterator<K> for UnorderedSet<K> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = K>>(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl<K: Hash + Eq> Extend<K> for UnorderedSet<K> {
    #[inline]
    fn extend<I: IntoIterator<Item = K>>(&mut self, iter: I) {
        self.inner.extend(iter);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_contains_remove() {
        let mut set = UnorderedSet::new();
        assert!(set.is_empty());

        assert!(set.insert(1));
        assert!(set.insert(2));
        assert!(!set.insert(1)); // duplicate
        assert_eq!(set.len(), 2);

        assert!(set.contains(&1));
        assert!(!set.contains(&3));

        assert!(set.remove(&1));
        assert!(!set.remove(&1));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_get_and_take() {
        let mut set = UnorderedSet::new();
        set.insert(42);
        assert_eq!(set.get(&42), Some(&42));
        assert_eq!(set.take(&42), Some(42));
        assert!(set.is_empty());
    }

    #[test]
    fn test_replace() {
        let mut set = UnorderedSet::new();
        assert_eq!(set.replace(1), None);
        assert_eq!(set.replace(1), Some(1));
    }

    #[test]
    fn test_clear() {
        let mut set = UnorderedSet::with_capacity(16);
        set.insert(1);
        set.clear();
        assert!(set.is_empty());
    }

    #[test]
    fn test_set_relations() {
        let a: UnorderedSet<i32> = [1, 2, 3].into_iter().collect();
        let b: UnorderedSet<i32> = [1, 2, 3, 4, 5].into_iter().collect();
        let c: UnorderedSet<i32> = [6, 7].into_iter().collect();

        assert!(a.is_subset(&b));
        assert!(!b.is_subset(&a));
        assert!(b.is_superset(&a));
        assert!(a.is_disjoint(&c));
        assert!(!a.is_disjoint(&b));
    }

    #[test]
    fn test_default() {
        let set: UnorderedSet<String> = Default::default();
        assert!(set.is_empty());
    }

    #[test]
    fn test_eq() {
        let a: UnorderedSet<i32> = [1, 2, 3].into_iter().collect();
        let b: UnorderedSet<i32> = [3, 1, 2].into_iter().collect();
        assert_eq!(a, b);
    }

    #[test]
    fn test_extend() {
        let mut set = UnorderedSet::new();
        set.insert(1);
        set.extend([2, 3, 4]);
        assert_eq!(set.len(), 4);
    }

    #[test]
    fn test_reserve() {
        let mut set = UnorderedSet::new();
        set.insert(1);
        set.reserve(100);
    }

    #[test]
    fn test_debug() {
        let mut set = UnorderedSet::new();
        set.insert(42);
        let s = format!("{:?}", set);
        assert!(s.contains("len: 1"));
    }
}
