// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::VMConfig,
    native_functions::{NativeFunction, NativeFunctions},
    storage::{
        ty_layout_converter::LAYOUT_CACHE,
        ty_tag_converter::{TypeTagCache, TypeTagConverter},
        verified_module_cache::VERIFIED_MODULES_CACHE,
    },
    Module, Script,
};
use ambassador::delegatable_trait;
use bytes::Bytes;
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    errors::{verification_error, Location, PartialVMError, PartialVMResult, VMResult},
    file_format::{CompiledScript, StructFieldInformation, TableIndex},
    CompiledModule, IndexKind,
};
use move_bytecode_verifier::dependencies;
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, TypeTag},
    vm_status::{sub_status::unknown_invariant_violation::EPARANOID_FAILURE, StatusCode},
};
use move_vm_metrics::{Timer, VERIFIED_MODULE_CACHE_SIZE, VM_TIMER};
#[cfg(any(test, feature = "testing"))]
use move_vm_types::loaded_data::{
    runtime_types::StructIdentifier, struct_name_indexing::StructNameIndex,
};
use move_vm_types::loaded_data::{runtime_types::Type, struct_name_indexing::StructNameIndexMap};
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
    struct_name_index_map: Arc<StructNameIndexMap>,

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
            paranoid_type_checks: false,
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
        LAYOUT_CACHE.clear();
        let natives = NativeFunctions::new(natives)
            .unwrap_or_else(|e| panic!("Failed to create native functions: {}", e));
        Self {
            vm_config,
            natives,
            struct_name_index_map: Arc::new(StructNameIndexMap::empty()),
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
        compiled_script: Arc<CompiledScript>,
    ) -> VMResult<LocallyVerifiedScript> {
        move_bytecode_verifier::verify_script_with_config(
            &self.vm_config().verifier_config,
            compiled_script.as_ref(),
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
            &self.vm_config.verifier_config,
            locally_verified_script.0.as_ref(),
            immediate_dependencies
                .iter()
                .map(|module| module.as_ref().as_ref()),
        )?;
        Script::new(locally_verified_script.0, self.struct_name_index_map())
            .map_err(|err| err.finish(Location::Script))
    }

    /// Creates a locally verified compiled module by running:
    ///   1. Move bytecode verifier,
    ///   2. Verifier extension, if provided.
    pub fn build_locally_verified_module(
        &self,
        compiled_module: Arc<CompiledModule>,
        module_size: usize,
        module_hash: &[u8; 32],
    ) -> VMResult<LocallyVerifiedModule> {
        if !VERIFIED_MODULES_CACHE.contains(module_hash) {
            let _timer =
                VM_TIMER.timer_with_label("move_bytecode_verifier::verify_module_with_config");

            // For regular execution, we cache already verified modules. Note that this even caches
            // verification for the published modules. This should be ok because as long as the
            // hash is the same, the deployed bytecode and any dependencies are the same, and so
            // the cached verification result can be used.
            move_bytecode_verifier::verify_module_with_config(
                &self.vm_config().verifier_config,
                compiled_module.as_ref(),
            )?;
            check_natives(compiled_module.as_ref())?;
            VERIFIED_MODULES_CACHE.put(*module_hash);
        }

        Ok(LocallyVerifiedModule(compiled_module, module_size))
    }

    /// Creates a verified module by running dependency verification pass for a locally verified
    /// module. The caller must provide verified module dependencies.
    pub(crate) fn build_verified_module_with_linking_checks(
        &self,
        locally_verified_module: LocallyVerifiedModule,
        immediate_dependencies: &[Arc<Module>],
    ) -> VMResult<Module> {
        dependencies::verify_module(
            &self.vm_config.verifier_config,
            locally_verified_module.0.as_ref(),
            immediate_dependencies
                .iter()
                .map(|module| module.as_ref().as_ref()),
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

    /// Creates a verified module for a locally verified module. Does not perform linking checks
    /// for module's verified dependencies.
    pub(crate) fn build_verified_module_skip_linking_checks(
        &self,
        locally_verified_module: LocallyVerifiedModule,
    ) -> VMResult<Module> {
        Module::new(
            &self.natives,
            locally_verified_module.1,
            locally_verified_module.0,
            self.struct_name_index_map(),
        )
        .map_err(|err| err.finish(Location::Undefined))
    }

    /// Deserializes bytes into a compiled module.
    pub fn deserialize_into_compiled_module(&self, bytes: &Bytes) -> VMResult<CompiledModule> {
        CompiledModule::deserialize_with_config(bytes, &self.vm_config().deserializer_config)
            .map_err(|err| {
                let msg = format!("Deserialization error: {:?}", err);
                PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR)
                    .with_message(msg)
                    .finish(Location::Undefined)
            })
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
    #[allow(dead_code)]
    pub(crate) fn natives(&self) -> &NativeFunctions {
        &self.natives
    }

    /// Returns the re-indexing map currently used by this runtime environment to remap struct
    /// identifiers into indices.
    pub(crate) fn struct_name_index_map(&self) -> &StructNameIndexMap {
        &self.struct_name_index_map
    }

    /// Returns the type tag cache used by this environment to store already constructed struct
    /// tags.
    pub(crate) fn ty_tag_cache(&self) -> &TypeTagCache {
        &self.ty_tag_cache
    }

    /// Returns the type tag for the given type. Construction of the tag can fail if it is too
    /// "complex": i.e., too deeply nested, or has large struct identifiers.
    pub fn ty_to_ty_tag(&self, ty: &Type) -> PartialVMResult<TypeTag> {
        let ty_tag_builder = TypeTagConverter::new(self);
        ty_tag_builder.ty_to_ty_tag(ty)
    }

    /// If type is a (generic or non-generic) struct or enum, returns its name. Otherwise, returns
    /// [None].
    pub fn get_struct_name(&self, ty: &Type) -> PartialVMResult<Option<(ModuleId, Identifier)>> {
        use Type::*;

        Ok(match ty {
            Struct { idx, .. } | StructInstantiation { idx, .. } => {
                let struct_identifier = self.struct_name_index_map().idx_to_struct_name(*idx)?;
                Some((struct_identifier.module, struct_identifier.name))
            },
            Bool
            | U8
            | U16
            | U32
            | U64
            | U128
            | U256
            | Address
            | Signer
            | TyParam(_)
            | Vector(_)
            | Reference(_)
            | MutableReference(_)
            | Function { .. } => None,
        })
    }

    /// Returns the size of the struct name re-indexing cache. Can be used to bound the size of the
    /// cache at block boundaries.
    pub fn struct_name_index_map_size(&self) -> PartialVMResult<usize> {
        self.struct_name_index_map.checked_len()
    }

    /// Flushes the global caches with struct name indices and struct tags. Note that when calling
    /// this function, modules that still store indices into struct name cache must also be flushed.
    pub fn flush_struct_name_and_tag_caches(&self) {
        LAYOUT_CACHE.clear();
        self.ty_tag_cache.flush();
        self.struct_name_index_map.flush();
    }

    /// Flushes the global verified module cache. Should be used when verifier configuration has
    /// changed.
    pub fn flush_verified_module_cache() {
        VERIFIED_MODULES_CACHE.flush();
    }

    /// Logs the size of the verified module cache.
    pub fn log_verified_cache_size() {
        let size = VERIFIED_MODULES_CACHE.size();
        VERIFIED_MODULE_CACHE_SIZE.set(size as i64);
    }

    /// Test-only function to be able to populate [StructNameIndexMap] outside of this crate.
    #[cfg(any(test, feature = "testing"))]
    pub fn struct_name_to_idx_for_test(
        &self,
        struct_name: StructIdentifier,
    ) -> PartialVMResult<StructNameIndex> {
        self.struct_name_index_map.struct_name_to_idx(&struct_name)
    }

    /// Test-only function to be able to check cached struct names.
    #[cfg(any(test, feature = "testing"))]
    pub fn idx_to_struct_name_for_test(
        &self,
        idx: StructNameIndex,
    ) -> PartialVMResult<StructIdentifier> {
        self.struct_name_index_map.idx_to_struct_name(idx)
    }
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
pub struct LocallyVerifiedModule(Arc<CompiledModule>, usize);

impl LocallyVerifiedModule {
    pub fn immediate_dependencies_iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (&AccountAddress, &IdentStr)> {
        self.0.immediate_dependencies_iter()
    }
}

/// Compiled script that passed local bytecode verification, but not the linking checks.
pub struct LocallyVerifiedScript(Arc<CompiledScript>);

impl LocallyVerifiedScript {
    pub fn immediate_dependencies_iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (&AccountAddress, &IdentStr)> {
        self.0.immediate_dependencies_iter()
    }
}

fn check_natives(module: &CompiledModule) -> VMResult<()> {
    // TODO: fix check and error code if we leave something around for native structs.
    // For now this generates the only error test cases care about...
    for (idx, struct_def) in module.struct_defs().iter().enumerate() {
        if struct_def.field_information == StructFieldInformation::Native {
            return Err(verification_error(
                StatusCode::MISSING_DEPENDENCY,
                IndexKind::FunctionHandle,
                idx as TableIndex,
            )
            .finish(Location::Module(module.self_id())));
        }
    }
    Ok(())
}
