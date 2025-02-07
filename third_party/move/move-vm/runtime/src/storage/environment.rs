// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::VMConfig,
    loader::check_natives,
    native_functions::{NativeFunction, NativeFunctions},
    storage::{ty_tag_converter::TypeTagCache, verified_module_cache::VERIFIED_MODULES_V2},
    Module, Script,
};
use ambassador::delegatable_trait;
use bytes::Bytes;
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
    file_format::CompiledScript,
    CompiledModule,
};
use move_bytecode_verifier::dependencies;
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    vm_status::{sub_status::unknown_invariant_violation::EPARANOID_FAILURE, StatusCode},
};
use move_vm_metrics::{Timer, VM_TIMER};
use move_vm_types::indices::{IndexMapManager, ModuleIdx};
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

    /// Map from struct names to indices, to save on unnecessary cloning and reduce memory
    /// consumption. Used by all struct type creations in the VM and in code cache.
    ///
    /// SAFETY:
    ///   By itself, it is fine to index struct names even of non-successful module publishes. If
    ///   we cached some name, which was not published, it will stay in cache and will be used by
    ///   another republish. Since there is no other information other than index, even for structs
    ///   with different layouts it is fine to re-use the index.
    ///   We wrap the index map into an [Arc] so that on republishing these clones are cheap.
    struct_name_index_map: Arc<IndexMapManager>,

    /// Caches struct tags for instantiated types. This cache can be used concurrently and
    /// speculatively because type tag information does not change with module publishes.
    ty_tag_cache: Arc<TypeTagCache>,
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
            struct_name_index_map: Arc::new(IndexMapManager::new()),
            ty_tag_cache: Arc::new(TypeTagCache::empty()),
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

    /// Creates a locally verified compiled script by running:
    ///   1. Move bytecode verifier,
    ///   2. Verifier extension, if provided.
    pub fn build_locally_verified_script(
        &self,
        compiled_script: Arc<DeserializedScript>,
    ) -> VMResult<LocallyVerifiedScript> {
        move_bytecode_verifier::verify_script_with_config(
            &self.vm_config().verifier_config,
            compiled_script.compiled_script.as_ref(),
        )?;
        Ok(LocallyVerifiedScript(compiled_script))
    }

    /// Creates a verified script by running dependency verification pass over locally verified
    /// script. The caller must provide verified module dependencies.
    pub fn build_verified_script(
        &self,
        locally_verified_script: LocallyVerifiedScript,
        immediate_dependencies: &[Arc<Module>],
    ) -> VMResult<Script> {
        dependencies::verify_script(
            locally_verified_script.0.compiled_script.as_ref(),
            immediate_dependencies
                .iter()
                .map(|module| module.compiled_module.as_ref()),
        )?;

        Script::new(locally_verified_script.0, self.struct_name_index_map())
            .map_err(|err| err.finish(Location::Script))
    }

    /// Creates a locally verified compiled module by running:
    ///   1. Move bytecode verifier,
    ///   2. Verifier extension, if provided.
    pub fn build_locally_verified_module(
        &self,
        compiled_module: Arc<DeserializedModule>,
        module_size: usize,
        module_hash: &[u8; 32],
    ) -> VMResult<LocallyVerifiedModule> {
        if !VERIFIED_MODULES_V2.contains(module_hash) {
            let _timer = VM_TIMER.timer_with_label(
                "LoaderV2::build_locally_verified_module [verification cache miss]",
            );

            // For regular execution, we cache already verified modules. Note that this even caches
            // verification for the published modules. This should be ok because as long as the
            // hash is the same, the deployed bytecode and any dependencies are the same, and so
            // the cached verification result can be used.
            move_bytecode_verifier::verify_module_with_config(
                &self.vm_config().verifier_config,
                compiled_module.compiled_module.as_ref(),
            )?;
            check_natives(compiled_module.compiled_module.as_ref())?;
            VERIFIED_MODULES_V2.put(*module_hash);
        }

        Ok(LocallyVerifiedModule(compiled_module, module_size))
    }

    /// Creates a verified module by running dependency verification pass for a locally verified
    /// module. The caller must provide verified module dependencies.
    pub fn build_verified_module(
        &self,
        locally_verified_module: LocallyVerifiedModule,
        immediate_dependencies: &[Arc<Module>],
    ) -> VMResult<Module> {
        dependencies::verify_module(
            locally_verified_module.0.compiled_module.as_ref(),
            immediate_dependencies
                .iter()
                .map(|module| module.compiled_module.as_ref()),
        )?;
        let result = Module::new(
            &self.natives,
            locally_verified_module.1,
            locally_verified_module.0,
            self.struct_name_index_map(),
        );

        // Note: loader V1 implementation does not set locations for this error.
        result.map_err(|e| e.finish(Location::Undefined))
    }

    /// Deserializes bytes into a compiled module.
    pub fn deserialize_into_compiled_module(&self, bytes: &Bytes) -> VMResult<DeserializedModule> {
        let compiled_module =
            CompiledModule::deserialize_with_config(bytes, &self.vm_config().deserializer_config)
                .map_err(|err| {
                let msg = format!("Deserialization error: {:?}", err);
                PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR)
                    .with_message(msg)
                    .finish(Location::Undefined)
            })?;

        let deps = compiled_module
            .immediate_dependencies_iter()
            .map(|(addr, name)| self.struct_name_index_map().module_idx(addr, name))
            .collect();
        let friends = compiled_module
            .immediate_friends_iter()
            .map(|(addr, name)| self.struct_name_index_map().module_idx(addr, name))
            .collect();

        Ok(DeserializedModule {
            compiled_module: Arc::new(compiled_module),
            deps: Arc::new(deps),
            friends: Arc::new(friends),
        })
    }

    /// Deserializes bytes into a compiled script.
    pub fn deserialize_into_script(
        &self,
        serialized_script: &[u8],
    ) -> VMResult<DeserializedScript> {
        let compiled_script = CompiledScript::deserialize_with_config(
            serialized_script,
            &self.vm_config().deserializer_config,
        )
        .map_err(|err| {
            let msg = format!("[VM] deserializer for script returned error: {:?}", err);
            PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR)
                .with_message(msg)
                .finish(Location::Script)
        })?;

        let deps = compiled_script
            .immediate_dependencies_iter()
            .map(|(addr, name)| self.struct_name_index_map().module_idx(addr, name))
            .collect();

        Ok(DeserializedScript {
            compiled_script: Arc::new(compiled_script),
            deps: Arc::new(deps),
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
    pub fn struct_name_index_map(&self) -> &IndexMapManager {
        &self.struct_name_index_map
    }

    /// Returns the type tag cache used by this environment to store already constructed struct
    /// tags.
    pub(crate) fn ty_tag_cache(&self) -> &TypeTagCache {
        &self.ty_tag_cache
    }

    /// Returns the size of the struct name re-indexing cache. Can be used to bound the size of the
    /// cache at block boundaries.
    pub fn struct_name_index_map_size(&self) -> PartialVMResult<usize> {
        Ok(0)
        // FIXME
        // self.struct_name_index_map.checked_len()
    }

    /// Flushes the global caches with struct name indices and struct tags. Note that when calling
    /// this function, modules that still store indices into struct name cache must also be flushed.
    pub fn flush_struct_name_and_tag_caches(&self) {
        self.ty_tag_cache.flush();
        // FIXME
        // self.struct_name_index_map.flush();
    }

    // /// Test-only function to be able to populate [StructNameIndexMap] outside of this crate.
    // FIXME
    // #[cfg(any(test, feature = "testing"))]
    // pub fn struct_name_to_idx_for_test(
    //     &self,
    //     struct_name: StructIdentifier,
    // ) -> PartialVMResult<StructNameIndex> {
    //     self.struct_name_index_map.struct_name_to_idx(&struct_name)
    // }

    // /// Test-only function to be able to check cached struct names.
    // FIXME
    // #[cfg(any(test, feature = "testing"))]
    // pub fn idx_to_struct_name_for_test(
    //     &self,
    //     idx: StructNameIndex,
    // ) -> PartialVMResult<StructIdentifier> {
    //     self.struct_name_index_map.idx_to_struct_name(idx)
    // }
}

impl Clone for RuntimeEnvironment {
    fn clone(&self) -> Self {
        Self {
            vm_config: self.vm_config.clone(),
            natives: self.natives.clone(),
            struct_name_index_map: Arc::clone(&self.struct_name_index_map),
            ty_tag_cache: Arc::clone(&self.ty_tag_cache),
        }
    }
}

/// Represents any type that contains a [RuntimeEnvironment].
#[delegatable_trait]
pub trait WithRuntimeEnvironment {
    fn runtime_environment(&self) -> &RuntimeEnvironment;
}

impl WithRuntimeEnvironment for RuntimeEnvironment {
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self
    }
}

///Compiled module that passed local bytecode verification, but not the linking checks yet for its
/// dependencies. Also carries module size in bytes.
pub struct LocallyVerifiedModule(Arc<DeserializedModule>, usize);

impl LocallyVerifiedModule {
    pub fn immediate_dependencies_iter(&self) -> impl DoubleEndedIterator<Item = &ModuleIdx> {
        self.0.deps.iter()
    }
}

/// Compiled script that passed local bytecode verification, but not the linking checks.
pub struct LocallyVerifiedScript(Arc<DeserializedScript>);

impl LocallyVerifiedScript {
    pub fn immediate_dependencies_iter(&self) -> impl DoubleEndedIterator<Item = &ModuleIdx> {
        self.0.deps.iter()
    }
}

#[derive(Debug)]
pub struct DeserializedModule {
    pub compiled_module: Arc<CompiledModule>,
    pub deps: Arc<Vec<ModuleIdx>>,
    pub friends: Arc<Vec<ModuleIdx>>,
}

#[derive(Debug)]
pub struct DeserializedScript {
    pub compiled_script: Arc<CompiledScript>,
    pub deps: Arc<Vec<ModuleIdx>>,
}
