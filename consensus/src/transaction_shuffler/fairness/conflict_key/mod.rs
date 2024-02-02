// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_shuffler::fairness::TxnIdx;
use std::{collections::HashMap, hash::Hash};

pub(crate) mod entry_fun;
pub(crate) mod entry_fun_module;
pub(crate) mod txn_sender;

#[cfg(test)]
pub(crate) mod test_utils;

pub(crate) trait ConflictKey<Txn>: Eq + Hash + PartialEq {
    fn extract_from(txn: &Txn) -> Self;

    fn conflict_exempt(&self) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ConflictKeyId(usize);

impl ConflictKeyId {
    pub fn as_idx(&self) -> usize {
        self.0
    }
}

#[derive(Debug)]
pub(crate) struct ConflictKeyRegistry {
    id_by_txn: Vec<ConflictKeyId>,
    is_exempt_by_id: Vec<bool>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct MapByKeyId<T> {
    inner: Vec<T>,
}

impl<T: Default> MapByKeyId<T> {
    pub fn new(size: usize) -> Self {
        let mut inner = Vec::with_capacity(size);
        inner.resize_with(size, Default::default);

        Self { inner }
    }

    pub fn get(&self, key_id: ConflictKeyId) -> &T {
        &self.inner[key_id.as_idx()]
    }

    pub fn get_mut(&mut self, key_id: ConflictKeyId) -> &mut T {
        &mut self.inner[key_id.as_idx()]
    }
}

impl ConflictKeyRegistry {
    pub fn build<K: ConflictKey<Txn>, Txn>(txns: &[Txn]) -> Self
    where
        K: ConflictKey<Txn>,
    {
        let mut registry = HashMap::<K, ConflictKeyId>::new();
        let mut is_exempt_by_id = Vec::new();

        let id_by_txn = txns
            .iter()
            .map(|txn| {
                let key = K::extract_from(txn);
                *registry.entry(key).or_insert_with_key(|key| {
                    is_exempt_by_id.push(key.conflict_exempt());
                    ConflictKeyId(is_exempt_by_id.len() - 1)
                })
            })
            .collect();

        Self {
            id_by_txn,
            is_exempt_by_id,
        }
    }

    fn num_ids(&self) -> usize {
        self.is_exempt_by_id.len()
    }

    pub fn num_txns(&self) -> usize {
        self.id_by_txn.len()
    }

    pub fn new_map_by_id<T: Default>(&self) -> MapByKeyId<T> {
        MapByKeyId::new(self.num_ids())
    }

    pub fn key_id_for_txn(&self, txn_idx: TxnIdx) -> ConflictKeyId {
        self.id_by_txn[txn_idx]
    }

    pub fn is_conflict_exempt(&self, key_id: ConflictKeyId) -> bool {
        self.is_exempt_by_id[key_id.0]
    }
}
