// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::state_store::state_value::StateValueMetadata;
use bytes::Bytes;
use move_binary_format::{errors::PartialVMResult, CompiledModule};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr, metadata::Metadata};
use std::sync::Arc;

pub trait AptosModuleStorage {
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

    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Bytes>;

    fn fetch_module_size_in_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<usize>;

    fn fetch_module_state_value_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<StateValueMetadata>;

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
}
