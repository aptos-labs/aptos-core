// Copyright © Aptos Foundation

use aptos_types::{
    state_store::state_key::StateKey,
    transaction::analyzed_transaction::{AnalyzedTransaction, StorageLocation},
};

pub trait PTransaction {
    type Key;

    type ReadSetIter<'a>: Iterator<Item = &'a Self::Key>
    where
        Self: 'a;

    type WriteSetIter<'a>: Iterator<Item = &'a Self::Key>
    where
        Self: 'a;

    fn read_set(&self) -> Self::ReadSetIter<'_>;

    fn write_set(&self) -> Self::WriteSetIter<'_>;
}

impl PTransaction for AnalyzedTransaction {
    type Key = StateKey;
    type ReadSetIter<'a> =
        std::iter::Map<std::slice::Iter<'a, StorageLocation>, fn(&StorageLocation) -> &StateKey>;
    type WriteSetIter<'a> =
        std::iter::Map<std::slice::Iter<'a, StorageLocation>, fn(&StorageLocation) -> &StateKey>;

    fn read_set(&self) -> Self::ReadSetIter<'_> {
        self.read_hints().iter().map(StorageLocation::state_key)
    }

    fn write_set(&self) -> Self::WriteSetIter<'_> {
        self.write_hints().iter().map(StorageLocation::state_key)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Direction {
    Front = -1,
    Back = 1,
}
