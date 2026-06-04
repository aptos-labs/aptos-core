// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Storage access and module verification for the loader's cache-miss path.

use anyhow::{anyhow, Result};
use bytes::Bytes;
use move_binary_format::CompiledModule;
use move_core_types::{account_address::AccountAddress, identifier::Identifier};
use std::collections::HashMap;

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

/// In-memory [`ModuleProvider`] backed by a `HashMap` of module bytes plus a
/// per-package sibling index. Tests compile Move sources, serialize the
/// resulting [`CompiledModule`]s, and populate the provider. An empty provider
/// (no modules added) returns `Ok(None)` for every lookup, so the loader sees
/// a clean cache miss.
pub struct InMemoryModuleProvider {
    module_bytes: HashMap<(AccountAddress, Identifier), Bytes>,
    packages: HashMap<(AccountAddress, Identifier), Vec<Identifier>>,
}

impl InMemoryModuleProvider {
    pub fn new() -> Self {
        Self {
            module_bytes: HashMap::new(),
            packages: HashMap::new(),
        }
    }

    /// Adds a module. The module's bytes are obtained by serializing the
    /// provided [`CompiledModule`].
    pub fn add_module(&mut self, module: &CompiledModule) {
        let id = module.self_id();
        let mut bytes = Vec::new();
        module
            .serialize(&mut bytes)
            .expect("module serialization should succeed");
        self.module_bytes
            .insert((id.address, id.name), Bytes::from(bytes));
    }

    /// Adds every module from a compiled source.
    pub fn add_modules(&mut self, modules: &[CompiledModule]) {
        for m in modules {
            self.add_module(m);
        }
    }

    /// Declares that `(address, name)` belongs to a package whose other
    /// members are the given `siblings`. The `siblings` list must NOT
    /// include `name` itself — each stored entry is built to include the
    /// owner module, matching the `get_same_package_modules` contract
    /// ("returns all package members, including self").
    pub fn declare_package(
        &mut self,
        address: AccountAddress,
        name: Identifier,
        siblings: Vec<Identifier>,
    ) {
        let mut all = siblings.clone();
        all.push(name.clone());
        for member in &all {
            self.packages.insert((address, member.clone()), all.clone());
        }
    }
}

impl Default for InMemoryModuleProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleProvider for InMemoryModuleProvider {
    fn get_module_bytes(&self, address: &AccountAddress, name: &str) -> Result<Option<Bytes>> {
        let Ok(ident) = Identifier::new(name) else {
            return Ok(None);
        };
        Ok(self.module_bytes.get(&(*address, ident)).cloned())
    }

    fn deserialize_module(&self, bytes: &[u8]) -> Result<CompiledModule> {
        CompiledModule::deserialize(bytes).map_err(|e| anyhow!("deserialize failed: {e:?}"))
    }

    fn verify_module(&self, _module: &CompiledModule) -> Result<()> {
        // Tests assume the compiled modules are already valid.
        Ok(())
    }

    fn get_same_package_modules(
        &self,
        address: &AccountAddress,
        module_name: &str,
    ) -> Result<Vec<Identifier>> {
        let ident = Identifier::new(module_name)
            .map_err(|e| anyhow!("invalid module name {module_name:?}: {e}"))?;
        self.packages
            .get(&(*address, ident))
            .cloned()
            .ok_or_else(|| anyhow!("no package declared for {address}::{module_name}"))
    }
}
