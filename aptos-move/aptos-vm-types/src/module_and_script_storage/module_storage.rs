// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    executable::ModulePath,
    state_store::{state_key::StateKey, state_value::StateValueMetadata},
};
use move_binary_format::errors::PartialVMResult;
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};
use move_vm_runtime::ModuleStorage;
use std::fmt::Debug;

/// Represents module storage used by the Aptos blockchain.
pub trait AptosModuleStorage: TAptosModuleStorage<Key = StateKey> {}

impl<M: TAptosModuleStorage<Key = StateKey>> AptosModuleStorage for M {}

/// Represents module storage used by the Aptos blockchain, with generic keys. This allows generic
/// implementations in block executor.
pub trait TAptosModuleStorage: ModuleStorage {
    type Key: Debug + ModulePath;

    /// Returns the state value metadata associated with this module. The error is returned if
    /// there is a storage error. If the module does not exist, [None] is returned.
    fn fetch_state_value_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Option<StateValueMetadata>>;
}
