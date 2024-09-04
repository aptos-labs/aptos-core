// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::module_and_script_storage::{
    code_storage::AptosCodeStorage, module_storage::AptosModuleStorage,
};
use ambassador::Delegate;
use aptos_types::state_store::{state_key::StateKey, state_value::StateValueMetadata, StateView};
use bytes::Bytes;
use move_binary_format::{
    errors::{PartialVMResult, VMResult},
    file_format::CompiledScript,
    CompiledModule,
};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr, metadata::Metadata};
use move_vm_runtime::{
    ambassador_impl_CodeStorage, ambassador_impl_ModuleStorage,
    ambassador_impl_WithRuntimeEnvironment, module_storage_error, move_vm::MoveVM, CodeStorage,
    IntoUnsyncCodeStorage, Module, ModuleBytesStorage, ModuleStorage, RuntimeEnvironment, Script,
    UnsyncCodeStorage, UnsyncModuleStorage, WithRuntimeEnvironment,
};
use std::sync::Arc;

/// Same as [module_storage_error], but works with state keys and is kept
/// as a partial VM error.
macro_rules! aptos_module_storage_error {
    ($state_key:ident, $err:ident) => {
        move_binary_format::errors::PartialVMError::new(
            move_core_types::vm_status::StatusCode::STORAGE_ERROR,
        )
        .with_message(format!(
            "Unexpected storage error for module at {:?}: {:?}",
            $state_key, $err
        ))
    };
}

/// Avoids orphan rule to implement [ModuleBytesStorage] for [StateView].
struct StateViewAdapter<'s, S> {
    state_view: &'s S,
}

impl<'s, S: StateView> ModuleBytesStorage for StateViewAdapter<'s, S> {
    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>> {
        let state_key = StateKey::module(address, module_name);
        self.state_view
            .get_state_value_bytes(&state_key)
            .map_err(|e| module_storage_error!(address, module_name, e))
    }
}

/// A (not thread-safe) implementation of code storage on top of a state view.
/// It is never built directly by clients - only via [AsAptosCodeStorage] trait.
/// Can be used to resolve both modules and scripts.
#[derive(Delegate)]
#[delegate(WithRuntimeEnvironment, where = "S: StateView")]
#[delegate(ModuleStorage, where = "S: StateView")]
#[delegate(CodeStorage, where = "S: StateView")]
pub struct AptosCodeStorageAdapter<'s, S> {
    storage: UnsyncCodeStorage<UnsyncModuleStorage<'s, StateViewAdapter<'s, S>>>,
}

impl<'s, S: StateView> AptosCodeStorageAdapter<'s, S> {
    fn new(state_view: &'s S, vm: &'s MoveVM) -> Self {
        let adapter = StateViewAdapter { state_view };
        let storage = adapter.into_unsync_code_storage(vm.runtime_environment());
        Self { storage }
    }

    fn state_view(&self) -> &'s S {
        self.storage.module_storage().byte_storage().state_view
    }
}

impl<'s, S: StateView> AptosModuleStorage for AptosCodeStorageAdapter<'s, S> {
    fn fetch_state_value_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Option<StateValueMetadata>> {
        let state_key = StateKey::module(address, module_name);
        Ok(self
            .state_view()
            .get_state_value(&state_key)
            .map_err(|e| module_storage_error!(address, module_name, e).to_partial())?
            .map(|s| s.into_metadata()))
    }

    fn fetch_module_size_by_state_key(
        &self,
        state_key: &StateKey,
    ) -> PartialVMResult<Option<usize>> {
        Ok(self
            .state_view()
            .get_state_value(state_key)
            .map_err(|e| aptos_module_storage_error!(state_key, e))?
            .map(|s| s.size()))
    }
}

impl<'s, S: StateView> AptosCodeStorage for AptosCodeStorageAdapter<'s, S> {}

/// Allows to treat a state view as a code storage with scripts and modules. The
/// main use case is when transaction or a Move function has to be executed outside
/// the long-living environment or block executor, e.g., for single transaction
/// simulation, Aptos debugger, etc.
pub trait AsAptosCodeStorage<'s, S> {
    fn as_aptos_code_storage(&'s self, vm: &'s MoveVM) -> AptosCodeStorageAdapter<'s, S>;
}

impl<'s, S: StateView> AsAptosCodeStorage<'s, S> for S {
    fn as_aptos_code_storage(&'s self, vm: &'s MoveVM) -> AptosCodeStorageAdapter<S> {
        AptosCodeStorageAdapter::new(self, vm)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_language_e2e_tests::data_store::FakeDataStore;
    use claims::{assert_none, assert_ok, assert_some_eq};

    #[test]
    fn test_aptos_code_storage() {
        let mut state_view = FakeDataStore::default();

        let state_key_1 = StateKey::raw(&[1]);
        state_view.set_legacy(state_key_1.clone(), vec![]);

        let state_key_2 = StateKey::raw(&[2]);
        state_view.set_legacy(state_key_2.clone(), vec![1, 2, 3, 4, 5]);

        let state_key_3 = StateKey::raw(&[3]);

        let vm = MoveVM::new(vec![]);
        let code_storage = state_view.as_aptos_code_storage(&vm);

        let size_1 = assert_ok!(code_storage.fetch_module_size_by_state_key(&state_key_1));
        assert_some_eq!(size_1, 0);

        let size_2 = assert_ok!(code_storage.fetch_module_size_by_state_key(&state_key_2));
        assert_some_eq!(size_2, 5);

        let size_3 = assert_ok!(code_storage.fetch_module_size_by_state_key(&state_key_3));
        assert_none!(size_3);
    }
}
