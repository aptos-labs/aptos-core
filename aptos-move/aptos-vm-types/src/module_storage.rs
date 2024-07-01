// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::state_store::state_value::StateValueMetadata;
use bytes::Bytes;
use move_binary_format::{errors::PartialVMResult, CompiledModule};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr, metadata::Metadata};
use move_vm_runtime::module_storage::ModuleStorage;
use std::sync::Arc;

pub trait AptosModuleStorage: ModuleStorage {
    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Bytes>;

    fn fetch_module_state_value_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<StateValueMetadata>;
}

#[allow(dead_code)]
pub struct TemporaryModuleStorage {}

impl ModuleStorage for TemporaryModuleStorage {
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

impl AptosModuleStorage for TemporaryModuleStorage {
    fn fetch_module_bytes(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> PartialVMResult<Bytes> {
        todo!()
    }

    fn fetch_module_state_value_metadata(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> PartialVMResult<StateValueMetadata> {
        todo!()
    }
}
