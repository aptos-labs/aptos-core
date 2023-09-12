// Copyright © Aptos Foundation

use std::collections::{HashMap, HashSet};
use std::slice::Iter;
use std::sync::Arc;
use aptos_mvhashmap::types::{END_TXN_INDEX, TxnIndex};

pub struct IndexHelper {
    pub txns: Arc<Vec<TxnIndex>>,
    pub local_rank_by_txn: HashMap<TxnIndex, usize>,
    pub txns_and_deps: Vec<TxnIndex>,
}

impl IndexHelper {
    pub fn new(indices_for_current_shard: Arc<Vec<TxnIndex>>) -> Self {
        let mut local_rank_by_index = HashMap::new();
        for (local_rank, index) in indices_for_current_shard.iter().enumerate() {
            local_rank_by_index.insert(*index, local_rank);
        }
        Self {
            txns: indices_for_current_shard,
            local_rank_by_txn: local_rank_by_index,
            txns_and_deps: Default::default(), //sharding todo
        }
    }

    pub fn num_txns(&self) -> usize {
        self.txns.len()
    }

    pub fn first_txn(&self) -> TxnIndex {
        self.txns.first().copied().unwrap_or(END_TXN_INDEX)
    }

    pub fn next_txn(&self, idx: TxnIndex) -> TxnIndex {
        let local_idx = self.local_rank_by_txn.get(&idx).copied().unwrap();
        self.txns.get(local_idx + 1).copied().unwrap_or(END_TXN_INDEX)
    }

    pub fn txns(&self) -> Iter<TxnIndex> {
        self.txns.iter()
    }

    pub fn txns_and_deps(&self) -> Iter<TxnIndex> {
        self.txns_and_deps.iter()
    }
}
