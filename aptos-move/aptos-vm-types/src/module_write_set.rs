// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{change_set::WriteOpInfo, resolver::ExecutorView};
use aptos_types::{
    state_store::state_key::StateKey,
    vm::module_write_op::ModuleWriteOp,
    write_set::{WriteOp, WriteOpSize},
};
use move_binary_format::errors::PartialVMResult;
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ModuleWriteSet {
    module_write_ops: BTreeMap<StateKey, ModuleWriteOp>,
}

impl ModuleWriteSet {
    pub fn empty() -> Self {
        Self {
            module_write_ops: BTreeMap::new(),
        }
    }

    pub fn new(module_write_ops: BTreeMap<StateKey, ModuleWriteOp>) -> Self {
        Self { module_write_ops }
    }

    pub fn into_write_ops(self) -> impl IntoIterator<Item = (StateKey, WriteOp)> {
        self.module_write_ops
            .into_iter()
            .map(|(k, w)| (k, w.into_write_op()))
    }

    pub fn module_write_ops(&self) -> &BTreeMap<StateKey, ModuleWriteOp> {
        &self.module_write_ops
    }

    pub fn module_addresses_and_names(&self) -> impl Iterator<Item = (&AccountAddress, &IdentStr)> {
        self.module_write_ops.values().map(|w| {
            let compiled_module = w.compiled_module();
            (compiled_module.self_addr(), compiled_module.self_name())
        })
    }

    pub fn num_write_ops(&self) -> usize {
        self.module_write_ops.len()
    }

    pub fn write_set_size_iter(&self) -> impl Iterator<Item = (&StateKey, WriteOpSize)> {
        self.module_write_ops
            .iter()
            .map(|(k, v)| (k, v.write_op_size()))
    }

    pub fn write_op_info_iter_mut<'a>(
        &'a mut self,
        executor_view: &'a dyn ExecutorView,
    ) -> impl Iterator<Item = PartialVMResult<WriteOpInfo>> {
        self.module_write_ops.iter_mut().map(|(key, op)| {
            let compiled_module = op.compiled_module();
            let address = compiled_module.self_addr();
            let module_name = compiled_module.self_name();

            let prev_size = if executor_view.check_module_exists(address, module_name)? {
                executor_view.fetch_module_size_in_bytes(address, module_name)? as u64
            } else {
                0
            };
            Ok(WriteOpInfo {
                key,
                op_size: op.write_op_size(),
                prev_size,
                metadata_mut: op.get_metadata_mut(),
            })
        })
    }

    pub fn has_writes_to_special_address(&self) -> bool {
        self.module_addresses_and_names()
            .any(|(address, _)| address.is_special())
    }
}
