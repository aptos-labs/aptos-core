// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{change_set::ChangeSet, op::Op};
use aptos_aggregator::delta_change_set::DeltaChangeSet;
use aptos_state_view::StateView;
use aptos_types::{
    state_store::state_key::StateKey,
    write_set::{WriteSet, WriteSetMut},
};
use move_core_types::vm_status::{StatusCode, VMStatus};
use move_vm_types::types::Store;
use std::{
    collections::btree_map::{
        Entry::{Occupied, Vacant},
        IntoIter,
    },
    ops::{Deref, DerefMut},
};

/// All writes produced by the VM.
#[derive(Debug, Clone)]
pub struct WriteChangeSet<T: Store>(ChangeSet<Op<T>>);

impl<T: Store> Deref for WriteChangeSet<T> {
    type Target = ChangeSet<Op<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Store> DerefMut for WriteChangeSet<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Store> IntoIterator for WriteChangeSet<T> {
    type IntoIter = IntoIter<StateKey, Op<T>>;
    type Item = (StateKey, Op<T>);

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T: Store> WriteChangeSet<T> {
    pub fn new(writes: impl IntoIterator<Item = (StateKey, Op<T>)>) -> Self {
        Self(ChangeSet::new(writes))
    }

    pub fn empty() -> Self {
        Self(ChangeSet::empty())
    }

    /// Converts the set of writes produced by the VM into storage-friendly
    /// write set containing blobs. Returns an error if serialization of one
    /// of the writes fails.
    pub fn into_write_set(self) -> anyhow::Result<WriteSet, VMStatus> {
        let mut write_ops = Vec::with_capacity(self.0.len());
        for (key, op) in self {
            let write_op = op
                .into_write_op()
                .ok_or(VMStatus::Error(StatusCode::INTERNAL_TYPE_ERROR, None))?;
            write_ops.push((key, write_op));
        }
        WriteSetMut::new(write_ops)
            .freeze()
            .map_err(|_| VMStatus::Error(StatusCode::DATA_FORMAT_ERROR, None))
    }

    /// Merges two sets of writes. Returns an error if an error occurred
    /// while squashing the writes.
    pub fn extend_with_writes(
        &mut self,
        writes: impl IntoIterator<Item = (StateKey, Op<T>)>,
    ) -> anyhow::Result<()> {
        for (key, op) in writes {
            match self.0.entry(key) {
                Occupied(mut entry) => {
                    if !Op::squash(entry.get_mut(), op)? {
                        entry.remove();
                    }
                },
                Vacant(entry) => {
                    entry.insert(op);
                },
            }
        }
        Ok(())
    }
}

impl WriteChangeSet<Vec<u8>> {
    /// Consumes the delta change set and tries to materialize it. Returns a
    /// write set if materialization succeeds.
    pub fn from_deltas(
        deltas: DeltaChangeSet,
        state_view: &impl StateView,
    ) -> anyhow::Result<Self, VMStatus> {
        let materialized_writes = deltas.take(state_view)?;
        Ok(Self(ChangeSet::new(
            materialized_writes
                .into_iter()
                .map(|(k, w)| (k, Op::from_write_op(w))),
        )))
    }
}
