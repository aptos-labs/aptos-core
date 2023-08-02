// Copyright © Aptos Foundation

use crate::common::PTransaction;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

pub type CompressedKey = u32;

#[derive(Default)]
pub struct CompressedPTransactionInner<T> {
    pub original: Box<T>,
    pub read_set: Vec<CompressedKey>,
    pub write_set: Vec<CompressedKey>,
}

pub type CompressedPTransaction<T> = Rc<CompressedPTransactionInner<T>>;

impl<T> PTransaction for CompressedPTransaction<T> {
    type Key = CompressedKey;
    type ReadSetIter<'a> = std::slice::Iter<'a, Self::Key> where T: 'a;
    type WriteSetIter<'a> = std::slice::Iter<'a, Self::Key> where T: 'a;

    fn read_set<'a>(&'a self) -> Self::ReadSetIter<'a> {
        self.read_set.iter()
    }

    fn write_set<'a>(&'a self) -> Self::WriteSetIter<'a> {
        self.write_set.iter()
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

    pub fn compress_transactions<T, I>(&mut self, block: I) -> Vec<CompressedPTransaction<T>>
    where
        T: PTransaction<Key = K>,
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
}

pub fn compress_transactions<T>(block: Vec<T>) -> Vec<CompressedPTransaction<T>>
where
    T: PTransaction,
    T::Key: Hash + Clone + Eq,
{
    let mut compressor = TransactionCompressor::new();
    compressor.compress_transactions(block)
}
