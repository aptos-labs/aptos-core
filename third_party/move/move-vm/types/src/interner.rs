// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

const INITIAL_SIZE: usize = 32;

/// Efficient generic interner implementation.
///
/// It uses the technique from https://matklad.github.io/2020/03/22/fast-simple-rust-interner.html
/// to avoid making an additional copy of the interned value.
pub struct BTreeInterner<T: 'static> {
    next_size: usize,

    map: BTreeMap<&'static T, usize>,
    vec: Vec<&'static T>,

    buffer: Vec<T>,
    pool: Vec<Vec<T>>,
}

impl<T> BTreeInterner<T> {
    /// Creates a new empty interner.
    pub fn new() -> Self {
        Self {
            next_size: INITIAL_SIZE * 2,
            map: BTreeMap::new(),
            vec: Vec::new(),
            buffer: Vec::with_capacity(INITIAL_SIZE),
            pool: Vec::new(),
        }
    }
}

impl<T> Default for BTreeInterner<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> BTreeInterner<T>
where
    T: Ord,
{
    /// Interns a value and returns its index.
    pub fn intern(&mut self, val: T) -> usize {
        if let Some(idx) = self.map.get(&val) {
            return *idx;
        }

        unsafe {
            let r = self.alloc(val);
            self.vec.push(r);
            let idx = self.vec.len() - 1;
            self.map.insert(r, idx);
            idx
        }
    }

    /// Returns a reference to the value corresponding to the index.
    /// Returns None if the index is out of bounds.
    pub fn get(&self, idx: usize) -> Option<&T> {
        self.vec.get(idx).cloned()
    }

    /// Returns the number of interned values.
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    /// Returns true if the interner is empty.
    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    /// Allocates a value in the internal buffer.
    ///
    /// In case the current buffer is full, a new one will be allocated, with double the capacity,
    /// guaranteeing no reallocations. This allows us to store the reference to the value in other
    /// data structures safely.
    ///
    /// Note that this function is still UNSAFE, because the returned reference does not really have
    /// a static lifetime -- it cannot outlive the interner itself. If you need to give the reference
    /// out to an external caller, you need to shorten its lifetime to that of the interner.
    unsafe fn alloc(&mut self, val: T) -> &'static T {
        if self.buffer.len() >= self.buffer.capacity() {
            let new_buffer = Vec::with_capacity(self.next_size);
            self.next_size *= 2;

            let old_buffer = std::mem::replace(&mut self.buffer, new_buffer);
            self.pool.push(old_buffer);
        }

        self.buffer.push(val);
        unsafe { &*(self.buffer.last().expect("last always exists") as *const T) }
    }
}
