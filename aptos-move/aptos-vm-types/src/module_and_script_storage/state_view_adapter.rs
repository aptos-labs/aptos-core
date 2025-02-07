// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::module_and_script_storage::module_storage::AptosModuleStorage;
use ambassador::Delegate;
use aptos_types::{
    error::PanicError,
    state_store::{state_key::StateKey, state_value::StateValueMetadata, StateView, TStateView},
    vm::modules::AptosModuleExtension,
};
use bytes::Bytes;
use move_binary_format::{
    errors::{PartialVMResult, VMResult},
    file_format::CompiledScript,
    CompiledModule,
};
use move_core_types::{language_storage::TypeTag, metadata::Metadata};
use move_vm_runtime::{
    ambassador_impl_CodeStorage, ambassador_impl_ModuleStorage,
    ambassador_impl_WithRuntimeEnvironment, AsUnsyncCodeStorage, BorrowedOrOwned, CodeStorage,
    Function, Module, ModuleStorage, RuntimeEnvironment, Script, UnsyncCodeStorage,
    UnsyncModuleStorage, WithRuntimeEnvironment,
};
use move_vm_types::{
    code::{ModuleBytesStorage, ModuleCode},
    indices::{FunctionIdx, ModuleIdx, StructIdx},
    loaded_data::runtime_types::{StructType, Type},
    module_storage_error,
};
use std::{ops::Deref, sync::Arc};

/// Avoids orphan rule to implement [ModuleBytesStorage] for [StateView].
struct StateViewAdapter<'s, S, E> {
    #[allow(dead_code)]
    env: E,
    state_view: BorrowedOrOwned<'s, S>,
}

impl<'s, S: StateView, E: WithRuntimeEnvironment> ModuleBytesStorage
    for StateViewAdapter<'s, S, E>
{
    fn fetch_module_bytes(&self, idx: &ModuleIdx) -> VMResult<Option<Bytes>> {
        let idx_map = self.env.runtime_environment().struct_name_index_map();
        let (address, module_name) = idx_map.module_addr_name_from_module_idx(idx);
        let state_key = StateKey::module(&address, &module_name);
        self.state_view
            .get_state_value_bytes(&state_key)
            .map_err(|e| module_storage_error!(&address, &module_name, e))
    }
}

impl<'s, S: StateView, E: WithRuntimeEnvironment> Deref for StateViewAdapter<'s, S, E> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.state_view
    }
}

/// A (not thread-safe) implementation of code storage on top of a state view. It is never built
/// directly by clients - only via [AsAptosCodeStorage] trait. Can be used to resolve both modules
/// and cached scripts.
#[derive(Delegate)]
#[delegate(
    WithRuntimeEnvironment,
    where = "S: StateView, E: Clone + WithRuntimeEnvironment"
)]
#[delegate(
    ModuleStorage,
    where = "S: StateView, E: Clone + WithRuntimeEnvironment"
)]
#[delegate(CodeStorage, where = "S: StateView, E: Clone + WithRuntimeEnvironment")]
pub struct AptosCodeStorageAdapter<'s, S, E> {
    storage: UnsyncCodeStorage<UnsyncModuleStorage<'s, StateViewAdapter<'s, S, E>, E>>,
}

impl<'s, S: StateView, E: Clone + WithRuntimeEnvironment> AptosCodeStorageAdapter<'s, S, E> {
    /// Creates new instance of [AptosCodeStorageAdapter] built on top of the passed state view and
    /// the provided runtime environment.
    fn from_borrowed(state_view: &'s S, runtime_environment: E) -> Self {
        let adapter = StateViewAdapter {
            env: runtime_environment.clone(),
            state_view: BorrowedOrOwned::Borrowed(state_view),
        };
        let storage = adapter.into_unsync_code_storage(runtime_environment);
        Self { storage }
    }

