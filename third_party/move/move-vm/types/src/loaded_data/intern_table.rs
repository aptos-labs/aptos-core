// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0
//! Loaded definition of code data used in runtime.

use boxcar::Vec as CVec;
use dashmap::{mapref::entry::Entry, DashMap};
use std::{fmt::Debug, hash::Hash, borrow::Borrow};

#[derive(Debug)]
pub struct IndexMap<K: Debug + Hash + Eq> {
    pub(crate) map: DashMap<K, usize>,
    entries: CVec<K>,
}

impl<K: Eq + Hash + Clone + Debug> IndexMap<K> {
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
            entries: CVec::new(),
        }
    }

    pub fn get_or_insert(&self, key: K) -> usize {
        match self.map.entry(key) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let idx = self.entries.push(entry.key().clone());
                entry.insert(idx);
                idx
            },
        }
    }

    pub fn get_by_index(&self, index: usize) -> &K {
        &self.entries[index]
    }

    pub fn get<Q>(&self, key: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.get(key).map(|idx| *idx)
    }
}

#[cfg(test)]
mod test {
    use super::IndexMap;
    use proptest::{collection::vec, prelude::*};
    use std::collections::HashMap;

    fn insertion_test(data: Vec<Vec<u64>>) {
        let index_map = IndexMap::new();
        std::thread::scope(|s| {
            // Spawn threads to insert multiple data to the same index map concurrently.
            let join_handles = data
                .iter()
                .map(|values| {
                    s.spawn(|| {
                        values
                            .iter()
                            .map(|value| {
                                let idx = index_map.get_or_insert(*value);
                                // Make sure the insertion result is as expected.
                                assert!(*index_map.get_by_index(idx) == *value);
                                idx
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect::<Vec<_>>();

            // Make sure all key is mapped to the exact same index.
            let mut dedup_map = HashMap::new();
            for (handle, values) in join_handles.into_iter().zip(data.iter()) {
                let insertion_results = handle.join().unwrap();
                for (idx, value) in insertion_results.into_iter().zip(values.iter()) {
                    match dedup_map.entry(*value) {
                        std::collections::hash_map::Entry::Occupied(entry) => {
                            assert!(*entry.get() == idx)
                        },
                        std::collections::hash_map::Entry::Vacant(entry) => {
                            entry.insert(idx);
                        },
                    }
                }
            }
        });
    }

    proptest! {
        #[test]
        fn test_concurrent_insertion(insertion_data in vec(vec(0..20u64, 1000), 10)) {
            insertion_test(insertion_data);
        }
    }
}
