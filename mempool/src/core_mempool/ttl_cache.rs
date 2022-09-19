// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::{BTreeMap, HashMap},
    time::Duration,
};

struct ValueInfo<V> {
    value: V,
    ttl: Duration,
}

pub struct TtlCache<K, V> {
    capacity: usize,
    data: HashMap<K, ValueInfo<V>>,
    ttl_index: BTreeMap<Duration, K>,
}

impl<K, V> TtlCache<K, V>
where
    K: std::cmp::Eq + std::hash::Hash + std::clone::Clone,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            data: HashMap::new(),
            ttl_index: BTreeMap::new(),
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.data.get(key).map(|v| &v.value)
    }

    pub fn insert(&mut self, key: K, value: V, expiration_time: Duration) {
        // Remove the old entry from data and ttl_index (if it exists)
        match self.data.remove(&key) {
            Some(info) => {
                self.ttl_index.remove(&info.ttl);
            }
            None => {
                // Remove the oldest entry if the cache is still full
                if self.data.len() == self.capacity {
                    if let Some(tst) = self.ttl_index.keys().next().cloned() {
                        if let Some(key) = self.ttl_index.remove(&tst) {
                            self.data.remove(&key);
                        }
                    }
                }
            }
        }

        // Insert the new transaction
        let value_info = ValueInfo {
            value,
            ttl: expiration_time,
        };
        self.ttl_index.insert(expiration_time, key.clone());
        self.data.insert(key, value_info);
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        match self.data.remove(key) {
            Some(info) => {
                self.ttl_index.remove(&info.ttl);
                Some(info.value)
            }
            None => None,
        }
    }

    pub fn gc(&mut self, gc_time: Duration) {
        // Remove the expired entries.
        let mut active = self.ttl_index.split_off(&gc_time);
        for key in self.ttl_index.values() {
            self.data.remove(key);
        }
        self.ttl_index.clear();
        self.ttl_index.append(&mut active);
    }

    #[cfg(test)]
    pub fn size(&self) -> usize {
        self.data.len()
    }
}
