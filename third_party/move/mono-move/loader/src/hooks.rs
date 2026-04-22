// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Hooks for the loader's cache-miss path.

use bytes::Bytes;
use move_binary_format::CompiledModule;
use move_core_types::{account_address::AccountAddress, identifier::Identifier};

/// Customizable hooks to use on cache miss.
// TODO: change error type
pub trait LoaderHooks {
    /// Returns raw module bytes from storage for the given module.
    // TODO: see if str is fine for state key
    fn get_module_bytes(
        &self,
        address: &AccountAddress,
        name: &str,
    ) -> anyhow::Result<Option<Bytes>>;

    /// Deserializes raw bytes into a [`CompiledModule`].
    fn deserialize_module(&self, bytes: &[u8]) -> anyhow::Result<CompiledModule>;

    /// Verifies deserialized compiled module.
    fn verify_module(&self, module: &CompiledModule) -> anyhow::Result<()>;

    /// Returns **all** module names that belong to the same package as the
    /// given module. The returned list **includes** the module itself.
    ///
    /// Names in the returned list must be unique. Callers rely on this for
    /// correctness; duplicates will cause a double-record into the read-set.
    fn get_same_package_modules(
        &self,
        address: &AccountAddress,
        module_name: &str,
    ) -> anyhow::Result<Vec<Identifier>>;
}
