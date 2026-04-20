// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use rustc_hash::{FxBuildHasher, FxHashMap};
use std::{borrow::Borrow, collections::hash_map::Entry, fmt, hash::Hash, iter::FromIterator};

/// A fast, non-cryptographic, non-hash-DoS-resistant hash map.
/// Iteration over keys or key-value pairs are not exposed to avoid any
/// reliance on non-deterministic ordering.
#[derive(Clone)]
pub struct UnorderedMap<K, V> {
    inner: FxHashMap<K, V>,
}

// ---------------------------------------------------------------------------
// Methods that require no bounds on K/V
// ---------------------------------------------------------------------------

impl<K, V> UnorderedMap<K, V> {
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: FxHashMap::default(),
        }
    }

    /// Creates an empty map pre-allocated to hold at least `capacity` elements
    /// without reallocation. The load factor is handled internally, so pass the
    /// number of elements you expect, not an inflated value.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: FxHashMap::with_capacity_and_hasher(capacity, FxBuildHasher),
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

impl<K: Hash + Eq, V> UnorderedMap<K, V> {
    #[inline]
    pub fn get<Q>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.get(k)
    }

    #[inline]
    pub fn get_mut<Q>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.get_mut(k)
    }

    #[inline]
    pub fn contains_key<Q>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.contains_key(k)
    }

    /// Inserts a key-value pair. Returns the previous value if the key was
    /// already present, or `None` if it was newly inserted.
    #[inline]
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.inner.insert(k, v)
    }

    /// Removes a key, returning its value if the key was present.
    #[inline]
    pub fn remove<Q>(&mut self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.remove(k)
    }

    /// Removes a key, returning the key-value pair if the key was present.
    #[inline]
    pub fn remove_entry<Q>(&mut self, k: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.remove_entry(k)
    }

    #[inline]
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        self.inner.entry(key)
    }

    /// Retains only the key-value pairs for which the predicate returns `true`.
    /// The predicate is applied in an arbitrary order — callers must ensure
    /// that `f` does not depend on the order in which key-value pairs are visited.
    #[inline]
    pub fn retain(&mut self, f: impl FnMut(&K, &mut V) -> bool) {
        self.inner.retain(f);
    }

    /// Reserves space for at least `additional` more elements. Pass the number
    /// of elements you expect to add, not an inflated value — the load factor
    /// is handled internally.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional);
    }
}

// ---------------------------------------------------------------------------
// Trait implementations
// ---------------------------------------------------------------------------

impl<K, V> Default for UnorderedMap<K, V> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> fmt::Debug for UnorderedMap<K, V> {
    /// Only shows the length to avoid exposing arbitrary iteration order.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnorderedMap")
            .field("len", &self.inner.len())
            .finish()
    }
}

impl<K: Hash + Eq, V: PartialEq> PartialEq for UnorderedMap<K, V> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<K: Hash + Eq, V: Eq> Eq for UnorderedMap<K, V> {}

impl<K: Hash + Eq, V> FromIterator<(K, V)> for UnorderedMap<K, V> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl<K: Hash + Eq, V> Extend<(K, V)> for UnorderedMap<K, V> {
    #[inline]
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        self.inner.extend(iter);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_get_remove() {
        let mut map = UnorderedMap::new();
        assert!(map.is_empty());

        assert_eq!(map.insert("a", 1), None);
        assert_eq!(map.insert("b", 2), None);
        assert_eq!(map.len(), 2);

        assert_eq!(map.get("a"), Some(&1));
        assert_eq!(map.get("c"), None);
        assert!(map.contains_key("b"));

        assert_eq!(map.remove("a"), Some(1));
        assert_eq!(map.len(), 1);
        assert!(!map.contains_key("a"));
    }

    #[test]
    fn test_get_mut() {
        let mut map = UnorderedMap::new();
        map.insert("x", 10);
        *map.get_mut("x").unwrap() = 20;
        assert_eq!(map.get("x"), Some(&20));
    }

    #[test]
    fn test_remove_entry() {
        let mut map = UnorderedMap::new();
        map.insert("k", 42);
        assert_eq!(map.remove_entry("k"), Some(("k", 42)));
        assert!(map.is_empty());
    }

    #[test]
    fn test_entry_api() {
        let mut map = UnorderedMap::new();
        map.entry("a").or_insert(1);
        map.entry("a").and_modify(|v| *v += 10);
        assert_eq!(map.get("a"), Some(&11));
    }

    #[test]
    fn test_clear() {
        let mut map = UnorderedMap::with_capacity(16);
        map.insert(1, 1);
        map.clear();
        assert!(map.is_empty());
    }

    #[test]
    fn test_default() {
        let map: UnorderedMap<String, i32> = Default::default();
        assert!(map.is_empty());
    }

    #[test]
    fn test_eq() {
        let a: UnorderedMap<_, _> = [(1, 2), (3, 4)].into_iter().collect();
        let b: UnorderedMap<_, _> = [(3, 4), (1, 2)].into_iter().collect();
        assert_eq!(a, b);
    }

    #[test]
    fn test_extend() {
        let mut map = UnorderedMap::new();
        map.insert(1, "a");
        map.extend([(2, "b"), (3, "c")]);
        assert_eq!(map.len(), 3);
    }

    #[test]
    fn test_reserve() {
        let mut map = UnorderedMap::new();
        map.insert(1, 1);
        map.reserve(100);
    }

    #[test]
    fn test_debug() {
        let mut map = UnorderedMap::new();
        map.insert(1, 2);
        let s = format!("{:?}", map);
        assert!(s.contains("len: 1"));
    }
}
