// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::state_store::state_value::StateValueMetadata;
use bytes::Bytes;
use move_binary_format::errors::PartialVMResult;
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};
use move_vm_runtime::module_storage::ModuleStorage;

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
