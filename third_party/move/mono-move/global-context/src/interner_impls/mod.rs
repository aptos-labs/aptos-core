// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Concurrent interner implementations with different strategies.
//!
//! This module provides multiple interner implementations that vary in:
//! - Map layer: RwLock (BTreeMap/HashMap) vs DashMap
//! - Arena layer: Coupled, Decoupled, Sharded, Per-thread, Chunked

use std::hash::Hash;

pub mod arena;
pub mod dashmap_chunked;
pub mod dashmap_mutex;
pub mod dashmap_perthread_array;
pub mod dashmap_sharded;
pub mod rwlock_btree;
pub mod rwlock_decoupled;
pub mod rwlock_hashmap;
pub mod stable_ptr;

pub use stable_ptr::StablePtr;

/// Common trait for all interner implementations.
pub trait InternerImpl<T: Hash + Eq + Clone + Send + Sync + 'static>: Send + Sync {
    /// Creates a new interner instance.
    fn new() -> Self;

    /// Interns a value and returns a stable pointer.
    ///
    /// If the value already exists, returns the existing pointer.
    /// Otherwise, allocates the value and returns a new pointer.
    fn intern(&self, value: &T) -> StablePtr<T>;

    /// Clears all interned values.
    fn flush(&mut self);

    /// Returns the approximate memory usage in bytes.
    fn memory_usage(&self) -> usize;

    /// Returns the number of unique interned values.
    fn len(&self) -> usize;

    /// Returns true if no values are interned.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
