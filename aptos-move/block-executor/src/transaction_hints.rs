// Copyright © Aptos Foundation

use crate::task::{IntoTransaction, Transaction};
use aptos_types::{
    state_store::state_key::StateKey,
    transaction::analyzed_transaction::{AnalyzedTransaction, StorageLocation},
};

/// Provide hints about the keys accessed by a transaction.
pub trait TransactionHints {
    type Key;

    type ReadSetIter<'a>: Iterator<Item = &'a Self::Key>
    where
        Self: 'a;

    type WriteSetIter<'a>: Iterator<Item = &'a Self::Key>
    where
        Self: 'a;

    type DeltaSetIter<'a>: Iterator<Item = &'a Self::Key>
    where
        Self: 'a;

    fn read_set(&self) -> Self::ReadSetIter<'_>;

    fn write_set(&self) -> Self::WriteSetIter<'_>;

    fn delta_set(&self) -> Self::DeltaSetIter<'_>;
}

/// A transaction with hints about the keys accessed by it.
#[derive(Clone, Debug)]
pub struct TransactionWithHints<T: Transaction> {
    pub transaction: T,
    pub read_set: Vec<T::Key>,
    pub write_set: Vec<T::Key>,
    pub delta_set: Vec<T::Key>,
}

impl<T: Transaction> IntoTransaction for TransactionWithHints<T> {
    type Txn = T;

    fn into_transaction(self) -> Self::Txn {
        self.transaction
    }
}

impl<T: Transaction> TransactionHints for TransactionWithHints<T> {
    type DeltaSetIter<'a> = std::slice::Iter<'a, T::Key>;
    type Key = T::Key;
    type ReadSetIter<'a> = std::slice::Iter<'a, T::Key>;
    type WriteSetIter<'a> = std::slice::Iter<'a, T::Key>;

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

impl TransactionHints for AnalyzedTransaction {
    type DeltaSetIter<'a> = std::iter::Empty<&'a StateKey>;
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

    fn delta_set(&self) -> Self::DeltaSetIter<'_> {
        std::iter::empty()
    }
}
