// Copyright © Aptos Foundation

use aptos_types::{
    block_executor::partitioner::TxnIndex,
    transaction::analyzed_transaction::{AnalyzedTransaction, StorageLocation},
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

#[derive(Default, Clone, Debug)]
pub struct RWSet {
    // Represents a set of storage locations that are read by the transactions in this shard.
    read_set: Arc<HashSet<StorageLocation>>,
    // Represents a set of storage locations that are written by the transactions in this shard.
    write_set: Arc<HashSet<StorageLocation>>,
}

impl RWSet {
    pub fn new(txns: &[AnalyzedTransaction]) -> Self {
        let mut read_set = HashSet::new();
        let mut write_set = HashSet::new();
        for analyzed_txn in txns {
            for write_location in analyzed_txn.write_hints().iter() {
                write_set.insert(write_location.clone());
            }
            for read_location in analyzed_txn.read_hints().iter() {
                read_set.insert(read_location.clone());
            }
        }

        Self {
            read_set: Arc::new(read_set),
            write_set: Arc::new(write_set),
        }
    }

    pub fn has_write_lock(&self, location: &StorageLocation) -> bool {
        self.write_set.contains(location)
    }

    pub fn has_read_lock(&self, location: &StorageLocation) -> bool {
        self.read_set.contains(location)
    }

    pub fn has_read_or_write_lock(&self, location: &StorageLocation) -> bool {
        self.has_read_lock(location) || self.has_write_lock(location)
    }
}

#[derive(Default, Clone, Debug)]
/// Contains a list of storage location along with the maximum transaction index in this shard
/// that has taken a read/write lock on this storage location.  For example, if the chunk contains 3
/// transactions with read/write set as follows:
/// Txn 0: Write set: [A, B, C]
/// Txn 1: Write set: [A, B]
/// Txn 2: Write set: [A]
/// Then the WriteSetWithTxnIndex will be:
/// Write set: {A: 2, B: 1, C: 0}
/// Please note that the index is the global index which includes the offset of the shard.
pub struct WriteSetWithTxnIndex {
    write_set: Arc<HashMap<StorageLocation, TxnIndex>>,
}

impl WriteSetWithTxnIndex {
    // Creates a new dependency analysis object from a list of transactions. In this case, since the
    // transactions are frozen, we can set the maximum transaction index to the index of the last
    // transaction in the list.
    pub fn new(txns: &[AnalyzedTransaction], txn_index_offset: TxnIndex) -> Self {
        let mut write_set = HashMap::new();
        for (index, txn) in txns.iter().enumerate() {
            for write_location in txn.write_hints().iter() {
                write_set.insert(write_location.clone(), txn_index_offset + index);
            }
        }

        Self {
            write_set: Arc::new(write_set),
        }
    }

    pub fn has_write_lock(&self, location: &StorageLocation) -> bool {
        self.write_set.contains_key(location)
    }

    pub fn get_write_lock_txn_index(&self, location: &StorageLocation) -> TxnIndex {
        *self.write_set.get(location).unwrap()
    }
}