    /// Creates new instance of [AptosCodeStorageAdapter] capturing the passed state view and the
    /// provided environment.
    fn from_owned(state_view: S, runtime_environment: E) -> Self {
        let adapter = StateViewAdapter {
            env: runtime_environment.clone(),
            state_view: BorrowedOrOwned::Owned(state_view),
        };
        let storage = adapter.into_unsync_code_storage(runtime_environment);
        Self { storage }
    }

    /// Drains cached verified modules from the code storage, transforming them into format used by
    /// global caches.
    pub fn into_verified_module_code_iter(
        self,
    ) -> Result<
        impl Iterator<
            Item = (
                ModuleIdx,
                Arc<ModuleCode<CompiledModule, Module, AptosModuleExtension>>,
            ),
        >,
        PanicError,
    > {
        let (state_view, env, verified_modules_iter) = self
            .storage
            .into_module_storage()
            .unpack_into_verified_modules_iter();
        let index_map = env.runtime_environment().struct_name_index_map();

        Ok(verified_modules_iter
            .map(|(key, verified_code)| {
                // We have cached the module previously, so we must be able to find it in storage.
                let (address, module_name) = index_map.module_addr_name_from_module_idx(&key);
                let state_key = StateKey::module(&address, &module_name);

                let extension = state_view
                    .get_state_value(&state_key)
                    .map_err(|err| {
                        let msg = format!(
                            "Failed to retrieve module {}::{} from storage {:?}",
                            address, module_name, err
                        );
                        PanicError::CodeInvariantError(msg)
                    })?
                    .map_or_else(
                        || {
                            let msg = format!(
                                "Module {}::{} should exist, but it does not anymore",
                                address, module_name
                            );
                            Err(PanicError::CodeInvariantError(msg))
                        },
                        |state_value| Ok(AptosModuleExtension::new(state_value)),
                    )?;

                let module = ModuleCode::from_arced_verified(verified_code, Arc::new(extension));
                Ok((key, Arc::new(module)))
            })
            .collect::<Result<Vec<_>, PanicError>>()?
            .into_iter())
    }
}

impl<'s, S: StateView, E: Clone + WithRuntimeEnvironment> AptosModuleStorage
    for AptosCodeStorageAdapter<'s, S, E>
{
    fn fetch_state_value_metadata(
        &self,
        idx: &ModuleIdx,
    ) -> PartialVMResult<Option<StateValueMetadata>> {
        let idx_map = self.runtime_environment().struct_name_index_map();
        let (address, module_name) = idx_map.module_addr_name_from_module_idx(idx);
        let state_key = StateKey::module(&address, &module_name);
        Ok(self
            .storage
            .module_storage()
            .byte_storage()
            .state_view
            .get_state_value(&state_key)
            .map_err(|err| module_storage_error!(address, module_name, err).to_partial())?
            .map(|state_value| state_value.into_metadata()))
    }
}

/// Allows to treat the state view as a code storage with scripts and modules. The main use case is
/// when a transaction or a Move function has to be executed outside the long-living environment or
/// block executor, e.g., for single transaction simulation, in Aptos debugger, etc.
pub trait AsAptosCodeStorage<'s, S, E> {
    fn as_aptos_code_storage(&'s self, runtime_environment: E)
        -> AptosCodeStorageAdapter<'s, S, E>;

    fn into_aptos_code_storage(self, runtime_environment: E) -> AptosCodeStorageAdapter<'s, S, E>;
}

impl<'s, S: StateView, E: Clone + WithRuntimeEnvironment> AsAptosCodeStorage<'s, S, E> for S {
    fn as_aptos_code_storage(&'s self, runtime_environment: E) -> AptosCodeStorageAdapter<S, E> {
        AptosCodeStorageAdapter::from_borrowed(self, runtime_environment)
    }

    fn into_aptos_code_storage(self, runtime_environment: E) -> AptosCodeStorageAdapter<'s, S, E> {
        AptosCodeStorageAdapter::from_owned(self, runtime_environment)
    }
}
