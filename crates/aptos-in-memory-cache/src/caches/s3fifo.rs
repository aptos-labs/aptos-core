// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::Cache;
use quick_cache::{sync::Cache as S3FIFOCache, Lifecycle, Weighter};
use std::hash::{BuildHasher, Hash};

impl<K, V, We, B, L> Cache<K, V> for S3FIFOCache<K, V, We, B, L>
where
    K: Eq + Hash + Clone + Send + Sync,
    V: Clone + Send + Sync,
    We: Weighter<K, V> + Clone + Send + Sync,
    B: BuildHasher + Clone + Send + Sync,
    L: Lifecycle<K, V> + Clone + Send + Sync,
{
    fn get(&self, key: &K) -> Option<V> {
        S3FIFOCache::get(self, key)
    }

    fn insert(&self, key: K, value: V) -> (u64, u64) {
        S3FIFOCache::insert(self, key, value);
        (0, 0)
    }

    fn total_size(&self) -> u64 {
        S3FIFOCache::weight(self)
    }
}
