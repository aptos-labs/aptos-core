// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::loader::Module;
use ambassador::delegatable_trait;
use bytes::Bytes;
use move_binary_format::{errors::VMResult, CompiledModule};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr, metadata::Metadata};
use std::sync::Arc;

/// Represents module storage backend, abstracting away any caching behaviour. The clients can
/// implement their own module storage to pass to the VM to resolve code.
// TODO(loader_v2): Implement tests to enforce these invariants for ALL implementations.
#[delegatable_trait]
pub trait ModuleStorage {
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
    /// a storage error.
    fn fetch_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Vec<Metadata>>>;

    /// Returns the deserialized module, or [None] otherwise. An error is returned if:
    ///   1. the deserialization fails, or
    ///   2. there is an error from the underlying storage.
    fn fetch_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<CompiledModule>>>;

    /// Returns the verified module if it exists, or [None] otherwise. The existing module can be
    /// either in a cached state (it is then returned) or newly constructed. The error is returned
    /// if the storage fails to fetch the deserialized module and verify it.
    fn fetch_verified_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<Module>>>;
}
