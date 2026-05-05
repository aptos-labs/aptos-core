// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::BlockExecutableTransaction as Transaction;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{
    collections::HashSet,
    fmt::{self, Debug},
};

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum InputOutputKey<K, T> {
    Resource(K),
    Group(K, T),
    DelayedField(DelayedFieldID),
}

pub struct ReadWriteSummary<T: Transaction> {
    pub reads: HashSet<InputOutputKey<T::Key, T::Tag>>,
    pub writes: HashSet<InputOutputKey<T::Key, T::Tag>>,
}

impl<T: Transaction> ReadWriteSummary<T> {
    pub fn new(
        reads: HashSet<InputOutputKey<T::Key, T::Tag>>,
        writes: HashSet<InputOutputKey<T::Key, T::Tag>>,
    ) -> Self {
        Self { reads, writes }
    }

    pub fn conflicts_with_previous(&self, previous: &Self) -> bool {
        !self.reads.is_disjoint(&previous.writes)
    }

    pub fn find_conflicts<'a>(
        &'a self,
        previous: &'a Self,
    ) -> HashSet<&'a InputOutputKey<T::Key, T::Tag>> {
        self.reads
            .intersection(&previous.writes)
            .collect::<HashSet<_>>()
    }

    pub fn collapse_resource_group_conflicts(self) -> Self {
        let collapse = |k: InputOutputKey<T::Key, T::Tag>| match k {
            InputOutputKey::Resource(k) => InputOutputKey::Resource(k),
            InputOutputKey::Group(k, _) => InputOutputKey::Resource(k),
            InputOutputKey::DelayedField(id) => InputOutputKey::DelayedField(id),
        };
        Self {
            reads: self.reads.into_iter().map(collapse).collect(),
            writes: self.writes.into_iter().map(collapse).collect(),
        }
    }

    pub fn keys_written(&self) -> impl Iterator<Item = &T::Key> {
        Self::keys_except_delayed_fields(self.writes.iter())
    }

    pub fn keys_read(&self) -> impl Iterator<Item = &T::Key> {
        Self::keys_except_delayed_fields(self.reads.iter())
    }

    fn keys_except_delayed_fields<'a>(
        keys: impl Iterator<Item = &'a InputOutputKey<T::Key, T::Tag>>,
    ) -> impl Iterator<Item = &'a T::Key> {
        keys.filter_map(|k| match k {
            InputOutputKey::Resource(key) | InputOutputKey::Group(key, _) => Some(key),
            InputOutputKey::DelayedField(_) => None,
        })
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
