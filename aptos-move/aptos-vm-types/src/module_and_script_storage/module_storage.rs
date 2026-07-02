// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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

    /// Records `(address, module_name)` as a module read by the current transaction, for hot
    /// state promotion. The default is a no-op; only the read-recording storage overrides it.
    ///
    /// Lets callers record a script's declared dependencies from the loaded script, keeping
    /// the recorded read set independent of the verified-script cache, whose warmth depends on
    /// the execution schedule and must not influence the consensus-visible promoted set.
    fn record_module_read(&self, _address: &AccountAddress, _module_name: &IdentStr) {}
}
