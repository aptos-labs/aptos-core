// Copyright © Aptos Foundation

use aptos_block_executor::transaction_hints::TransactionHints;
use std::{collections::HashMap, hash::Hash};

pub type CompressedKey = u32;

#[derive(Clone, Default)]
pub struct CompressedHintsTransaction<T> {
    pub original: T,
    pub read_set: Vec<CompressedKey>,
    pub write_set: Vec<CompressedKey>,
    pub delta_set: Vec<CompressedKey>,
}

impl<T> TransactionHints for CompressedHintsTransaction<T> {
    type DeltaSetIter<'a> = std::slice::Iter<'a, Self::Key> where T: 'a;
    type Key = CompressedKey;
    type ReadSetIter<'a> = std::slice::Iter<'a, Self::Key> where T: 'a;
    type WriteSetIter<'a> = std::slice::Iter<'a, Self::Key> where T: 'a;

    fn read_set(&self) -> Self::ReadSetIter<'_> {
        self.read_set.iter()
    }

    fn write_set(&self) -> Self::WriteSetIter<'_> {
        self.write_set.iter()
    }

    fn delta_set(&self) -> Self::DeltaSetIter<'_> {
        self.delta_set.iter()
    }
}

pub struct TransactionCompressor<K> {
    key_mapping: HashMap<K, CompressedKey>,
    next_key: CompressedKey,
}

impl<K> TransactionCompressor<K> {
    pub fn new() -> Self {
        Self {
            key_mapping: HashMap::new(),
            next_key: 0,
        }
    }
}

impl<K> Default for TransactionCompressor<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Hash + Clone + Eq> TransactionCompressor<K> {
    pub fn map_key(&mut self, key: &K) -> CompressedKey {
        if let Some(&mapped_key) = self.key_mapping.get(key) {
            mapped_key
        } else {
            let mapped_key = self.next_key;
            self.next_key += 1;
            self.key_mapping.insert(key.clone(), mapped_key);
            mapped_key
        }
    }

    pub fn compress_transactions<T, I>(&mut self, block: I) -> Vec<CompressedHintsTransaction<T>>
    where
        T: TransactionHints<Key = K>,
        I: IntoIterator<Item = T>,
    {
        let mut res = vec![];

        for tx in block.into_iter() {
            let read_set = tx.read_set().map(|key| self.map_key(key)).collect();
            let write_set = tx.write_set().map(|key| self.map_key(key)).collect();
            let delta_set = tx.delta_set().map(|key| self.map_key(key)).collect();

            res.push(CompressedHintsTransaction {
                original: tx,
                read_set,
                write_set,
                delta_set,
            });
        }

        res
    }
}

pub fn compress_transactions<T>(block: Vec<T>) -> Vec<CompressedHintsTransaction<T>>
where
    T: TransactionHints,
    T::Key: Hash + Clone + Eq,
{
    let mut compressor = TransactionCompressor::new();
    compressor.compress_transactions(block)
}
