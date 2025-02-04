// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_in_memory_cache::Cache;
use common::{NotATransaction, TestCache};
use std::time::Duration;

pub mod common;

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_in_memory_cache::caches::sync_mutex::SyncMutexCache;

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_insert_out_of_order() {
        let c = SyncMutexCache::with_capacity(10);
        let cache = TestCache::with_capacity(c, 150, 100);
        let key = 100;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key).clone(), Some(value));

        let key = 101;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key).clone(), Some(value));

        let key = 105;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key).clone(), Some(value));

        let key = 103;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key).clone(), Some(value));

        let key = 102;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key).clone(), Some(value));

        let key = 104;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key).clone(), Some(value));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_array_wrap_around() {
        let c = SyncMutexCache::with_capacity(10);
        let cache = TestCache::with_capacity(c, 150, 100);
        let key = 7;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key).clone(), Some(value));

        let key = 8;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key).clone(), Some(value));

        let key = 12;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key).clone(), Some(value));

        let key = 10;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key).clone(), Some(value));

        let key = 9;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key).clone(), Some(value));

        let key = 11;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value.clone());
        assert_eq!(cache.get(&key).clone(), Some(value));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_eviction_on_size_limit() {
        let c = SyncMutexCache::with_capacity(10);
        let cache = TestCache::with_capacity(c, 56, 48);

        // Insert initial items
        for i in 0..6 {
            let value = NotATransaction::new(i as i64);
            cache.insert(i, value);
        }

        assert_eq!(
            cache.total_size(),
            6 * std::mem::size_of_val(&NotATransaction::new(0))
        );

        tokio::time::sleep(Duration::from_micros(1)).await;

        // This insert should trigger eviction
        let key = 6;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value);

        // Wait for eviction to occur
        tokio::time::sleep(Duration::from_micros(1)).await;

        assert_eq!(
            cache.total_size(),
            6 * std::mem::size_of_val(&NotATransaction::new(0))
        );

        // Further inserts to ensure eviction continues correctly
        for i in 7..10 {
            let value = NotATransaction::new(i as i64);
            cache.insert(i, value);
        }

        // Wait for eviction to occur
        tokio::time::sleep(Duration::from_micros(1)).await;

        assert_eq!(
            cache.total_size(),
            6 * std::mem::size_of_val(&NotATransaction::new(0))
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_eviction_out_of_order_inserts() {
        let c = SyncMutexCache::with_capacity(20);
        let cache = TestCache::with_capacity(c, 88, 80);

        // Insert items out of order
        let keys = [0, 5, 1, 3, 7, 2, 6, 4, 9, 8];
        for &key in &keys {
            let value = NotATransaction::new(key as i64);
            cache.insert(key, value);
        }

        tokio::time::sleep(Duration::from_micros(1)).await;

        assert_eq!(
            cache.total_size(),
            10 * std::mem::size_of_val(&NotATransaction::new(0))
        );

        tokio::time::sleep(Duration::from_micros(1)).await;

        // This insert should trigger eviction
        let key = 10;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value);

        // Wait for eviction to occur
        tokio::time::sleep(Duration::from_micros(1)).await;

        assert_eq!(
            cache.total_size(),
            10 * std::mem::size_of_val(&NotATransaction::new(0))
        );

        tokio::time::sleep(Duration::from_micros(1)).await;

        // Further inserts to ensure eviction continues correctly
        let key = 11;
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value);

        tokio::time::sleep(Duration::from_micros(1)).await;

        assert_eq!(
            cache.total_size(),
            10 * std::mem::size_of_val(&NotATransaction::new(0))
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_eviction_with_array_wrap_around() {
        let c = SyncMutexCache::with_capacity(10);
        let cache = TestCache::with_capacity(c, 48, 40);

        // Insert items to fill the cache
        for i in 5..10 {
            let value = NotATransaction::new(i as i64);
            cache.insert(i, value);
        }

        tokio::time::sleep(Duration::from_micros(1)).await;

        assert_eq!(
            cache.total_size(),
            5 * std::mem::size_of_val(&NotATransaction::new(0))
        );

        tokio::time::sleep(Duration::from_micros(1)).await;

        // Insert more items to cause wrap-around
        for i in 10..12 {
            let value = NotATransaction::new(i as i64);
            cache.insert(i, value);
        }

        tokio::time::sleep(Duration::from_micros(1)).await;

        assert_eq!(
            cache.total_size(),
            5 * std::mem::size_of_val(&NotATransaction::new(0))
        );

        // Insert even more items to fully wrap-around
        for i in 12..15 {
            let value = NotATransaction::new(i as i64);
            cache.insert(i, value);
        }

        tokio::time::sleep(Duration::from_micros(1)).await;

        assert_eq!(
            cache.total_size(),
            5 * std::mem::size_of_val(&NotATransaction::new(0))
        );
    }
}
