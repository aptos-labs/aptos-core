// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    loader::{Module, Script},
    storage::{module_storage::ModuleStorage, script_storage::ScriptStorage, verifier::Verifier},
};
use move_binary_format::{
    errors::{PartialVMError, PartialVMResult},
    file_format::CompiledScript,
    CompiledModule,
};
use move_bytecode_verifier::dependencies;
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, metadata::Metadata,
    vm_status::StatusCode,
};
use std::sync::Arc;

/// An error which is returned in case unimplemented code is reached. This is just a safety
/// precaution to avoid panics in case we forget some gating.
macro_rules! unexpected_unimplemented_error {
    () => {
        Err(
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("New loader and code cache are not yet implemented".to_string()),
        )
    };
}

// Temporary infra to enable loader V2 to test & run things e2e locally.
pub fn should_use_loader_v2() -> bool {
    std::env::var("USE_LOADER_V2").is_ok()
}

pub(crate) fn ok_if_should_use_loader_v2() -> PartialVMResult<()> {
    if should_use_loader_v2() {
        Ok(())
    } else {
        unexpected_unimplemented_error!()
    }
}

/// Dummy implementation of code storage (for modules and scripts), to be removed in the future.
/// Used as a placeholder so that existing APIs can work, i.e., for now client side which has no
/// script or module storage can be still connected to the new V2 APIs by using the dummy.
pub struct DummyCodeStorage;

impl ModuleStorage for DummyCodeStorage {
    fn check_module_exists(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> PartialVMResult<bool> {
        unexpected_unimplemented_error!()
    }

    fn fetch_module_size_in_bytes(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> PartialVMResult<usize> {
        unexpected_unimplemented_error!()
    }

    fn fetch_module_metadata(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> PartialVMResult<Vec<Metadata>> {
        unexpected_unimplemented_error!()
    }

    fn fetch_deserialized_module(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> PartialVMResult<Arc<CompiledModule>> {
        unexpected_unimplemented_error!()
    }

    fn fetch_or_create_verified_module(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
        _f: &dyn Fn(Arc<CompiledModule>) -> PartialVMResult<Module>,
    ) -> PartialVMResult<Arc<Module>> {
        unexpected_unimplemented_error!()
    }
}

impl ScriptStorage for DummyCodeStorage {
    fn fetch_deserialized_script(
        &self,
        _serialized_script: &[u8],
    ) -> PartialVMResult<Arc<CompiledScript>> {
        unexpected_unimplemented_error!()
    }

    fn fetch_or_create_verified_script(
        &self,
        _serialized_script: &[u8],
        _f: &dyn Fn(Arc<CompiledScript>) -> PartialVMResult<Script>,
    ) -> PartialVMResult<Arc<Script>> {
        unexpected_unimplemented_error!()
    }
}

/// Placeholder to use for now before an actual verifier is implemented.
#[derive(Clone)]
pub struct DummyVerifier;

impl Verifier for DummyVerifier {
    fn verify_script(&self, script: &CompiledScript) -> PartialVMResult<()> {
        ok_if_should_use_loader_v2()?;
        move_bytecode_verifier::verify_script(script).map_err(|e| e.to_partial())?;
        Ok(())
    }

    fn verify_script_with_dependencies<'a>(
        &self,
        script: &CompiledScript,
        dependencies: impl IntoIterator<Item = &'a Module>,
    ) -> PartialVMResult<()> {
        ok_if_should_use_loader_v2()?;
        dependencies::verify_script(script, dependencies.into_iter().map(|m| m.module()))
            .map_err(|e| e.to_partial())?;
        Ok(())
    }

    fn verify_module(&self, module: &CompiledModule) -> PartialVMResult<()> {
        ok_if_should_use_loader_v2()?;
        move_bytecode_verifier::verify_module(module).map_err(|e| e.to_partial())?;
        Ok(())
    }

    fn verify_module_with_dependencies<'a>(
        &self,
        module: &CompiledModule,
        dependencies: impl IntoIterator<Item = &'a Module>,
    ) -> PartialVMResult<()> {
        ok_if_should_use_loader_v2()?;
        dependencies::verify_module(module, dependencies.into_iter().map(|m| m.module()))
            .map_err(|e| e.to_partial())?;
        Ok(())
    }
}
