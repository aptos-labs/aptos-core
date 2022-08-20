// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::txn_cache::CacheOption::{
    LruCache_CryptoHashAsKey, LruCache_ExistingFieldAsKey, NoCacheOpAtAll,
};
use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519Signature};
use aptos_crypto::{hash::DefaultHasher, HashValue, PrivateKey, Uniform};
use aptos_types::account_address::AccountAddress;
use aptos_types::chain_id::ChainId;
use aptos_types::transaction::{RawTransaction, Script, SignedTransaction};
use bcs::to_bytes;
use concurrent_lru::sharded::LruCache;
use dashmap::DashMap;
use dashmap::DashSet;
use itertools::Dedup;
use num_traits::Signed;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use parking_lot::RwLock;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use rayon::prelude::*;
use rayon::ThreadPool;
use serde::Serialize;
use std::convert::{TryFrom, TryInto};
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

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

pub struct ConcurrentTxnCache {
    state: LruCache<[u8; 32], u64>,
    cache_option: CacheOption,
    hit_rate: u32,
}

#[derive(Debug, Clone, Copy)]
enum CacheOption {
    LruCache_CryptoHashAsKey,
    LruCache_ExistingFieldAsKey,
    NoCacheOpAtAll,
}

impl ConcurrentTxnCache {
    fn new(cache_size: usize, cache_option: CacheOption, hit_rate: u32) -> ConcurrentTxnCache {
        ConcurrentTxnCache {
            state: LruCache::new(cache_size as u64),
            cache_option,
            hit_rate,
        }
    }

    pub fn get_key(&self, item: &SignedTransaction) -> [u8; 32] {
        match self.cache_option {
            NoCacheOpAtAll => [0_u8; 32],
            LruCache_CryptoHashAsKey => {
                let bytes = to_bytes(item).unwrap();
                let mut hasher = DefaultHasher::new(b"CacheTesting");
                hasher.update(&bytes);
                hasher.finish().get_bytes()
            }
            LruCache_ExistingFieldAsKey => {
                let mut ret = [0_u8; 32];
                let account_bytes = item.sender().into_bytes();
                let seq_num_bytes = item.sequence_number().to_le_bytes();
                ret[..24].copy_from_slice(&account_bytes[0..24]);
                ret[24..32].copy_from_slice(&seq_num_bytes[0..8]);
                ret
            }
        }
    }

    /// return whether the entry exists.
    pub fn insert(&self, item: &SignedTransaction) -> bool {
        match self.cache_option {
            NoCacheOpAtAll => thread_rng().gen_range(0_u32, 100_u32) < self.hit_rate,
            _ => {
                let nonce = thread_rng().gen::<u64>();
                let entry = self.state.get_or_init(self.get_key(item), 1, |_e| nonce);
                let hit = *entry.value() != nonce;
                hit
            }
        }
    }
}

fn fill_cache(cache: &ConcurrentTxnCache, cache_size: usize) -> () {
    for i in 1..cache_size + 1 {
        let txn = generate_txn(i.try_into().unwrap());
        cache.insert(&txn);
    }
}

fn create_batch(batch_size: u32) -> Vec<SignedTransaction> {
    let mut txn_batch: Vec<SignedTransaction> = Vec::new();

    for i in 0..batch_size {
        let txn = generate_txn(i.try_into().unwrap());
        txn_batch.push(txn);
    }
    txn_batch
}

struct Collector {
    should_clone: bool,
    items: Mutex<Vec<SignedTransaction>>,
    counter: AtomicU64,
}

impl Collector {
    pub fn new(should_clone: bool) -> Collector {
        Collector {
            should_clone,
            items: Mutex::new(Vec::new()),
            counter: AtomicU64::new(0),
        }
    }

    pub fn push(&self, tx: &SignedTransaction) {
        if self.should_clone {
            self.items.lock().push(tx.clone());
        } else {
            self.counter.fetch_add(1_u64, Ordering::SeqCst);
        }
    }
    pub fn get_total(&self) -> usize {
        if self.should_clone {
            self.items.lock().len()
        } else {
            self.counter.fetch_add(0_u64, Ordering::SeqCst) as usize
        }
    }
}

fn test_hit_rate(
    hit_rate: u32,
    cache_size: usize,
    thread_pool_size: usize,
    chunk_size: usize,
    cache_option: CacheOption,
    should_clone: bool,
) -> () {
    let mut durations: Vec<Duration> = (0..10)
        .map(|_iteration| {
            // Init cache.
            let cache: ConcurrentTxnCache =
                ConcurrentTxnCache::new(cache_size, cache_option, hit_rate);
            fill_cache(&cache, cache_size);

            // Init tx batch.
            let batch_size = 10000;
            let mut batch = create_batch(batch_size);
            let hit_limit = hit_rate * batch_size / 100;
            let mut collector = Collector::new(should_clone);
            let thread_pool: ThreadPool = rayon::ThreadPoolBuilder::new()
                .num_threads(thread_pool_size)
                .build()
                .unwrap();
            for i in 0..hit_limit {
                cache.insert(&batch[i as usize]);
            }
            batch.shuffle(&mut thread_rng());

            let start = Instant::now();
            thread_pool.install(|| {
                batch.par_chunks(chunk_size).for_each(|chunk| {
                    let mut rng = thread_rng();
                    for tx in chunk {
                        let in_cache = cache.insert(tx); //Almost no-op.
                        if !in_cache {
                            collector.push(tx);
                        }
                    }
                })
            });
            let duration = start.elapsed();
            duration
        })
        .collect();

    durations.sort();

    println!(
        "hitRate={}, cacheSize={}, threadPoolSize={}, parChunkSize={}, cloneTXsToNewVec={}, cacheOption={:?}  =>  durationP90={:?}",
        hit_rate, cache_size, thread_pool_size, chunk_size, should_clone, cache_option, durations[8]
    );
}

#[test]
fn rati_test() {
    let hit_rates = [10];
    let chunk_sizes = [1, 100];
    let cache_sizes = [70000];
    let thread_pool_sizes = [4];
    for hit_rate in hit_rates {
        for cache_size in cache_sizes {
            for thread_pool_size in thread_pool_sizes {
                for chunk_size in chunk_sizes {
                    for should_clone in [true, false] {
                        for cache_strategy in [
                            NoCacheOpAtAll,
                            LruCache_CryptoHashAsKey,
                            LruCache_ExistingFieldAsKey,
                        ] {
                            test_hit_rate(
                                hit_rate,
                                cache_size,
                                thread_pool_size,
                                chunk_size,
                                cache_strategy,
                                should_clone,
                            );
                        }
                    }
                }
            }
        }
    }
}
