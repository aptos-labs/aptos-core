// Copyright © Aptos Foundation

use crate::transaction_compressor::{CompressedHintsTransaction, CompressedKey};
use aptos_block_executor::transaction_hints::TransactionHints;
use dashmap::DashMap;
use rayon::prelude::*;
use std::{
    hash::Hash,
    sync::{
        atomic::{AtomicU32, Ordering::SeqCst},
    },
};

pub struct ParallelTransactionCompressor<K> {
    key_mapping: DashMap<K, CompressedKey>,
    // TODO: consider using thread-local counters instead of atomic.
    next_key: AtomicU32,
}

impl<K: Hash + Eq> ParallelTransactionCompressor<K> {
    pub fn new() -> Self {
        Self {
            key_mapping: DashMap::new(),
            next_key: AtomicU32::new(0),
        }
    }
}

impl<K: Hash + Eq> Default for ParallelTransactionCompressor<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Hash + Clone + Eq + Send + Sync> ParallelTransactionCompressor<K> {
    pub fn map_key(&self, key: &K) -> CompressedKey {
        if let Some(entry) = self.key_mapping.get(key) {
            *entry.value()
        } else {
            let mapped_key = self.next_key.fetch_add(1, SeqCst);
            // The resulting key may be different from `mapped_key` due to concurrency.
            // This may create "holes" in the key space, but that's fine.
            *self
                .key_mapping
                .entry(key.clone())
                .or_insert(mapped_key)
                .value()
        }
    }

    pub fn compress_transactions<T, I>(&mut self, block: I) -> Vec<CompressedHintsTransaction<T>>
    where
        T: TransactionHints<Key = K> + Send + Sync,
        I: IntoParallelIterator<Item = T>,
    {
        block
            .into_par_iter()
            .map(|tx| {
                let read_set = tx.read_set().map(|key| self.map_key(key)).collect();
                let write_set = tx.write_set().map(|key| self.map_key(key)).collect();
                let delta_set = tx.delta_set().map(|key| self.map_key(key)).collect();

                CompressedHintsTransaction {
                    original: tx,
                    read_set,
                    write_set,
                    delta_set,
                }
            })
            .collect()
    }
}

pub fn compress_transactions_in_parallel<T>(block: Vec<T>) -> Vec<CompressedHintsTransaction<T>>
where
    T: TransactionHints + Send + Sync,
    T::Key: Hash + Clone + Eq + Send + Sync,
{
    let mut compressor = ParallelTransactionCompressor::new();
    compressor.compress_transactions(block)
}
