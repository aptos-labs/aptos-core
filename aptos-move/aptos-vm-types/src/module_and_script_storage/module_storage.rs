// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::state_store::state_value::StateValueMetadata;
use move_binary_format::errors::PartialVMResult;
use move_core_types::language_storage::ModuleId;
use move_vm_runtime::ModuleStorage;

/// Represents module storage used by the Aptos blockchain.
pub trait AptosModuleStorage: ModuleStorage {
    /// Returns the state value metadata associated with this module. The error is returned if
    /// there is a storage error. If the module does not exist, [None] is returned.
    fn fetch_state_value_metadata(
        &self,
        module_id: &ModuleId,
    ) -> PartialVMResult<Option<StateValueMetadata>>;
}
