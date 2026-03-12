// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module defines [`DashMap`]-based interner for strings, address-name
//! pairs, type tags, and signature tokens.
//!
//! For zero-copy lookups through equivalent types, [`LookupKey`] wraps a
//! borrowed value and implements [`Hash`] and [`Equivalent`] against the
//! corresponding [`InternerKey`]. This enables cross-format deduplication.
//!
//! # Safety model
//!
//! Key types dereference [`GlobalArenaPtr`] in their [`Hash`], [`PartialEq`]
//! and [`Equivalent`] trait implementations. This is not possible to enforce
//! at type system level because it requires the following:
//!
//!   If global arena is reset, the interner is also reset.
//!
//! There is no way to cleanly enforce this relationship.

use crate::{alloc::GlobalArenaPtr, ExecutableId};
use dashmap::{DashMap, Entry, Equivalent};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};
use std::hash::{Hash, Hasher};

/// **Internal** key representation used by interner. Wraps interned data
/// pointer.
///
/// # Safety
///
/// **Must** implement structural hash and equality.
struct InternerKey<T: ?Sized>(GlobalArenaPtr<T>);

/// **Internal** lookup key representation used by interner. Is used to wrap
/// equivalent keys for zero-copy lookups.
struct LookupKey<'a, T: ?Sized>(&'a T);

/// Interner based on [`DashMap`]. Stores global arena-allocated pointers,
/// deduplicating them based on structural hash and equality.
pub(super) struct DashMapInterner<T: ?Sized> {
    // Note: using ahash for fast yet DoS resistant lookups because structural
    // hash is used here.
    inner: DashMap<InternerKey<T>, GlobalArenaPtr<T>, ahash::RandomState>,
}

#[allow(private_bounds)]
impl<T> DashMapInterner<T>
where
    InternerKey<T>: Hash + Eq,
    T: ?Sized,
{
    /// Creates a new interner with default settings.
    pub(crate) fn new() -> Self {
        Self {
            inner: DashMap::with_hasher(ahash::RandomState::new()),
        }
    }

    /// Returns the pointer to interned data if it exists, and [`None`]
    /// otherwise.
    ///
    /// # Safety invariant
    ///
    /// Any [`GlobalArenaPtr`] stored in the map **must** be valid and safe
    /// to dereference. That is, if the data behind the pointer is deallocated,
    /// the interner has to be reset.
    pub(super) fn get<Q>(&self, key: &Q) -> Option<GlobalArenaPtr<T>>
    where
        Q: ?Sized,
        for<'a> LookupKey<'a, Q>: Hash + Equivalent<InternerKey<T>>,
    {
        self.inner.get(&LookupKey(key)).map(|entry| *entry.value())
    }

    /// Inserts the pointer to the interner (key is derived from the pointer
    /// and **must** have structural hash and equality). If the entry exists
    /// (e.g., due to a race condition), does not insert the value and returns
    /// the existing pointer instead. If the entry does not exist, inserts the
    /// pointer and returns its copy.
    ///
    /// # Safety
    ///
    /// Pointer that is being inserted **must** be safe to dereference.
    ///
    /// # Safety invariant
    ///
    /// Any [`GlobalArenaPtr`] stored in the map **must** be valid and safe to
    /// dereference.That is, the interner has to be reset if the data behind
    /// the pointer is deallocated.
    pub(super) unsafe fn insert(&self, ptr: GlobalArenaPtr<T>) -> GlobalArenaPtr<T> {
        match self.inner.entry(InternerKey(ptr)) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => *entry.insert(ptr),
        }
    }

    /// Returns the number of interned entries.
    pub(super) fn len(&self) -> usize {
        self.inner.len()
    }

    /// Removes all entries from the interner's cache.
    pub(super) fn reset(&self) {
        self.inner.clear()
    }
}

impl<T> Default for DashMapInterner<T>
where
    InternerKey<T>: Hash + Eq,
    T: ?Sized,
{
    fn default() -> Self {
        Self::new()
    }
}

//
// Only private APIs below. These implementations are used when getting the
// data from the interner map or inserting into it. Hence, the caller must
// enforce the safety preconditions to ensure every raw pointer dereference
// is safe and sound.
// ----------------------------------------------------------------------------

impl Hash for InternerKey<str> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // SAFETY: User of interner must enforce the safety precondition: the
        // global arena pointer points to a live allocation.
        unsafe {
            self.0.as_ref_unchecked().hash(state);
        }
    }
}

impl Hash for LookupKey<'_, str> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl PartialEq for InternerKey<str> {
    fn eq(&self, other: &Self) -> bool {
        // SAFETY: User of interner must enforce the safety precondition: the
        // global arena pointer points to a live allocation.
        unsafe { self.0.as_ref_unchecked() == other.0.as_ref_unchecked() }
    }
}

impl Eq for InternerKey<str> {}

impl Equivalent<InternerKey<str>> for LookupKey<'_, str> {
    fn equivalent(&self, key: &InternerKey<str>) -> bool {
        // SAFETY: User of interner must enforce the safety precondition: the
        // global arena pointer points to a live allocation.
        unsafe { self.0 == key.0.as_ref_unchecked() }
    }
}

impl Hash for InternerKey<ExecutableId> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // SAFETY: User of interner must enforce the safety precondition: the
        // global arena pointer points to a live allocation.
        unsafe {
            let id = self.0.as_ref_unchecked();
            id.address.hash(state);
            id.name.as_ref_unchecked().hash(state);
        }
    }
}

impl Hash for LookupKey<'_, (&AccountAddress, &IdentStr)> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0 .0.hash(state);
        self.0 .1.hash(state);
    }
}

impl PartialEq for InternerKey<ExecutableId> {
    fn eq(&self, other: &Self) -> bool {
        // SAFETY: User of interner must enforce the safety precondition: the
        // global arena pointer points to a live allocation.
        unsafe {
            let this_id = self.0.as_ref_unchecked();
            let other_id = other.0.as_ref_unchecked();
            this_id.address == other_id.address
                && this_id.name.as_ref_unchecked() == other_id.name.as_ref_unchecked()
        }
    }
}

impl Eq for InternerKey<ExecutableId> {}

impl Equivalent<InternerKey<ExecutableId>> for LookupKey<'_, (&AccountAddress, &IdentStr)> {
    fn equivalent(&self, other: &InternerKey<ExecutableId>) -> bool {
        // SAFETY: User of interner must enforce the safety precondition: the
        // global arena pointer points to a live allocation.
        unsafe {
            let other_id = other.0.as_ref_unchecked();
            self.0 .0 == &other_id.address && self.0 .1.as_str() == other_id.name.as_ref_unchecked()
        }
    }
}
