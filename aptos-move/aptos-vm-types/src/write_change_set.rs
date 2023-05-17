// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{change_set::ChangeSet, op::Op, vm_view::AptosVMView};
use aptos_aggregator::{
    delta_change_set::{deserialize, serialize, DeltaChangeSet, DeltaOp},
    module::AGGREGATOR_MODULE,
};
use aptos_types::{
    state_store::state_key::StateKey,
    write_set::{WriteSet, WriteSetMut},
};
use move_binary_format::errors::Location;
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

// TODO: This is almost 1-to-1 copy of DeltaOp::try_into_write_op. We must have it here
// to avoid cyclic dependency with AptosVMView.
fn try_into_write_op(
    delta_op: DeltaOp,
    state_view: &impl AptosVMView,
    state_key: &StateKey,
) -> anyhow::Result<Op<Vec<u8>>, VMStatus> {
    state_view
        .get_aggregator_value(state_key)
        .map_err(|_| VMStatus::Error(StatusCode::STORAGE_ERROR, None))
        .and_then(|maybe_bytes| {
            match maybe_bytes {
                Some(bytes) => {
                    let base = deserialize(&bytes);
                    delta_op
                        .apply_to(base)
                        .map_err(|partial_error| {
                            // If delta application fails, transform partial VM
                            // error into an appropriate VM status.
                            partial_error
                                .finish(Location::Module(AGGREGATOR_MODULE.clone()))
                                .into_vm_status()
                        })
                        .map(|result| Op::Modification(serialize(&result)))
                },
                // Something is wrong, the value to which we apply delta should
                // always exist. Guard anyway.
                None => Err(VMStatus::Error(StatusCode::STORAGE_ERROR, None)),
            }
        })
}

impl WriteChangeSet<Vec<u8>> {
    /// Consumes the delta change set and tries to materialize it. Returns a
    /// write set if materialization succeeds.
    pub fn from_deltas(
        deltas: DeltaChangeSet,
        state_view: &impl AptosVMView,
    ) -> anyhow::Result<Self, VMStatus> {
        let mut materialized_writes = Vec::with_capacity(deltas.len());

        for (state_key, delta_op) in deltas {
            let op = try_into_write_op(delta_op, state_view, &state_key)?;
            materialized_writes.push((state_key, op));
        }

        Ok(Self(ChangeSet::new(materialized_writes)))
    }
}
