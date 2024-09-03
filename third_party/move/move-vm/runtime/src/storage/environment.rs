// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::VMConfig,
    loader::check_natives,
    native_functions::{NativeFunction, NativeFunctions},
    storage::{
        struct_name_index_map::StructNameIndexMap, verified_module_cache::VERIFIED_MODULES_V2,
        verifier::VerifierExtension,
    },
    Module, Script,
};
use bytes::Bytes;
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    errors::{Location, PartialVMError, VMResult},
    file_format::CompiledScript,
    CompiledModule,
};
use move_bytecode_verifier::dependencies;
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    vm_status::{sub_status::unknown_invariant_violation::EPARANOID_FAILURE, StatusCode},
};
use sha3::{Digest, Sha3_256};
use std::sync::Arc;

/// Wrapper around partially verified compiled module, i.e., one that passed
/// local bytecode verification, but not the dependency checks yet. Also
/// carries size in bytes.
pub struct PartiallyVerifiedModule(Arc<CompiledModule>, usize);

impl PartiallyVerifiedModule {
    pub fn immediate_dependencies_iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (&AccountAddress, &IdentStr)> {
        self.0.immediate_dependencies_iter()
    }

    pub fn immediate_friends_iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (&AccountAddress, &IdentStr)> {
        self.0.immediate_friends_iter()
    }
}

/// Wrapper around partially verified compiled script, i.e., one that passed
/// local bytecode verification, but not the dependency checks yet.
pub struct PartiallyVerifiedScript(Arc<CompiledScript>);

impl PartiallyVerifiedScript {
    pub fn immediate_dependencies_iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (&AccountAddress, &IdentStr)> {
        self.0.immediate_dependencies_iter()
    }
}

/// [MoveVM] runtime environment encapsulating different configurations. Shared
/// between the VM and the code cache.
pub struct RuntimeEnvironment {
    /// Configuration for the VM. Contains information about enabled checks,
    /// verification, deserialization, etc.
    vm_config: VMConfig,
    /// All registered native functions in the current context (binary). When
    /// a verified [Module] is constructed, existing native functions are inlined
    /// in the module representation, so that the interpreter can call them directly.
    natives: NativeFunctions,
    /// Optional verifier extension to run passes on modules and scripts provided externally.
    verifier_extension: Option<Arc<dyn VerifierExtension>>,

    /// Map from struct names to indices, to save on unnecessary cloning and reduce
    /// memory consumption. Used by all struct type creations in the VM and in code cache.
    struct_name_index_map: StructNameIndexMap,
}

impl RuntimeEnvironment {
    /// Creates a new runtime environment with native functions and VM configurations.
    /// If there are duplicated natives, creation panics. Also, callers can provide
    /// verification extensions to add hooks on top of a bytecode verifier.
    pub fn new(
        vm_config: VMConfig,
        natives: impl IntoIterator<Item = (AccountAddress, Identifier, Identifier, NativeFunction)>,
        verifier_extension: Option<Arc<dyn VerifierExtension>>,
    ) -> Self {
        let natives = NativeFunctions::new(natives)
            .unwrap_or_else(|e| panic!("Failed to create native functions: {}", e));
        Self {
            vm_config,
            natives,
            struct_name_index_map: StructNameIndexMap::empty(),
            verifier_extension,
        }
    }

    pub fn test() -> Self {
        Self {
            vm_config: VMConfig::default(),
            natives: NativeFunctions::new(vec![]).unwrap(),
            struct_name_index_map: StructNameIndexMap::empty(),
            verifier_extension: None,
        }
    }

    /// Returns the config currently used by this runtime environment.
    pub fn vm_config(&self) -> &VMConfig {
        &self.vm_config
    }

    /// Returns native functions available to this runtime.
    pub(crate) fn natives(&self) -> &NativeFunctions {
        &self.natives
    }

    /// Returns the re-indexing map currently used by this runtime environment
    /// to remap struct identifiers into indices.
    pub(crate) fn struct_name_index_map(&self) -> &StructNameIndexMap {
        &self.struct_name_index_map
    }

    /// Returns the cloned environment, with a deep-clone of struct name index map.
    pub fn clone_with_new_struct_name_index_map(&self) -> Self {
        Self {
            vm_config: self.vm_config.clone(),
            natives: self.natives.clone(),
            verifier_extension: self.verifier_extension.clone(),
            struct_name_index_map: self.struct_name_index_map.clone(),
        }
    }

