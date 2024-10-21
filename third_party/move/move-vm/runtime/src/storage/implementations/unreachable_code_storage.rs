// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    CodeStorage, Module, ModuleStorage, RuntimeEnvironment, Script, WithRuntimeEnvironment,
};
use bytes::Bytes;
use move_binary_format::{errors::VMResult, file_format::CompiledScript, CompiledModule};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr, metadata::Metadata};
use std::sync::Arc;

/// Implementation of code storage (for modules and scripts) traits, to be used in case VM uses
/// V1 loader implementation in tests.
pub struct UnreachableCodeStorage;

impl WithRuntimeEnvironment for UnreachableCodeStorage {
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        unreachable!()
    }
}

impl ModuleStorage for UnreachableCodeStorage {
    fn check_module_exists(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> VMResult<bool> {
        unreachable!()
    }

    fn fetch_module_bytes(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>> {
        unreachable!()
    }

    fn fetch_module_size_in_bytes(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> VMResult<Option<usize>> {
        unreachable!()
    }

    fn fetch_module_metadata(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> VMResult<Option<Vec<Metadata>>> {
        unreachable!()
    }

    fn fetch_deserialized_module(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> VMResult<Option<Arc<CompiledModule>>> {
        unreachable!()
    }

    fn fetch_verified_module(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> VMResult<Option<Arc<Module>>> {
        unreachable!()
    }
}

impl CodeStorage for UnreachableCodeStorage {
    fn deserialize_and_cache_script(
        &self,
        _serialized_script: &[u8],
    ) -> VMResult<Arc<CompiledScript>> {
        unreachable!()
    }

    fn verify_and_cache_script(&self, _serialized_script: &[u8]) -> VMResult<Arc<Script>> {
        unreachable!()
    }
}
