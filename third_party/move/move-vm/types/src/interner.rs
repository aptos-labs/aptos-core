// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use parking_lot::RwLock;
#[cfg(test)]
use parking_lot::{MappedRwLockReadGuard, RwLockReadGuard};
use std::{borrow::Cow, collections::BTreeMap};

const INITIAL_SIZE: usize = 1024;

/// Efficient generic interner implementation.
///
/// It uses the technique from https://matklad.github.io/2020/03/22/fast-simple-rust-interner.html
/// to avoid making an additional copy of the interned value.
pub struct ConcurrentBTreeInterner<T: 'static> {
    inner: RwLock<InternerPool<T>>,
}

/// Pool storing the interned values.
struct InternerPool<T: 'static> {
    /// The size for the next allocation of the active buffer.
    /// When the current buffer fills up, it will be moved into the pool and a new one will be allocated.
    next_size: usize,

    /// A mapping from interned values to their corresponding ids.
    map: BTreeMap<&'static T, usize>,
    /// A vector of interned values to allow reverse lookup of values by their ids.
    vec: Vec<&'static T>,

    /// The currently active buffer used to store new interned values.
    buffer: Vec<T>,
    /// A collection of previously filled (frozen) buffers that own interned values.
    pool: Vec<Vec<T>>,
}

impl<T> InternerPool<T> {
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

impl<T> Default for InternerPool<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> InternerPool<T>
where
    T: Ord,
{
    /// Flushes the pool, clearing all interned values.
    ///
    /// Note that this specifically does not reset the current buffer size, nor the next size,
    /// as resetting gives no real benefit. Asymptotically, we are still using O(n) memory.
    ///
    /// Another way to think about this is that after a flush, we are starting with a larger
    /// initial size.
    fn flush(&mut self) {
        self.map.clear();
        self.vec.clear();
        self.buffer.clear();
        self.pool.clear();
    }

    /// Allocates a value in the internal buffer.
    ///
    /// In case the current buffer is full, a new one will be allocated, with double the capacity,
    /// guaranteeing no reallocations. This allows us to store the reference to the value in other
    /// data structures safely.
    ///
    /// SAFETY: this function is still UNSAFE, because the returned reference does not really have
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

impl<T> ConcurrentBTreeInterner<T> {
    /// Creates a new empty concurrent interner.
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(InternerPool::new()),
        }
    }
}

impl<T> Default for ConcurrentBTreeInterner<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ConcurrentBTreeInterner<T>
where
    T: Ord,
{
    /// Interns a value and returns its index.
    pub fn intern(&self, val: T) -> usize
    where
        T: Clone,
    {
        self.intern_deferred(Cow::Owned(val))
    }

    /// Interns a value by reference.
    /// If the value has not been interned yet, a clone needs to be made.
    pub fn intern_by_ref(&self, val: &T) -> usize
    where
        T: Clone,
    {
        self.intern_deferred(Cow::Borrowed(val))
    }

    /// Interns a value using Cow for deferred ownership semantics -- start with a borrow
    /// for the initial lookup, and upgrade to owned if a copy does not yet exist in
    /// the interner's buffer.
    pub fn intern_deferred(&self, val: Cow<T>) -> usize
    where
        T: Clone + Ord,
    {
        {
            let inner = self.inner.read();
            if let Some(idx) = inner.map.get(val.as_ref()) {
                return *idx;
            }
        }

        // Convert the value into owned outside the critical section to reduce contention.
        // (This could be an expensive clone.)
        let val = val.into_owned();

        // Note on synchronization: once we acquire the write lock, we need to check again
        // if the value has already been interned, which could happen due to race conditions.
        let mut inner = self.inner.write();
        if let Some(idx) = inner.map.get(&val) {
            return *idx;
        }

        unsafe {
            let r = inner.alloc(val);
            inner.vec.push(r);
            let idx = inner.vec.len() - 1;
            inner.map.insert(r, idx);
            idx
        }
    }

    /// Returns the number of interned values.
    pub fn len(&self) -> usize {
        self.inner.read().vec.len()
    }

    /// Returns true if the interner is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.read().vec.is_empty()
    }

    /// Looks up a value by reference and returns its index.
    /// Returns None if the value has not been interned yet.
    pub fn lookup(&self, val: &T) -> Option<usize> {
        self.inner.read().map.get(val).cloned()
    }

    /// Returns a reference to the value corresponding to the index.
    /// Returns None if the index is out of bounds.
    ///
    /// This is currently test-only as it's easy to create contention unintentionally --
    /// the caller needs to make sure the returned guard is kept alive as short as possible.
    #[cfg(test)]
    fn get(&self, idx: usize) -> Option<MappedRwLockReadGuard<'_, T>> {
        RwLockReadGuard::try_map(self.inner.read(), |inner| inner.vec.get(idx).cloned()).ok()
    }

    /// Flushes the interner, clearing all interned values.
    ///
    /// This is a DANGEROUS operation as it invalidates all current references to the interned
    /// values, conceptually.
    ///
    /// The caller needs to make sure there are no more active references when calling `flush`.
    /// As we currently return indices as opposed to actual references, this means to not compare
    /// indices across flushes.
    pub fn flush(&self) {
        self.inner.write().flush();
    }
}

#[cfg(test)]
mod btree_interner_tests {
    use super::*;

    #[test]
    fn test_new_empty_interner() {
        let interner = ConcurrentBTreeInterner::<&'static str>::new();
        assert!(interner.is_empty());
        assert_eq!(interner.len(), 0);
    }