    /// Creates a partially verified compiled script by running:
    ///   1. Move bytecode verifier,
    ///   2. Verifier extension, if provided.
    pub fn build_partially_verified_script(
        &self,
        compiled_script: Arc<CompiledScript>,
    ) -> VMResult<PartiallyVerifiedScript> {
        move_bytecode_verifier::verify_script_with_config(
            &self.vm_config().verifier_config,
            compiled_script.as_ref(),
        )?;
        if let Some(verifier) = &self.verifier_extension {
            verifier.verify_script(compiled_script.as_ref())?;
        }
        Ok(PartiallyVerifiedScript(compiled_script))
    }

    /// Creates a fully verified script by running dependency verification
    /// pass. The caller must provide verified module dependencies.
    pub fn build_verified_script(
        &self,
        partially_verified_script: PartiallyVerifiedScript,
        immediate_dependencies: &[Arc<Module>],
    ) -> VMResult<Script> {
        dependencies::verify_script(
            partially_verified_script.0.as_ref(),
            immediate_dependencies.iter().map(|m| m.module()),
        )?;
        Script::new(partially_verified_script.0, self.struct_name_index_map())
            .map_err(|e| e.finish(Location::Script))
    }

    /// Creates a partially verified compiled module by running:
    ///   1. Move bytecode verifier,
    ///   2. Verifier extension, if provided.
    pub fn build_partially_verified_module(
        &self,
        compiled_module: Arc<CompiledModule>,
        module_size: usize,
        module_hash: &[u8; 32],
    ) -> VMResult<PartiallyVerifiedModule> {
        if !VERIFIED_MODULES_V2.contains(module_hash) {
            // For regular execution, we cache already verified modules. Note
            // that this even caches verification for the published modules.
            // This should be ok because as long as the hash is the same, the
            // deployed bytecode and any dependencies are the same, and so the
            // cached verification result can be used.
            move_bytecode_verifier::verify_module_with_config(
                &self.vm_config().verifier_config,
                compiled_module.as_ref(),
            )?;
            check_natives(compiled_module.as_ref())?;

            if let Some(verifier) = &self.verifier_extension {
                verifier.verify_module(compiled_module.as_ref())?;
            }

            VERIFIED_MODULES_V2.put(*module_hash);
        }

        Ok(PartiallyVerifiedModule(compiled_module, module_size))
    }

    /// Creates a fully verified module by running dependency verification
    /// pass. The caller must provide verified module dependencies.
    pub fn build_verified_module(
        &self,
        partially_verified_module: PartiallyVerifiedModule,
        immediate_dependencies: &[Arc<Module>],
    ) -> VMResult<Module> {
        dependencies::verify_module(
            partially_verified_module.0.as_ref(),
            immediate_dependencies.iter().map(|m| m.module()),
        )?;
        let result = Module::new(
            &self.natives,
            partially_verified_module.1,
            partially_verified_module.0,
            self.struct_name_index_map(),
        );

        // Note: loader V1 implementation does not set locations for this error.
        result.map_err(|e| e.finish(Location::Undefined))
    }

    /// Deserializes bytes into a compiled module. In addition, returns the size
    /// of the module and its hash.
    pub fn deserialize_into_compiled_module(
        &self,
        bytes: &Bytes,
    ) -> VMResult<(CompiledModule, usize, [u8; 32])> {
        let compiled_module =
            CompiledModule::deserialize_with_config(bytes, &self.vm_config().deserializer_config)
                .map_err(|err| {
                let msg = format!("Deserialization error: {:?}", err);
                PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR)
                    .with_message(msg)
                    .finish(Location::Undefined)
            })?;

        let mut sha3_256 = Sha3_256::new();
        sha3_256.update(bytes);
        let module_hash: [u8; 32] = sha3_256.finalize().into();

        Ok((compiled_module, bytes.len(), module_hash))
    }

    /// Returns ann error is module's address and name do not match the expected values.
    /// In general, we enforce this is the case at module publish time.
    #[inline]
    pub fn paranoid_check_module_address_and_name(
        &self,
        module: &CompiledModule,
        expected_address: &AccountAddress,
        expected_module_name: &IdentStr,
    ) -> VMResult<()> {
        if self.vm_config().paranoid_type_checks {
            let actual_address = module.self_addr();
            let actual_module_name = module.self_name();
            if expected_address != actual_address || expected_module_name != actual_module_name {
                let msg = format!(
                    "Expected module {}::{}, but got {}::{}",
                    expected_address, expected_module_name, actual_address, actual_module_name
                );
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(msg)
                        .with_sub_status(EPARANOID_FAILURE)
                        .finish(Location::Undefined),
                );
            }
        }
        Ok(())
    }
}

/// Represents any type that contains a [RuntimeEnvironment].
pub trait WithRuntimeEnvironment {
    fn runtime_environment(&self) -> &RuntimeEnvironment;
}
