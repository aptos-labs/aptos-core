// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Storage access and module verification for the loader's cache-miss path.

use crate::{ExecutionErrorKind, IntoExecutionError, VMInternalError, VMResult};
use bytes::Bytes;
use move_binary_format::{
    deserializer::DeserializerConfig, errors::PartialVMError, CompiledModule,
};
use move_bytecode_verifier::VerifierConfig;
use move_core_types::{account_address::AccountAddress, identifier::Identifier};
use thiserror::Error;

/// Provides module bytes, package membership, and deserialization and
/// verification configs to the loader.
///
/// Stored modules may already be verified at publication; providers control
/// re-verification through [`Self::verify_module`]. The loader deserializes
/// and verifies transaction scripts using [`Self::deserializer_config`] and
/// [`Self::verifier_config`].
///
/// TODO(cleanup): move module verification and dependency linking checks into
/// the loader, using `verifier_config`. Remove `verify_module`, use
/// `VerificationScope::Nothing` to disable verification, and pass verified
/// modules to destack.
pub trait ModuleProvider {
    /// Returns raw module bytes from storage for the given module.
    // TODO(cleanup): see if str is fine for state key
    fn get_module_bytes(&self, address: &AccountAddress, name: &str) -> VMResult<Option<Bytes>>;

    /// Deserializes raw bytes into a [`CompiledModule`] under
    /// [`Self::deserializer_config`].
    fn deserialize_module(&self, bytes: &[u8]) -> VMResult<CompiledModule> {
        CompiledModule::deserialize_with_config(bytes, self.deserializer_config())
            .map_err(|err| VMInternalError::new(ModuleDeserializationError(err)))
    }

    /// Verifies a deserialized module. Providers may skip this check for code
    /// verified elsewhere.
    fn verify_module(&self, module: &CompiledModule) -> VMResult<()>;

    /// Returns the deserialization config for modules and scripts.
    fn deserializer_config(&self) -> &DeserializerConfig;

    /// Returns the loader's script verification config. Module verification
    /// uses [`Self::verify_module`].
    fn verifier_config(&self) -> &VerifierConfig;

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

#[derive(Debug, Error)]
#[error("module deserialization failed: {0}")]
struct ModuleDeserializationError(PartialVMError);

impl IntoExecutionError for ModuleDeserializationError {
    fn kind(&self) -> ExecutionErrorKind {
        // TODO(cleanup): add an execution error kind for deserialization failures.
        ExecutionErrorKind::Placeholder
    }
}

/// Empty storage with no modules: every fetch reports the module as absent.
pub struct NoModuleProvider;

impl ModuleProvider for NoModuleProvider {
    fn get_module_bytes(&self, _address: &AccountAddress, _name: &str) -> VMResult<Option<Bytes>> {
        Ok(None)
    }

    fn verify_module(&self, _module: &CompiledModule) -> VMResult<()> {
        Ok(())
    }

    fn deserializer_config(&self) -> &DeserializerConfig {
        &DeserializerConfig::DEFAULT
    }

    fn verifier_config(&self) -> &VerifierConfig {
        &VerifierConfig::DEFAULT
    }

    fn get_same_package_modules(
        &self,
        _address: &AccountAddress,
        _module_name: &str,
    ) -> VMResult<Vec<Identifier>> {
        Ok(vec![])
    }
}
