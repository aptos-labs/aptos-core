// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::change_set::ChangeSet;
use aptos_aggregator::delta_change_set::DeltaChangeSet;
use aptos_state_view::StateView;
use aptos_types::{
    state_store::state_key::StateKey,
    write_set::{WriteOp, WriteSet, WriteSetMut},
};
use move_core_types::vm_status::{StatusCode, VMStatus};
use std::{
    collections::btree_map::{
        Entry::{Occupied, Vacant},
        IntoIter,
    },
    ops::{Deref, DerefMut},
};

/// All writes produced by the VM.
#[derive(Debug, Clone)]
pub struct WriteChangeSet(ChangeSet<WriteOp>);

impl Deref for WriteChangeSet {
    type Target = ChangeSet<WriteOp>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WriteChangeSet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for WriteChangeSet {
    type IntoIter = IntoIter<StateKey, WriteOp>;
    type Item = (StateKey, WriteOp);

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl WriteChangeSet {
    pub fn new(write_set: WriteSet) -> Self {
        Self(ChangeSet::new(write_set))
    }

    pub fn empty() -> Self {
        Self(ChangeSet::empty())
    }

    /// Consumes the delta change set and tries to materialize it. Returns a
    /// write set if materialization succeeds.
    pub fn from_deltas(
        deltas: DeltaChangeSet,
        state_view: &impl StateView,
    ) -> anyhow::Result<Self, VMStatus> {
        let materialized_writes = deltas.take(state_view)?;
        Ok(Self(ChangeSet::new(materialized_writes)))
    }

    /// Converts the set of writes produced by the VM into storage-friendly
    /// write set containing blobs. Returns an error if serialization of one
    /// of the writes fails.
    pub fn into_write_set(self) -> anyhow::Result<WriteSet, VMStatus> {
        let mut write_ops = Vec::with_capacity(self.0.len());
        for (key, write) in self {
            write_ops.push((key, write));
        }
        Ok(WriteSetMut::new(write_ops)
            .freeze()
            .map_err(|_| VMStatus::Error(StatusCode::DATA_FORMAT_ERROR, None))?)
    }

    /// Merges two sets of writes. Returns an error if an error occurred
    /// while squashing the writes.
    pub fn extend_with_writes(
        &mut self,
        writes: impl IntoIterator<Item = (StateKey, WriteOp)>,
    ) -> anyhow::Result<()> {
        for (key, write) in writes {
            match self.0.entry(key) {
                Occupied(mut entry) => {
                    if !WriteOp::squash(entry.get_mut(), write)? {
                        entry.remove();
                    }
                },
                Vacant(entry) => {
                    entry.insert(write);
                },
            }
        }
        Ok(())
    }
}
