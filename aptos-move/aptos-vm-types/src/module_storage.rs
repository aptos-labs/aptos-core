// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::module_write_set::ModuleWriteSet;
use aptos_types::{state_store::state_key::StateKey, vm::module_write_op::ModuleWrite};
use bytes::Bytes;
use move_binary_format::{access::ModuleAccess, errors::PartialVMResult};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    metadata::Metadata,
};
use move_vm_types::resolver::ModuleResolver;

/// Module storage implementation which can be used to stash module changes on
/// top of existing module storage back-end. For example, it is used performing
/// module initialization for published modules.
pub struct TemporaryModuleStorage<'m, M: ModuleResolver> {
    module_write_set: &'m ModuleWriteSet,
    module_storage: &'m M,
}

impl<'m, M: ModuleResolver> TemporaryModuleStorage<'m, M> {
    /// Creates a new temporary storage, capturing the reference to temporary module
    /// changes that will be stashed on top of existing storage.
    pub fn new(module_write_set: &'m ModuleWriteSet, module_storage: &'m M) -> Self {
        Self {
            module_write_set,
            module_storage,
        }
    }
}

impl<'m, M: ModuleResolver> ModuleResolver for TemporaryModuleStorage<'m, M> {
    fn check_module_exists(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<bool> {
        let state_key = StateKey::module(address, module_name);
        match self.module_write_set.module_write_ops().get(&state_key) {
            Some(_) => Ok(true),
            None => self
                .module_storage
                .check_module_exists(address, module_name),
        }
    }

    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Option<Bytes>> {
        let state_key = StateKey::module(address, module_name);
        Ok(
            match self.module_write_set.module_write_ops().get(&state_key) {
                Some(write_op) => Some(write_op.serialized_module_bytes().clone()),
                None => self
                    .module_storage
                    .fetch_module_bytes(address, module_name)?,
            },
        )
    }

    fn fetch_module_size_in_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<usize> {
        let state_key = StateKey::module(address, module_name);
        Ok(
            match self.module_write_set.module_write_ops().get(&state_key) {
                Some(write_op) => write_op.module_size_in_bytes(),
                None => self
                    .module_storage
                    .fetch_module_size_in_bytes(address, module_name)?,
            },
        )
    }

    fn fetch_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Vec<Metadata>> {
        let state_key = StateKey::module(address, module_name);
        Ok(
            match self.module_write_set.module_write_ops().get(&state_key) {
                Some(write_op) => write_op.compiled_module().metadata.clone(),
                None => self
                    .module_storage
                    .fetch_module_metadata(address, module_name)?,
            },
        )
    }

    fn fetch_module_immediate_dependencies(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Vec<(AccountAddress, Identifier)>> {
        let state_key = StateKey::module(address, module_name);
        Ok(
            match self.module_write_set.module_write_ops().get(&state_key) {
                Some(write_op) => write_op
                    .compiled_module()
                    .immediate_dependencies()
                    .into_iter()
                    .map(|module_id| (*module_id.address(), module_id.name().to_owned()))
                    .collect(),
                None => self
                    .module_storage
                    .fetch_module_immediate_dependencies(address, module_name)?,
            },
        )
    }

    fn fetch_module_immediate_friends(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Vec<(AccountAddress, Identifier)>> {
        let state_key = StateKey::module(address, module_name);
        Ok(
            match self.module_write_set.module_write_ops().get(&state_key) {
                Some(write_op) => write_op
                    .compiled_module()
                    .immediate_friends()
                    .into_iter()
                    .map(|module_id| (*module_id.address(), module_id.name().to_owned()))
                    .collect(),
                None => self
                    .module_storage
                    .fetch_module_immediate_friends(address, module_name)?,
            },
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use claims::assert_ok_eq;
    use move_vm_test_utils::InMemoryStorage;

    #[test]
    fn test_temporary_module_storage_module_existence() {
        let module_storage = InMemoryStorage::new();
        let module_write_set = ModuleWriteSet::empty();
        let tmp_module_storage = TemporaryModuleStorage::new(&module_write_set, &module_storage);

        let result = tmp_module_storage
            .check_module_exists(&AccountAddress::ONE, IdentStr::new("foo").unwrap());
        assert_ok_eq!(result, false);

        // FIXME(George): Add more tests here.
    }
}
