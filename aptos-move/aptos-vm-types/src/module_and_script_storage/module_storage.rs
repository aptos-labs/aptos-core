// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::state_store::{state_key::StateKey, state_value::StateValueMetadata};
use move_binary_format::errors::PartialVMResult;
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};
use move_vm_runtime::{DummyCodeStorage, ModuleStorage};

/// Represents module storage used by the Aptos blockchain.
pub trait AptosModuleStorage: ModuleStorage {
    /// Returns the state value metadata of an associated with this module. The
    /// error is returned if there is a storage error. If the module does not exist,
    /// `None` is returned.
    fn fetch_state_value_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Option<StateValueMetadata>>;

    fn fetch_module_size_by_state_key(
        &self,
        state_key: &StateKey,
    ) -> PartialVMResult<Option<usize>>;
}

impl AptosModuleStorage for DummyCodeStorage {
    fn fetch_state_value_metadata(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> PartialVMResult<Option<StateValueMetadata>> {
        Ok(None)
    }

    fn fetch_module_size_by_state_key(
        &self,
        _state_key: &StateKey,
    ) -> PartialVMResult<Option<usize>> {
        Ok(None)
    }
}
