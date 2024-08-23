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
    ambassador_impl_CodeStorage, ambassador_impl_ModuleStorage, module_storage_error,
    move_vm::MoveVM, CodeStorage, IntoUnsyncCodeStorage, Module, ModuleBytesStorage, ModuleStorage,
    Script, UnsyncCodeStorage, UnsyncModuleStorage,
};
use std::sync::Arc;

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
#[delegate(ModuleStorage)]
#[delegate(CodeStorage)]
pub struct AptosCodeStorageAdapter<'s, S> {
    storage: UnsyncCodeStorage<UnsyncModuleStorage<'s, StateViewAdapter<'s, S>>>,
}

impl<'s, S: StateView> AptosCodeStorageAdapter<'s, S> {
    fn new(state_view: &'s S, vm: &'s MoveVM) -> Self {
        let adapter = StateViewAdapter { state_view };
        let storage = adapter.into_unsync_code_storage(vm.runtime_environment());
        Self { storage }
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
            .storage
            .module_storage()
            .byte_storage()
            .state_view
            .get_state_value(&state_key)
            .map_err(|e| module_storage_error!(address, module_name, e).to_partial())?
            .map(|s| s.into_metadata()))
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
