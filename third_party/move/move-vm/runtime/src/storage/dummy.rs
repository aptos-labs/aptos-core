// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    loader::{Module, Script},
    storage::{code_storage::CodeStorage, module_storage::ModuleStorage},
};
use bytes::Bytes;
use move_binary_format::{
    errors::{Location, PartialVMError, VMResult},
    file_format::CompiledScript,
    CompiledModule,
};
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
                .with_message("New loader and code cache are not yet implemented".to_string())
                .finish(Location::Undefined),
        )
    };
}

// TODO(loader_v2): Temporary infra to enable loader V2 to test & run things e2e locally, remove.
pub fn use_loader_v2_based_on_env() -> bool {
    std::env::var("USE_LOADER_V2").is_ok()
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
    ) -> VMResult<bool> {
        unexpected_unimplemented_error!()
    }

    fn fetch_module_size_in_bytes(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> VMResult<Option<usize>> {
        unexpected_unimplemented_error!()
    }

    fn fetch_module_bytes(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>> {
        unexpected_unimplemented_error!()
    }

    fn fetch_module_metadata(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> VMResult<Vec<Metadata>> {
        unexpected_unimplemented_error!()
    }

    fn fetch_deserialized_module(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> VMResult<Arc<CompiledModule>> {
        unexpected_unimplemented_error!()
    }

    fn fetch_verified_module(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> VMResult<Arc<Module>> {
        unexpected_unimplemented_error!()
    }
}

impl CodeStorage for DummyCodeStorage {
    fn deserialize_and_cache_script(
        &self,
        _serialized_script: &[u8],
    ) -> VMResult<Arc<CompiledScript>> {
        unexpected_unimplemented_error!()
    }

    fn verify_and_cache_script(&self, _serialized_script: &[u8]) -> VMResult<Arc<Script>> {
        unexpected_unimplemented_error!()
    }
}