    #[test]
    fn test_intern_basic() {
        let interner = ConcurrentBTreeInterner::new();

        let idx1 = interner.intern("hello");
        assert_eq!(interner.len(), 1);
        assert!(!interner.is_empty());

        let _idx2 = interner.intern("world");
        assert_eq!(interner.len(), 2);

        // Test that we get the same index for duplicate values
        let idx3 = interner.intern("hello");
        assert_eq!(idx3, idx1);
        assert_eq!(interner.len(), 2); // Length should not increase
    }

    #[test]
    fn test_intern_duplicate() {
        let interner = ConcurrentBTreeInterner::new();

        let val = "duplicate";
        let idx1 = interner.intern(val);
        let idx2 = interner.intern(val);

        assert_eq!(idx1, idx2);
        assert_eq!(interner.len(), 1);
    }

    #[test]
    fn test_lookup() {
        let interner = ConcurrentBTreeInterner::new();

        // Lookup should return None for non-interned values
        assert_eq!(interner.lookup(&"hello"), None);

        let idx = interner.intern("hello");
        assert_eq!(interner.lookup(&"hello"), Some(idx));
        assert_eq!(interner.lookup(&"world"), None);
    }

    #[test]
    fn test_get() {
        let interner = ConcurrentBTreeInterner::new();

        // Get should return None for out-of-bounds indices
        assert!(interner.get(0).is_none());

        let val = "hello";
        let idx = interner.intern(val);

        // Get should return the correct value
        assert_eq!(*interner.get(idx).unwrap(), val);
        assert!(interner.get(idx + 1).is_none());
    }

    #[test]
    fn test_mixed_intern_methods() {
        let interner = ConcurrentBTreeInterner::new();

        let val1 = "val1".to_string();
        let val2 = "val2".to_string();

        let idx1 = interner.intern(val1.clone());
        let idx2 = interner.intern_by_ref(&val2);

        assert_eq!(interner.len(), 2);
        assert_eq!(&*interner.get(idx1).unwrap(), &val1);
        assert_eq!(&*interner.get(idx2).unwrap(), &val2);

        // Test interning using different methods still detects duplicates
        let idx3 = interner.intern_by_ref(&val1);
        let idx4 = interner.intern(val2.clone());

        assert_eq!(idx3, idx1);
        assert_eq!(idx4, idx2);
        assert_eq!(interner.len(), 2); // Length should not increase
    }

    #[allow(clippy::needless_range_loop)]
    #[test]
    fn test_many_unique_values() {
        let interner = ConcurrentBTreeInterner::new();

        let mut indices = Vec::new();
        let num_values = 1000;

        for i in 0..num_values {
            indices.push(interner.intern(i));

            if i % 10 == 0 {
                for j in 0..=i {
                    assert_eq!(*interner.get(indices[j]).unwrap(), j);
                }
            }
        }

        assert_eq!(interner.len(), num_values);
    }

    #[test]
    fn test_interner_consistency() {
        use proptest::prelude::*;
        use std::collections::HashMap;

        #[derive(Debug, Clone)]
        enum Operation {
            Intern(i32),
            Lookup(i32),
        }

        impl Arbitrary for Operation {
            type Parameters = ();
            type Strategy = BoxedStrategy<Operation>;

            fn arbitrary_with(_args: ()) -> Self::Strategy {
                use proptest::strategy::Strategy;

                (any::<bool>(), any::<i32>())
                    .prop_map(|(is_intern, value)| {
                        if is_intern {
                            Operation::Intern(value)
                        } else {
                            Operation::Lookup(value)
                        }
                    })
                    .boxed()
            }
        }

        proptest!(|(operations: Vec<Operation>)| {
            let interner = ConcurrentBTreeInterner::new();
            let mut expected_indices: HashMap<i32, usize> = HashMap::new();

            for operation in operations {
                match operation {
                    Operation::Intern(value) => {
                        let idx = interner.intern(value);

                        // Check index stability: if we've seen this value before, index should be the same
                        if let Some(&expected_idx) = expected_indices.get(&value) {
                            prop_assert_eq!(idx, expected_idx, "Index stability violated for value {}", value);
                        } else {
                            // First time seeing this value, record the index
                            expected_indices.insert(value, idx);
                        }

                        // Check intern <=> get consistency
                        let retrieved = interner.get(idx);
                        prop_assert!(retrieved.is_some(), "Get returned None for index {}", idx);
                        prop_assert_eq!(*retrieved.unwrap(), value, "Get returned wrong value for index {}", idx);

                        // Check intern <=> lookup consistency
                        let lookup_idx = interner.lookup(&value);
                        prop_assert_eq!(lookup_idx, Some(idx), "Lookup returned wrong index for value {}", value);
                    },
                    Operation::Lookup(value) => {
                        let lookup_idx = interner.lookup(&value);

                        if let Some(idx) = lookup_idx {
                            // If lookup found the value, check consistency
                            let retrieved = interner.get(idx);
                            prop_assert!(retrieved.is_some(), "Get returned None after lookup for value {}", value);
                            prop_assert_eq!(*retrieved.unwrap(), value, "Lookup <=> get consistency violated for value {}", value);

                            // Check index stability
                            if let Some(&expected_idx) = expected_indices.get(&value) {
                                prop_assert_eq!(idx, expected_idx, "Index stability violated for value {}", value);
                            } else {
                                // This shouldn't happen - if lookup found it, we should have recorded it
                                prop_assert!(false, "Lookup found value {} but it wasn't in expected_indices", value);
                            }
                        } else {
                            // If lookup didn't find it, it shouldn't be in our expected indices
                            prop_assert!(!expected_indices.contains_key(&value), "Lookup missed value {} that should exist", value);
                        }
                    }
                }
            }

            // Check that interner length matches our expectations
            prop_assert_eq!(interner.len(), expected_indices.len(), "Interner length mismatch");
        });
    }
}
