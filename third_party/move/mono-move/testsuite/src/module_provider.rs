// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared helpers for loader integration tests.
//!
//! Provides an in-memory [`ModuleProvider`] backed by a `HashMap` of module
//! bytes plus a per-package sibling index. Tests compile Move sources via
//! [`crate::compile_move_modules`], serialize the resulting
//! [`CompiledModule`]s, and populate the provider.

use bytes::Bytes;
use mono_move_core::{ExecutionErrorKind, IntoExecutionError, VMInternalError, VMResult};
use mono_move_loader::ModuleProvider;
use move_binary_format::{deserializer::DeserializerConfig, CompiledModule};
use move_bytecode_verifier::VerifierConfig;
use move_core_types::{account_address::AccountAddress, identifier::Identifier};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModuleProviderError {
    #[error("invalid module name {module_name:?}")]
    InvalidModuleName { module_name: String },

    #[error("no package declared for {address}::{module_name}")]
    PackageNotFound {
        address: AccountAddress,
        module_name: String,
    },
}

impl IntoExecutionError for ModuleProviderError {
    fn kind(&self) -> ExecutionErrorKind {
        ExecutionErrorKind::Placeholder
    }
}

#[derive(Clone, Default)]
pub struct InMemoryModuleProvider {
    module_bytes: HashMap<(AccountAddress, Identifier), Bytes>,
    packages: HashMap<(AccountAddress, Identifier), Vec<Identifier>>,
    deserializer_config: DeserializerConfig,
    verifier_config: VerifierConfig,
}

impl InMemoryModuleProvider {
    /// An empty provider using the default deserializer and verifier configs.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty provider with the given configs. Deserialization
    /// settings apply to modules and scripts; verification settings apply only
    /// to scripts.
    pub fn with_configs(
        deserializer_config: DeserializerConfig,
        verifier_config: VerifierConfig,
    ) -> Self {
        Self {
            deserializer_config,
            verifier_config,
            ..Self::default()
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

    /// Adds a module from its already-serialized bytes (e.g. bytecode read from chain).
    pub fn add_module_bytes(&mut self, address: AccountAddress, name: Identifier, bytes: Bytes) {
        self.module_bytes.insert((address, name), bytes);
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

impl ModuleProvider for InMemoryModuleProvider {
    fn get_module_bytes(&self, address: &AccountAddress, name: &str) -> VMResult<Option<Bytes>> {
        let Ok(ident) = Identifier::new(name) else {
            return Ok(None);
        };
        Ok(self.module_bytes.get(&(*address, ident)).cloned())
    }

    fn verify_module(&self, _module: &CompiledModule) -> VMResult<()> {
        // The loader verifies bytecode during translation, so no separate check is needed here.
        Ok(())
    }

    fn deserializer_config(&self) -> &DeserializerConfig {
        &self.deserializer_config
    }

    fn verifier_config(&self) -> &VerifierConfig {
        &self.verifier_config
    }

    fn get_same_package_modules(
        &self,
        address: &AccountAddress,
        module_name: &str,
    ) -> VMResult<Vec<Identifier>> {
        let ident = Identifier::new(module_name).map_err(|_| {
            VMInternalError::new(ModuleProviderError::InvalidModuleName {
                module_name: module_name.to_string(),
            })
        })?;
        self.packages
            .get(&(*address, ident))
            .cloned()
            .ok_or_else(|| {
                VMInternalError::new(ModuleProviderError::PackageNotFound {
                    address: *address,
                    module_name: module_name.to_string(),
                })
            })
    }
}
