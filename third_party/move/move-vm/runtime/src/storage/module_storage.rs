// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{loader::Module, WithRuntimeEnvironment};
use ambassador::delegatable_trait;
use bytes::Bytes;
use move_binary_format::{errors::VMResult, CompiledModule};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr, metadata::Metadata};
use move_vm_types::module_linker_error;
use std::sync::Arc;

/// Represents module storage backend, abstracting away any caching behaviour. The clients can
/// implement their own module storage to pass to the VM to resolve code.
#[delegatable_trait]
pub trait ModuleStorage: WithRuntimeEnvironment {
    /// Returns true if loader V2 implementation is enabled. Will be removed in the future, for now
    /// it is simply a convenient way to check the feature flag if module storage is available.
    // TODO(loader_v2): Remove this when loader V2 is enabled.
    fn is_enabled(&self) -> bool {
        self.runtime_environment().vm_config().use_loader_v2
    }

    /// Returns true if the module exists, and false otherwise. An error is returned if there is a
    /// storage error.
    fn check_module_exists(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<bool>;

    /// Returns module bytes if module exists, or [None] otherwise. An error is returned if there
    /// is a storage error.
    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>>;

    /// Returns the size of a module in bytes, or [None] otherwise. An error is returned if the
    /// there is a storage error.
    fn fetch_module_size_in_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<usize>>;

    /// Returns the metadata in the module, or [None] otherwise. An error is returned if there is
    /// a storage error or the module fails deserialization.
    fn fetch_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Vec<Metadata>>>;

    /// Returns the metadata in the module. An error is returned if there is a storage error,
    /// module fails deserialization, or does not exist.
    fn fetch_existing_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Vec<Metadata>> {
        self.fetch_module_metadata(address, module_name)?
            .ok_or_else(|| module_linker_error!(address, module_name))
    }

    /// Returns the deserialized module, or [None] otherwise. An error is returned if:
    ///   1. the deserialization fails, or
    ///   2. there is an error from the underlying storage.
    fn fetch_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<CompiledModule>>>;

    /// Returns the deserialized module. An error is returned if:
    ///   1. the deserialization fails,
    ///   2. there is an error from the underlying storage,
    ///   3. module does not exist.
    fn fetch_existing_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Arc<CompiledModule>> {
        self.fetch_deserialized_module(address, module_name)?
            .ok_or_else(|| module_linker_error!(address, module_name))
    }

    /// Returns the verified module if it exists, or [None] otherwise. The existing module can be
    /// either in a cached state (it is then returned) or newly constructed. The error is returned
    /// if the storage fails to fetch the deserialized module and verify it.
    fn fetch_verified_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<Module>>>;
}
