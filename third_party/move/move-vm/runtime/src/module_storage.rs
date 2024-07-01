// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{errors::PartialVMResult, CompiledModule};
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, metadata::Metadata,
    value::MoveTypeLayout,
};
use move_vm_types::loaded_data::runtime_types::Type;
use std::sync::Arc;

pub trait ModuleStorage {
    fn check_module_exists(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<bool>;

    fn fetch_compiled_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Arc<CompiledModule>>;

    fn fetch_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<&[Metadata]>;

    fn fetch_module_size_in_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<usize>;

    fn fetch_module_immediate_dependencies(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Vec<(&AccountAddress, &IdentStr)>>;

    fn fetch_module_immediate_friends(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Vec<(&AccountAddress, &IdentStr)>>;

    fn fetch_type_layout(
        &self,
        _ty: &Type,
        _is_fully_annotated: bool,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        todo!()
    }
}

impl ModuleStorage for () {
    fn check_module_exists(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> PartialVMResult<bool> {
        todo!()
    }

    fn fetch_compiled_module(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> PartialVMResult<Arc<CompiledModule>> {
        todo!()
    }

    fn fetch_module_metadata(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> PartialVMResult<&[Metadata]> {
        todo!()
    }

    fn fetch_module_size_in_bytes(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> PartialVMResult<usize> {
        todo!()
    }

    fn fetch_module_immediate_dependencies(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> PartialVMResult<Vec<(&AccountAddress, &IdentStr)>> {
        todo!()
    }

    fn fetch_module_immediate_friends(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> PartialVMResult<Vec<(&AccountAddress, &IdentStr)>> {
        todo!()
    }
}
