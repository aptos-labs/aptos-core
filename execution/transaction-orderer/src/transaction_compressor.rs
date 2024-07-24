// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

use crate::common::PTransaction;
use std::{collections::HashMap, hash::Hash, rc::Rc};
use std::fmt::Debug;

pub type CompressedKey = u32;

#[derive(Debug, Default)]
pub struct CompressedPTransactionInner<T: Debug> {
    pub original: Box<T>,
    pub read_set: Vec<CompressedKey>,
    pub write_set: Vec<CompressedKey>,
}

pub type CompressedPTransaction<T> = Rc<CompressedPTransactionInner<T>>;

impl<T: Debug> PTransaction for CompressedPTransaction<T> {
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

pub struct TransactionCompressor<K> {
    key_mapping: HashMap<K, CompressedKey>,
    uncompressed_keys: Vec<K>,
}

impl<K> TransactionCompressor<K> {
    pub fn new() -> Self {
        Self {
            key_mapping: HashMap::new(),
            uncompressed_keys: vec![],
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
            let mapped_key = self.uncompressed_keys.len() as u32;
            self.key_mapping.insert(key.clone(), mapped_key);
            self.uncompressed_keys.push(key.clone());
            mapped_key
        }
    }

    pub fn compress_transactions<T, I>(&mut self, block: I) -> Vec<CompressedPTransaction<T>>
    where
        T: Debug + PTransaction<Key = K>,
        I: IntoIterator<Item = T>,
    {
        let mut res = vec![];

        for tx in block.into_iter() {
            let read_set = tx.read_set().map(|key| self.map_key(key)).collect();

            let write_set = tx.write_set().map(|key| self.map_key(key)).collect();

            res.push(Rc::new(CompressedPTransactionInner {
                original: Box::new(tx),
                read_set,
                write_set,
            }));
        }

        res
    }

    pub fn uncompressed_key(&self, compressed_key: usize) -> &K {
        &self.uncompressed_keys[compressed_key]
    }
}

pub fn compress_transactions<T: Debug>(block: Vec<T>) -> (Vec<CompressedPTransaction<T>>, TransactionCompressor<T::Key>)
where
    T: PTransaction,
    T::Key: Hash + Clone + Eq,
{
    let mut compressor = TransactionCompressor::new();
    let txns = compressor.compress_transactions(block);
    (txns, compressor)
}
