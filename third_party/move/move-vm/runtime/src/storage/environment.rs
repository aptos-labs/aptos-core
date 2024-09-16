// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::VMConfig,
    loader::{check_natives, TypeCache},
    native_functions::{NativeFunction, NativeFunctions},
    storage::{
        struct_name_index_map::StructNameIndexMap, verified_module_cache::VERIFIED_MODULES_V2,
        verifier::VerifierExtension,
    },
    Module, Script,
};
use ambassador::delegatable_trait;
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
use parking_lot::RwLock;
use sha3::{Digest, Sha3_256};
use std::sync::Arc;

/// [MoveVM] runtime environment encapsulating different configurations. Shared between the VM and
/// the code cache, possibly across multiple threads.
pub struct RuntimeEnvironment {
    /// Configuration for the VM. Contains information about enabled checks, verification,
    /// deserialization, etc.
    vm_config: VMConfig,
    /// All registered native functions in the current context (binary). When a verified [Module]
    /// is constructed, existing native functions are inlined in the module representation, so that
    /// the interpreter can call them directly.
    natives: NativeFunctions,
    /// Optional verifier extension to run passes on modules and scripts provided externally.
    verifier_extension: Option<Arc<dyn VerifierExtension>>,

    /// Map from struct names to indices, to save on unnecessary cloning and reduce memory
    /// consumption. Used by all struct type creations in the VM and in code cache.
    struct_name_index_map: StructNameIndexMap,

    /// Type cache for struct layouts, tags and depths, shared across multiple threads.
    ///
    /// SAFETY:
    /// Here we informally show that it is safe to share type cache across multiple threads. Same
    /// argument applies to struct name indexing map.
    ///   1) Struct has been already published.
    ///      In this case, it is fine to have multiple transactions concurrently accessing and
    ///      caching struct names, layouts and depth formulas. Even if transaction failed due to
    ///      speculation, and is re-executed later, the speculative aborted execution cached a non-
    ///      speculative existing struct information. It is safe for other threads to access it.
    ///  2) Struct is being published with a module.
    ///     The design of V2 loader ensures that when modules are published, i.e., staged on top of
    ///     the existing module storage, the runtime environment is cloned. Hence, it is not even
    ///     possible to mutate this global cache speculatively.
    ty_cache: RwLock<TypeCache>,
}

impl RuntimeEnvironment {
    /// Creates a new runtime environment with native functions and default VM configurations. If
    /// there are duplicated natives, panics.
    pub fn new(
        natives: impl IntoIterator<Item = (AccountAddress, Identifier, Identifier, NativeFunction)>,
    ) -> Self {
        let vm_config = VMConfig {
            // Keep the paranoid mode on as we most likely want this for tests.
            paranoid_type_checks: true,
            ..VMConfig::default()
        };
        Self::new_with_config(natives, vm_config)
    }

    /// Creates a new runtime environment with native functions and VM configurations. If there are
    /// duplicated natives, panics.
    pub fn new_with_config(
        natives: impl IntoIterator<Item = (AccountAddress, Identifier, Identifier, NativeFunction)>,
        vm_config: VMConfig,
    ) -> Self {
        let natives = NativeFunctions::new(natives)
            .unwrap_or_else(|e| panic!("Failed to create native functions: {}", e));
        Self {
            vm_config,
            natives,
            verifier_extension: None,
            struct_name_index_map: StructNameIndexMap::empty(),
            ty_cache: RwLock::new(TypeCache::empty()),
        }
    }

    /// Returns the config currently used by this runtime environment.
    pub fn vm_config(&self) -> &VMConfig {
        &self.vm_config
    }

    /// Enables delayed field optimization for this environment.
    pub fn enable_delayed_field_optimization(&mut self) {
        self.vm_config.delayed_field_optimization_enabled = true;
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

    /// Creates a fully verified script by running dependency verification pass. The caller must
    /// provide verified module dependencies.
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

    /// Creates a fully verified module by running dependency verification pass. The caller must
    /// provide verified module dependencies.
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

    /// Deserializes bytes into a compiled module, also returning its size and hash.
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

    /// Deserializes bytes into a compiled script.
    pub fn deserialize_into_script(&self, serialized_script: &[u8]) -> VMResult<CompiledScript> {
        CompiledScript::deserialize_with_config(
            serialized_script,
            &self.vm_config().deserializer_config,
        )
        .map_err(|err| {
            let msg = format!("[VM] deserializer for script returned error: {:?}", err);
            PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR)
                .with_message(msg)
                .finish(Location::Script)
        })
    }

    /// Returns an error is module's address and name do not match the expected values.
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

    /// Returns native functions available to this runtime.
    pub(crate) fn natives(&self) -> &NativeFunctions {
        &self.natives
    }

    /// Returns the re-indexing map currently used by this runtime environment to remap struct
    /// identifiers into indices.
    pub(crate) fn struct_name_index_map(&self) -> &StructNameIndexMap {
        &self.struct_name_index_map
    }

    /// Returns the type cache owned by this runtime environment which stores information about
    /// struct layouts, tags and depth formulae.
    pub(crate) fn ty_cache(&self) -> &RwLock<TypeCache> {
        &self.ty_cache
    }
}

impl Clone for RuntimeEnvironment {
    /// Returns the cloned environment. Struct re-indexing map and type caches are cloned and no
    /// longer shared with the original environment.

    fn clone(&self) -> Self {
        Self {
            vm_config: self.vm_config.clone(),
            natives: self.natives.clone(),
            verifier_extension: self.verifier_extension.clone(),
            struct_name_index_map: self.struct_name_index_map.clone(),
            ty_cache: RwLock::new(self.ty_cache.read().clone()),
        }
    }
}

/// Represents any type that contains a [RuntimeEnvironment].
#[delegatable_trait]
pub trait WithRuntimeEnvironment {
    fn runtime_environment(&self) -> &RuntimeEnvironment;
}

/// Wrapper around partially verified compiled module, i.e., one that passed local bytecode
/// verification, but not the dependency checks yet. Also carries module size in bytes.
pub struct PartiallyVerifiedModule(Arc<CompiledModule>, usize);

impl PartiallyVerifiedModule {
    pub fn immediate_dependencies_iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (&AccountAddress, &IdentStr)> {
        self.0.immediate_dependencies_iter()
    }
}

/// Wrapper around partially verified compiled script, i.e., one that passed local bytecode
/// verification, but not the dependency checks yet.
pub struct PartiallyVerifiedScript(Arc<CompiledScript>);

impl PartiallyVerifiedScript {
    pub fn immediate_dependencies_iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (&AccountAddress, &IdentStr)> {
        self.0.immediate_dependencies_iter()
    }
}
