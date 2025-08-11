// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::state_store::state_value::StateValueMetadata;
use move_binary_format::errors::PartialVMResult;
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};
use move_vm_runtime::ModuleStorage;

/// Represents module storage used by the Aptos blockchain.
pub trait AptosModuleStorage: ModuleStorage {
    /// Returns the state value metadata associated with this module. The error is returned if
    /// there is a storage error. If the module does not exist, [None] is returned.
    ///
    /// Note: this API is not metered!
    fn unmetered_get_module_state_value_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Option<StateValueMetadata>>;
}
