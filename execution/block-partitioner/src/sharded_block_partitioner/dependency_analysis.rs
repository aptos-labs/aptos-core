// Copyright © Aptos Foundation

use crate::types::TxnIndex;
use aptos_types::transaction::analyzed_transaction::{AnalyzedTransaction, StorageLocation};
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
            for write_location in analyzed_txn.write_set().iter() {
                write_set.insert(write_location.clone());
            }
            for read_location in analyzed_txn.read_set().iter() {
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
/// Txn 0: Read set: [A, B, C]
/// Txn 1: Read set: [A, B]
/// Txn 2: Read set: [A]
/// Then the RWSetWithTxnIndex will be:
/// Read set: {A: 2, B: 1, C: 0}
/// Similar applies for the write set.
/// Please note that the index is the global index which includes the offset of the shard.
pub struct RWSetWithTxnIndex {
    read_set: Arc<HashMap<StorageLocation, TxnIndex>>,
    write_set: Arc<HashMap<StorageLocation, TxnIndex>>,
}

impl RWSetWithTxnIndex {
    // Creates a new dependency analysis object from a list of transactions. In this case, since the
    // transactions are frozen, we can set the maximum transaction index to the index of the last
    // transaction in the list.
    pub fn new(txns: &[AnalyzedTransaction], txn_index_offset: TxnIndex) -> Self {
        let mut read_set = HashMap::new();
        let mut write_set = HashMap::new();
        for (index, txn) in txns.iter().enumerate() {
            for write_location in txn.write_set().iter() {
                write_set.insert(write_location.clone(), txn_index_offset + index);
            }
            for read_location in txn.read_set().iter() {
                read_set.insert(read_location.clone(), txn_index_offset + index);
            }
        }

        Self {
            read_set: Arc::new(read_set),
            write_set: Arc::new(write_set),
        }
    }

    pub fn has_write_lock(&self, location: &StorageLocation) -> bool {
        self.write_set.contains_key(location)
    }

    pub fn has_read_lock(&self, location: &StorageLocation) -> bool {
        self.read_set.contains_key(location)
    }

    pub fn get_write_lock_txn_index(&self, location: &StorageLocation) -> TxnIndex {
        *self.write_set.get(location).unwrap()
    }

    pub fn get_read_lock_txn_index(&self, location: &StorageLocation) -> TxnIndex {
        *self.read_set.get(location).unwrap()
    }
}
