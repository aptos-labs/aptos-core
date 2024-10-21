// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

use std::iter::{Chain, Map};
use std::slice::Iter;
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

    fn get_id(&self) -> usize {
        0
    }
}

impl PTransaction for AnalyzedTransaction {
    type Key = StateKey;
    type ReadSetIter<'a> =
        Map<Chain<Iter<'a, StorageLocation>, Iter<'a, StorageLocation>>, fn(&StorageLocation) -> &StateKey>;
    type WriteSetIter<'a> =
        Map<Iter<'a, StorageLocation>, fn(&StorageLocation) -> &StateKey>;

    fn read_set(&self) -> Self::ReadSetIter<'_> {
        self.read_hints().iter().chain(self.write_hints().iter()).map(StorageLocation::state_key)
    }

    fn write_set(&self) -> Self::WriteSetIter<'_> {
        self.write_hints().iter().map(StorageLocation::state_key)
    }

    fn get_id(&self) -> usize {
        self.id
    }
}
