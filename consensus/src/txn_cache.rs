// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519Signature};
use aptos_crypto::{hash::DefaultHasher, HashValue, PrivateKey, Uniform};
use aptos_types::account_address::AccountAddress;
use aptos_types::chain_id::ChainId;
use aptos_types::transaction::{RawTransaction, Script, SignedTransaction};
use bcs::to_bytes;
use dashmap::DashMap;
use dashmap::DashSet;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use rayon::prelude::*;
use rayon::ThreadPool;
use serde::Serialize;
use std::convert::{TryFrom, TryInto};
use std::time::Instant;

fn generate_txn(id: u64) -> SignedTransaction {
    let txn: SignedTransaction = SignedTransaction::new(
        RawTransaction::new_script(
            AccountAddress::random(),
            0,
            Script::new(vec![], vec![], vec![]),
            0,
            0,
            100 * id,
            ChainId::test(),
        ),
        Ed25519PrivateKey::generate_for_testing().public_key(),
        Ed25519Signature::try_from(&[1u8; 64][..]).unwrap(),
    );
    txn
}

struct CacheState {
    caches: [DashSet<HashValue>; 2],
    current_idx: usize,
}

impl CacheState {
    fn new() -> CacheState {
        CacheState {
            caches: [DashSet::new(), DashSet::new()],
            current_idx: 0,
        }
    }

    /// Returns a bool whether the item was in cache. After executing,
    /// the item will be guaranteed to be in the current cache. If the item
    /// was added to the current cache, its new size is also returned, o.w. None.
    fn lru_update(&self, hash: HashValue) -> (bool, Option<usize>) {
        if !self.caches[self.current_idx].insert(hash) {
            // hash was in the current cache, no need to return size.
            (true, None)
        } else {
            // hash was added to active.
            //Some(self.caches[self.current_idx].len())
            (self.caches[1 - self.current_idx].contains(&hash), None)
        }
    }

    fn switch(&mut self) {
        self.caches[self.current_idx].clear();
        self.current_idx = 1 - self.current_idx;
    }
}

pub struct ConcurrentTxnCache {
    max_size: usize,
    state: RwLock<CacheState>,
}

impl ConcurrentTxnCache {
    pub fn new(cache_size: usize) -> ConcurrentTxnCache {
        ConcurrentTxnCache {
            max_size: cache_size,
            state: RwLock::new(CacheState::new()),
        }
    }

    fn hash<U: Clone + Serialize>(&self, element: U) -> HashValue {
        let bytes = to_bytes(&element).unwrap();
        let mut hasher = DefaultHasher::new(b"CacheTesting");
        hasher.update(&bytes);
        let hash_res = hasher.finish();
        hash_res
    }

    pub fn insert<U: Clone + Serialize>(&mut self, element: U) {
        let key = self.hash(element);
        let mut state = self.state.write();
        if let Some(cur_size) = state.lru_update(key).1 {
            if cur_size > self.max_size {
                state.switch();
            }
        }
    }
    pub fn filter_and_update<U: Clone + Serialize + Sync + Send>(
        &mut self,
        items: &Vec<U>,
        pool: ThreadPool,
    ) -> Vec<U> {
        // let mut ret = Vec::new();

        // .collect::<Vec<U>>()

        let chunk_size = 100;
        pool.install(|| {
            items
                .par_chunks(chunk_size)
                .flat_map(&|chunk: &[U]| {
                    let mut ret = Vec::new();
                    for i in chunk.iter() {
                        let key = self.hash(i);
                        let (in_cache, cur_size) = self.state.read().lru_update(key);
                        if let Some(cur_size) = cur_size {
                            if cur_size > self.max_size {
                                self.state.write().switch();
                            }
                        }

                        if !in_cache {
                            ret.push(i.clone());
                        }
                    }
                    ret
                })
                .collect::<Vec<U>>()
        })
    }
}

fn fill_cache(cache: &mut ConcurrentTxnCache, cache_size: usize) -> () {
    for i in 1..cache_size + 1 {
        let txn = generate_txn(i.try_into().unwrap());
        cache.insert(&txn);
    }
}

fn create_batch(batch_size: u32, hit_limit: u32, cache_size: u32) -> Vec<SignedTransaction> {
    let mut txn_batch: Vec<SignedTransaction> = Vec::new();

    let residue = batch_size - hit_limit;
    for i in 1..hit_limit + 1 {
        let txn = generate_txn(i.try_into().unwrap());
        txn_batch.push(txn);
    }
    for i in 1..residue + 1 {
        let txn = generate_txn((i + cache_size).into());
        txn_batch.push(txn);
    }
    txn_batch
}

fn test_hit_rate(hit_rate: u32, cache_size: usize, thread_pool_size: usize) -> () {
    let batch_size = 10000;
    let hit_limit = hit_rate / 100 * batch_size;

    let thread_pool: ThreadPool = rayon::ThreadPoolBuilder::new()
        .num_threads(thread_pool_size)
        .build()
        .unwrap();

    let mut my_cache = ConcurrentTxnCache::new(cache_size);
    let u_cache_size: u32 = cache_size as u32;

    fill_cache(&mut my_cache, cache_size);

    let mut batch = create_batch(batch_size, hit_limit, u_cache_size);

    let start = Instant::now();
    my_cache.filter_and_update(&mut batch, thread_pool);
    let duration = start.elapsed();
    println!(
        "hitRate={}, cacheSize={}, threadPoolSize={} | duration={:?}",
        hit_rate, cache_size, thread_pool_size, duration
    );
}

#[test]
fn rati_test() {
    let hit_rates = vec![10, 50];
    let cache_sizes = vec![70000, 10];
    let thread_pool_sizes = vec![1, 4];
    for hit_rate in hit_rates {
        for cache_size in &cache_sizes {
            for thread_pool_size in &thread_pool_sizes {
                test_hit_rate(hit_rate, *cache_size, *thread_pool_size);
            }
        }
    }
}
