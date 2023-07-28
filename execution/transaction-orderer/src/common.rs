// Copyright © Aptos Foundation

use aptos_types::state_store::state_key::StateKey;
use aptos_types::transaction::analyzed_transaction::{AnalyzedTransaction, StorageLocation};

pub trait PTransaction {
    type Key;

    // TODO: return an iterator for a potentially more efficient implementation
    fn read_set(&self) -> Vec<Self::Key>;

    fn write_set(&self) -> Vec<Self::Key>;
}

impl PTransaction for AnalyzedTransaction {
    type Key = StateKey;

    fn read_set(&self) -> Vec<StateKey> {
        let read_set_iter = self.read_hints()
            .iter()
            .map(StorageLocation::state_key)
            .cloned();
        let write_set_iter = self.write_hints()
            .iter()
            .map(StorageLocation::state_key)
            .cloned();
        read_set_iter.chain(write_set_iter).collect()
    }

    fn write_set(&self) -> Vec<StateKey> {
        self.write_hints()
            .iter()
            .map(StorageLocation::state_key)
            .cloned()
            .collect()
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Direction {
    Front = -1,
    Back = 1,
}
