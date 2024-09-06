// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::loader::Module;
use ambassador::delegatable_trait;
use bytes::Bytes;
use move_binary_format::{errors::VMResult, CompiledModule};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr, metadata::Metadata};
use std::sync::Arc;

/// Represents module storage backend, abstracting away any caching behaviour. The
/// clients can implement their own module storage to pass to the VM to resolve code.
#[delegatable_trait]
pub trait ModuleStorage {
    /// Returns true if the module exists, and false otherwise. An error is returned
    /// if there is a storage error.
    fn check_module_exists(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<bool>;

    /// Returns module bytes. An error is returned if there is a storage error. If
    /// the module does not exist, returns [None].
    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>>;

    /// Returns the size of a module in bytes. An error is returned if the there is
    /// a storage error. If module does not exist, [None] is returned.
    fn fetch_module_size_in_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<usize>>;

    /// Returns the metadata in the module. An error is returned if the module does
    /// not exist, or there is a storage error.
    fn fetch_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Vec<Metadata>>;

    /// Returns the deserialized module. There is no guarantees that the module has been
    /// previously verified. An error is returned if:
    ///   1. the module does not exist,
    ///   2. the deserialization fails, or
    ///   3. there is an error from the underlying storage, i.e., the DB.
    fn fetch_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Arc<CompiledModule>>;

    /// Returns the verified module. The module can be either in a cached state (it is
    /// then returned) or newly constructed. The error is returned if the storage fails
    /// to fetch the compiled script and verify it.
    fn fetch_verified_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Arc<Module>>;
}

/// Storage that contains serialized modules. Clients can implement this trait
/// for their own backends, so that [ModuleStorage] can be built on top of it.
pub trait ModuleBytesStorage {
    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>>;
}

/// Storage that contains serialized modules. Clients can implement this trait
/// for their own backends, so that [ModuleStorage] can be built on top of it.
pub trait ModuleBytesStorage {
    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Option<Bytes>>;
}
