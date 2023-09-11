// Copyright © Aptos Foundation

use crate::common::PTransaction;
use std::hash::Hash;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::SeqCst;
use dashmap::DashMap;
use crate::transaction_compressor::CompressedKey;
use rayon::prelude::*;

#[derive(Clone)]
pub struct ParallelCompressedPTransaction<T> {
    pub original: Box<T>,
    pub read_set: Vec<CompressedKey>,
    pub write_set: Vec<CompressedKey>,
}

impl<T> PTransaction for ParallelCompressedPTransaction<T> {
    type Key = CompressedKey;
    type ReadSetIter<'a> = std::slice::Iter<'a, Self::Key> where T: 'a;
    type WriteSetIter<'a> = std::slice::Iter<'a, Self::Key> where T: 'a;

    fn read_set(&self) -> Self::ReadSetIter<'_> {
        self.read_set.iter()
    }

    fn write_set(&self) -> Self::WriteSetIter<'_> {
        self.write_set.iter()
    }
}

pub struct ParallelTransactionCompressor<K> {
    key_mapping: DashMap<K, CompressedKey>,
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
            *self.key_mapping.entry(key.clone())
                .or_insert(mapped_key)
                .value()
        }
    }

    pub fn compress_transactions<T, I>(&mut self, block: I) -> Vec<ParallelCompressedPTransaction<T>>
    where
        T: PTransaction<Key = K> + Send,
        I: IntoParallelIterator<Item = T>,
    {
        block.into_par_iter().map(|tx| {
            let read_set = tx.read_set().map(|key| self.map_key(key)).collect();

            let write_set = tx.write_set().map(|key| self.map_key(key)).collect();

            ParallelCompressedPTransaction {
                original: Box::new(tx),
                read_set,
                write_set,
            }
        }).collect()
    }
}

pub fn compress_transactions_in_parallel<T>(block: Vec<T>) -> Vec<ParallelCompressedPTransaction<T>>
where
    T: PTransaction + Send,
    T::Key: Hash + Clone + Eq + Send + Sync,
{
    let mut compressor = ParallelTransactionCompressor::new();
    compressor.compress_transactions(block)
}
