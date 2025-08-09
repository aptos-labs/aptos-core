// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    contract_event::ContractEvent, state_store::state_key::StateKey, write_set::WriteOpSize,
};
use aptos_vm_types::{
    change_set::{ChangeSetInterface, VMChangeSet, WriteOpInfo},
    module_and_script_storage::module_storage::AptosModuleStorage,
    module_write_set::ModuleWriteSet,
    resolver::ExecutorView,
    storage::change_set_configs::ChangeSetConfigs,
};
use move_binary_format::errors::PartialVMResult;
use move_core_types::vm_status::VMStatus;

#[derive(Clone)]
pub struct UserSessionChangeSet {
    change_set: VMChangeSet,
    module_write_set: ModuleWriteSet,
}

impl UserSessionChangeSet {
    pub(crate) fn new(
        change_set: VMChangeSet,
        module_write_set: ModuleWriteSet,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<Self, VMStatus> {
        let user_session_change_set = Self {
            change_set,
            module_write_set,
        };
        change_set_configs.check_change_set(&user_session_change_set)?;
        Ok(user_session_change_set)
    }

    pub(crate) fn unpack(self) -> (VMChangeSet, ModuleWriteSet) {
        (self.change_set, self.module_write_set)
    }
}

impl ChangeSetInterface for UserSessionChangeSet {
    fn num_write_ops(&self) -> usize {
        self.change_set.num_write_ops() + self.module_write_set.num_write_ops()
    }

    fn write_set_size_iter(&self) -> impl Iterator<Item = (&StateKey, WriteOpSize)> {
        self.change_set
            .write_set_size_iter()
            .chain(self.module_write_set.write_set_size_iter())
    }

    fn write_op_info_iter_mut<'a>(
        &'a mut self,
        executor_view: &'a dyn ExecutorView,
        module_storage: &'a impl AptosModuleStorage,
        fix_prev_materialized_size: bool,
    ) -> impl Iterator<Item = PartialVMResult<WriteOpInfo<'a>>> {
        self.change_set
            .write_op_info_iter_mut(executor_view, module_storage, fix_prev_materialized_size)
            .chain(self.module_write_set.write_op_info_iter_mut(module_storage))
    }

    fn events_iter(&self) -> impl Iterator<Item = &ContractEvent> {
        self.change_set.events_iter()
    }
}

#[derive(Clone)]
pub struct SystemSessionChangeSet {
    change_set: VMChangeSet,
}

impl SystemSessionChangeSet {
    pub(crate) fn new(
        change_set: VMChangeSet,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<Self, VMStatus> {
        let system_session_change_set = Self { change_set };
        change_set_configs.check_change_set(&system_session_change_set)?;
        Ok(system_session_change_set)
    }

    pub(crate) fn has_writes(&self) -> bool {
        self.change_set != VMChangeSet::empty()
    }

    pub(crate) fn empty() -> Self {
        Self {
            change_set: VMChangeSet::empty(),
        }
    }

    pub(crate) fn unpack(self) -> VMChangeSet {
        self.change_set
    }
}

impl ChangeSetInterface for SystemSessionChangeSet {
    fn num_write_ops(&self) -> usize {
        self.change_set.num_write_ops()
    }

    fn write_set_size_iter(&self) -> impl Iterator<Item = (&StateKey, WriteOpSize)> {
        self.change_set.write_set_size_iter()
    }

    fn write_op_info_iter_mut<'a>(
        &'a mut self,
        executor_view: &'a dyn ExecutorView,
        module_storage: &'a impl AptosModuleStorage,
        fix_prev_materialized_size: bool,
    ) -> impl Iterator<Item = PartialVMResult<WriteOpInfo<'a>>> {
        self.change_set.write_op_info_iter_mut(
            executor_view,
            module_storage,
            fix_prev_materialized_size,
        )
    }

    fn events_iter(&self) -> impl Iterator<Item = &ContractEvent> {
        self.change_set.events_iter()
    }
}
