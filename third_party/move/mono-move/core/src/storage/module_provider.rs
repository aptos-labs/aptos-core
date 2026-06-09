// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Storage access and module verification for the loader's cache-miss path.

use anyhow::Result;
use bytes::Bytes;
use move_binary_format::CompiledModule;
use move_core_types::{account_address::AccountAddress, identifier::Identifier};

/// Provides modules to the loader on cache miss: fetching bytes, deserializing
/// and verifying them, and enumerating package membership.
// TODO: change error type
pub trait ModuleProvider {
    /// Returns raw module bytes from storage for the given module.
    // TODO: see if str is fine for state key
    fn get_module_bytes(&self, address: &AccountAddress, name: &str) -> Result<Option<Bytes>>;

    /// Deserializes raw bytes into a [`CompiledModule`].
    fn deserialize_module(&self, bytes: &[u8]) -> Result<CompiledModule>;

    /// Verifies deserialized compiled module.
    fn verify_module(&self, module: &CompiledModule) -> Result<()>;

    /// Returns **all** module names that belong to the same package as the
    /// given module.
    ///
    /// ## Invariants
    ///
    /// Invariants implementors must uphold:
    /// - Names in the returned list are unique.
    /// - The returned list includes the given module itself.
    ///
    /// ## Ordering
    ///
    /// No ordering guarantees are made or required.
    fn get_same_package_modules(
        &self,
        address: &AccountAddress,
        module_name: &str,
    ) -> Result<Vec<Identifier>>;
}
