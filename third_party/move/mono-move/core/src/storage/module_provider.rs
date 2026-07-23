// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Storage access and module verification for the loader's cache-miss path.

use crate::{ExecutionErrorKind, IntoExecutionError, VMInternalError, VMResult};
use bytes::Bytes;
use move_binary_format::CompiledModule;
use move_core_types::{account_address::AccountAddress, identifier::Identifier};
use thiserror::Error;

/// Provides modules to the loader on cache miss: fetching bytes, deserializing
/// and verifying them, and enumerating package membership.
pub trait ModuleProvider {
    /// Returns raw module bytes from storage for the given module.
    // TODO(cleanup): see if str is fine for state key
    fn get_module_bytes(&self, address: &AccountAddress, name: &str) -> VMResult<Option<Bytes>>;

    /// Deserializes raw bytes into a [`CompiledModule`].
    fn deserialize_module(&self, bytes: &[u8]) -> VMResult<CompiledModule>;

    /// Verifies deserialized compiled module.
    fn verify_module(&self, module: &CompiledModule) -> VMResult<()>;

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
    ) -> VMResult<Vec<Identifier>>;
}

/// Empty storage with no modules: every fetch reports the module as absent.
pub struct NoModuleProvider;

#[derive(Debug, Error)]
#[error("NoModuleProvider has no modules to deserialize")]
struct NoModuleProviderError;

impl IntoExecutionError for NoModuleProviderError {
    fn kind(&self) -> ExecutionErrorKind {
        ExecutionErrorKind::Placeholder
    }
}

impl ModuleProvider for NoModuleProvider {
    fn get_module_bytes(&self, _address: &AccountAddress, _name: &str) -> VMResult<Option<Bytes>> {
        Ok(None)
    }

    fn deserialize_module(&self, _bytes: &[u8]) -> VMResult<CompiledModule> {
        Err(VMInternalError::new(NoModuleProviderError))
    }

    fn verify_module(&self, _module: &CompiledModule) -> VMResult<()> {
        Ok(())
    }

    fn get_same_package_modules(
        &self,
        _address: &AccountAddress,
        _module_name: &str,
    ) -> VMResult<Vec<Identifier>> {
        Ok(vec![])
    }
}
