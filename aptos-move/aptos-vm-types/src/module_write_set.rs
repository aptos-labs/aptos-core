// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::change_set::WriteOpInfo;
use aptos_types::{
    state_store::state_key::StateKey,
    write_set::{TransactionWrite, WriteOp, WriteOpSize},
};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{language_storage::ModuleId, vm_status::StatusCode};
use move_vm_runtime::ModuleStorage;
use std::collections::BTreeMap;

/// A write with a published module, also containing the information about its address and name.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ModuleWrite<V> {
    id: ModuleId,
    write_op: V,
}

impl<V: TransactionWrite> ModuleWrite<V> {
    /// Creates a new module write.
    pub fn new(id: ModuleId, write_op: V) -> Self {
        Self { id, write_op }
    }

    /// Returns the module identifier for the write.
    pub fn module_id(&self) -> &ModuleId {
        &self.id
    }

    /// Returns the mutable reference to the write for the published module.
    pub fn write_op_mut(&mut self) -> &mut V {
        &mut self.write_op
    }

    /// Returns the reference to the write for the published module.
    pub fn write_op(&self) -> &V {
        &self.write_op
    }

    /// Returns the write for the published module.
    pub fn into_write_op(self) -> V {
        self.write_op
    }

    /// Returns the module identifier with the corresponding operation.
    pub fn unpack(self) -> (ModuleId, V) {
        (self.id, self.write_op)
    }
}

/// Represents a set of new modules published by a single transaction.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ModuleWriteSet {
    writes: BTreeMap<StateKey, ModuleWrite<WriteOp>>,
}

impl ModuleWriteSet {
    pub fn empty() -> Self {
        Self {
            writes: BTreeMap::new(),
        }
    }

    pub fn new(writes: BTreeMap<StateKey, ModuleWrite<WriteOp>>) -> Self {
        Self { writes }
    }

    pub fn into_write_ops(self) -> impl IntoIterator<Item = (StateKey, WriteOp)> {
        self.writes.into_iter().map(|(k, w)| (k, w.into_write_op()))
    }

    pub fn writes(&self) -> &BTreeMap<StateKey, ModuleWrite<WriteOp>> {
        &self.writes
    }

    pub fn num_write_ops(&self) -> usize {
        self.writes.len()
    }

    pub fn write_set_size_iter(&self) -> impl Iterator<Item = (&StateKey, WriteOpSize)> {
        self.writes
            .iter()
            .map(|(k, v)| (k, v.write_op().write_op_size()))
    }

    pub fn write_op_info_iter_mut<'a>(
        &'a mut self,
        module_storage: &'a impl ModuleStorage,
    ) -> impl Iterator<Item = PartialVMResult<WriteOpInfo<'a>>> {
        self.writes.iter_mut().map(move |(key, write)| {
            let prev_size = module_storage
                .fetch_module_size_in_bytes(write.module_id())
                .map_err(|e| e.to_partial())?
                .unwrap_or(0) as u64;
            Ok(WriteOpInfo {
                key,
                op_size: write.write_op().write_op_size(),
                prev_size,
                metadata_mut: write.write_op_mut().metadata_mut(),
            })
        })
    }

    pub fn is_empty(&self) -> bool {
        self.writes().is_empty()
    }

    pub fn is_empty_or_invariant_violation(&self) -> PartialVMResult<()> {
        if !self.is_empty() {
            return Err(PartialVMError::new(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            ));
        }
        Ok(())
    }
}
