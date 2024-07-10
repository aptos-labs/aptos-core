// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{change_set::WriteOpInfo, resolver::ExecutorView};
use aptos_types::{
    state_store::state_key::StateKey,
    write_set::{TransactionWrite, WriteOp, WriteOpSize},
};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::vm_status::StatusCode;
use std::collections::BTreeMap;

#[must_use]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ModuleWriteSet {
    // True if there are write ops which write to 0x1, etc. A special flag
    // is used for performance reasons, as otherwise we would need traverse
    // the write ops and deserializes access paths.
    has_writes_to_special_address: bool,
    write_ops: BTreeMap<StateKey, WriteOp>,
}

impl ModuleWriteSet {
    pub fn empty() -> Self {
        Self {
            has_writes_to_special_address: false,
            write_ops: BTreeMap::new(),
        }
    }

    pub fn new(
        has_writes_to_special_address: bool,
        write_ops: BTreeMap<StateKey, WriteOp>,
    ) -> Self {
        Self {
            has_writes_to_special_address,
            write_ops,
        }
    }

    pub fn into_write_ops(self) -> impl IntoIterator<Item = (StateKey, WriteOp)> {
        self.write_ops.into_iter()
    }

    pub fn write_ops(&self) -> &BTreeMap<StateKey, WriteOp> {
        &self.write_ops
    }

    pub fn num_write_ops(&self) -> usize {
        self.write_ops.len()
    }

    pub fn write_set_size_iter(&self) -> impl Iterator<Item = (&StateKey, WriteOpSize)> {
        self.write_ops.iter().map(|(k, v)| (k, v.write_op_size()))
    }

    pub fn write_op_info_iter_mut<'a>(
        &'a mut self,
        executor_view: &'a dyn ExecutorView,
    ) -> impl Iterator<Item = PartialVMResult<WriteOpInfo>> {
        self.write_ops.iter_mut().map(|(key, op)| {
            Ok(WriteOpInfo {
                key,
                op_size: op.write_op_size(),
                prev_size: executor_view.get_module_state_value_size(key)?.unwrap_or(0),
                metadata_mut: op.get_metadata_mut(),
            })
        })
    }

    pub fn has_writes_to_special_address(&self) -> bool {
        self.has_writes_to_special_address
    }

    pub fn is_empty_or_invariant_violation(&self) -> PartialVMResult<()> {
        if !self.write_ops().is_empty() {
            return Err(PartialVMError::new(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            ));
        }
        Ok(())
    }
}
