// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::BlockExecutableTransaction as Transaction;
use std::{collections::HashSet, fmt};

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum InputOutputKey<K, T, I> {
    Resource(K),
    Group(K, T),
    DelayedField(I),
}

pub struct ReadWriteSummary<T: Transaction> {
    reads: HashSet<InputOutputKey<T::Key, T::Tag, T::Identifier>>,
    writes: HashSet<InputOutputKey<T::Key, T::Tag, T::Identifier>>,
}

impl<T: Transaction> ReadWriteSummary<T> {
    pub fn new(
        reads: HashSet<InputOutputKey<T::Key, T::Tag, T::Identifier>>,
        writes: HashSet<InputOutputKey<T::Key, T::Tag, T::Identifier>>,
    ) -> Self {
        Self { reads, writes }
    }

    pub fn conflicts_with_previous(&self, previous: &Self) -> bool {
        !self.reads.is_disjoint(&previous.writes)
    }

    pub fn collapse_resource_group_conflicts(self) -> Self {
        let collapse = |k: InputOutputKey<T::Key, T::Tag, T::Identifier>| match k {
            InputOutputKey::Resource(k) => InputOutputKey::Resource(k),
            InputOutputKey::Group(k, _) => InputOutputKey::Resource(k),
            InputOutputKey::DelayedField(id) => InputOutputKey::DelayedField(id),
        };
        Self {
            reads: self.reads.into_iter().map(collapse).collect(),
            writes: self.writes.into_iter().map(collapse).collect(),
        }
    }
}

impl<T: Transaction> fmt::Debug for ReadWriteSummary<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ReadWriteSummary")?;
        writeln!(f, "reads:")?;
        for read in &self.reads {
            writeln!(f, "    {:?}", read)?;
        }
        writeln!(f, "writes:")?;
        for write in &self.writes {
            writeln!(f, "    {:?}", write)?;
        }
        Ok(())
    }
}
