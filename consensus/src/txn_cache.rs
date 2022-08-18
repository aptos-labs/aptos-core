// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519Signature};
use aptos_crypto::{hash::DefaultHasher, HashValue, PrivateKey, Uniform};
use aptos_types::account_address::AccountAddress;
use aptos_types::chain_id::ChainId;
use aptos_types::transaction::{RawTransaction, Script, SignedTransaction};
use bcs::to_bytes;
use concurrent_lru::sharded::LruCache;
use dashmap::DashMap;
use dashmap::DashSet;
use num_traits::Signed;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use rand::seq::SliceRandom;
use rand::thread_rng;
use rayon::prelude::*;
use rayon::ThreadPool;
use serde::Serialize;
use std::convert::{TryFrom, TryInto};
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};
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

pub struct ConcurrentTxnCache {
    mysterious_counter: AtomicU64,
    state: LruCache<[u8; 32], u64>,
    use_crypto_hash: bool,
}

impl ConcurrentTxnCache {
    pub fn new(cache_size: usize, use_crypto_hash: bool) -> ConcurrentTxnCache {
        ConcurrentTxnCache {
            mysterious_counter: AtomicU64::new(0),
            state: LruCache::new(cache_size as u64),
            use_crypto_hash,
        }
    }

    pub fn get_key(&self, item: &SignedTransaction) -> [u8; 32] {
        if self.use_crypto_hash {
            let bytes = to_bytes(item).unwrap();
            let mut hasher = DefaultHasher::new(b"CacheTesting");
            hasher.update(&bytes);
            let hash_res: HashValue = hasher.finish();
            hash_res.get_bytes()
        } else {
            let mut ret = [0_u8; 32];
            let account_bytes = item.sender().into_bytes();
            let seq_num_bytes = item.sequence_number().to_le_bytes();
            for i in 0..24 {
                ret[i] = account_bytes[i];
            }
            for i in 0..8 {
                ret[i + 24] = seq_num_bytes[i];
            }
            ret
        }
    }

    /// return whether the entry exists.
    pub fn insert(&self, item: &SignedTransaction) -> bool {
        let unique_value = self.mysterious_counter.fetch_add(1_u64, Ordering::SeqCst);
        let entry = self
            .state
            .get_or_init(self.get_key(item), 1, |_e| unique_value);
        let hit = *entry.value() != unique_value;
        hit
    }
}

fn filter_and_update(
    cache: &ConcurrentTxnCache,
    items: &Vec<SignedTransaction>,
    pool: &ThreadPool,
    chunk_size: usize,
) -> Vec<SignedTransaction> {
    pool.install(|| {
        items
            .par_chunks(chunk_size)
            .flat_map(|chunk| {
                let mut sub = Vec::new();
                for item in chunk.iter() {
                    let in_cache = cache.insert(&item);
                    if !in_cache {
                        sub.push(item.clone());
                    }
                }
                sub
            })
            .collect::<Vec<SignedTransaction>>()
    })

    // let mut ret = Vec::new();
    // for item in items {
    //     let in_cache = cache.insert(getKey(item));
    //     let in_cache = false;
    //     if !in_cache {
    //         ret.push(item.clone());
    //     }
    // }
    // ret
    // let chunk_size = 100;
    // pool.install(|| {
    //     items
    //         .par_chunks(chunk_size)
    //         .flat_map(&|chunk: &[SignedTransaction]| {
    //             let mut ret = Vec::new();
    //             for i in chunk.iter() {
    //                 let in_cache = cache.insert(getKey(i));
    //                 if !in_cache {
    //                     ret.push(i.clone());
    //                 }
    //             }
    //             ret
    //         })
    //         .collect::<Vec<SignedTransaction>>()
    // })
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

fn test_hit_rate(
    hit_rate: u32,
    cache_size: usize,
    thread_pool_size: usize,
    chunk_size: usize,
    use_crypto_hash: bool,
) -> () {
    let batch_size = 10000;
    let hit_limit = hit_rate * batch_size / 100;
    let thread_pool: ThreadPool = rayon::ThreadPoolBuilder::new()
        .num_threads(thread_pool_size)
        .build()
        .unwrap();

    let my_cache: ConcurrentTxnCache = ConcurrentTxnCache::new(cache_size, use_crypto_hash);

    fill_cache(&my_cache, cache_size);

    let mut batch = create_batch(batch_size);
    for i in 0..hit_limit {
        my_cache.insert(&batch[i as usize]);
    }

    batch.shuffle(&mut thread_rng());

    let start = Instant::now();
    let remaining = filter_and_update(&my_cache, &batch, &thread_pool, chunk_size);
    let duration = start.elapsed();
    println!(
        "hitRate={}, cacheSize={}, threadPoolSize={}, use_crypto_hash={} => duration={:?}, remaining={}",
        hit_rate, cache_size, thread_pool_size, use_crypto_hash, duration, remaining.len()
    );
}

#[test]
fn rati_test() {
    let hit_rates = [1, 10];
    let chunk_sizes = [100];
    let cache_sizes = [70000];
    let thread_pool_sizes = [4];
    for hit_rate in hit_rates {
        for cache_size in cache_sizes {
            for thread_pool_size in thread_pool_sizes {
                for chunk_size in chunk_sizes {
                    test_hit_rate(hit_rate, cache_size, thread_pool_size, chunk_size, false);
                    test_hit_rate(hit_rate, cache_size, thread_pool_size, chunk_size, true);
                }
            }
        }
    }
}
