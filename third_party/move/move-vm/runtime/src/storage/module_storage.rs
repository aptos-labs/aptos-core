// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    loader::{Function, Module},
    logging::expect_no_verification_errors,
    storage::environment::DeserializedModule,
    WithRuntimeEnvironment,
};
use ambassador::delegatable_trait;
use bytes::Bytes;
use hashbrown::HashSet;
use move_binary_format::{
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
    CompiledModule,
};
use move_core_types::{
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    metadata::Metadata,
    vm_status::StatusCode,
};
use move_vm_metrics::{Timer, VM_TIMER};
use move_vm_types::{
    code::{ModuleCache, ModuleCode, ModuleCodeBuilder, WithBytes, WithHash, WithSize},
    indices::{FunctionIdx, ModuleIdx, StructIdx},
    loaded_data::runtime_types::{StructType, Type},
    module_cyclic_dependency_error, module_linker_error,
    value_serde::FunctionValueExtension,
};
use std::sync::Arc;

/// Represents module storage backend, abstracting away any caching behaviour. The clients can
/// implement their own module storage to pass to the VM to resolve code.
#[delegatable_trait]
pub trait ModuleStorage: WithRuntimeEnvironment {
    /// Returns true if loader V2 implementation is enabled. Will be removed in the future, for now
    /// it is simply a convenient way to check the feature flag if module storage is available.
    // TODO(loader_v2): Remove this when loader V2 is enabled.
    fn is_enabled(&self) -> bool {
        self.runtime_environment().vm_config().use_loader_v2
    }

    /// Returns true if the module exists, and false otherwise. An error is returned if there is a
    /// storage error.
    fn check_module_exists(&self, idx: &ModuleIdx) -> VMResult<bool>;

    /// Returns module bytes if module exists, or [None] otherwise. An error is returned if there
    /// is a storage error.
    fn fetch_module_bytes(&self, idx: &ModuleIdx) -> VMResult<Option<Bytes>>;

    /// Returns the size of a module in bytes, or [None] otherwise. An error is returned if the
    /// there is a storage error.
    fn fetch_module_size_in_bytes(&self, idx: &ModuleIdx) -> VMResult<Option<usize>>;

    /// Returns the metadata in the module, or [None] otherwise. An error is returned if there is
    /// a storage error or the module fails deserialization.
    fn fetch_module_metadata(&self, idx: &ModuleIdx) -> VMResult<Option<Vec<Metadata>>>;

    /// Returns the metadata in the module. An error is returned if there is a storage error,
    /// module fails deserialization, or does not exist.
    fn fetch_existing_module_metadata(&self, idx: &ModuleIdx) -> VMResult<Vec<Metadata>> {
        self.fetch_module_metadata(idx)?
            .ok_or_else(|| module_linker_error!(idx, idx))
    }

    fn fetch_existing_module_dependencies(&self, idx: &ModuleIdx) -> VMResult<Arc<Vec<ModuleIdx>>>;

    fn fetch_existing_module_friends(&self, idx: &ModuleIdx) -> VMResult<Arc<Vec<ModuleIdx>>>;

    /// Returns the deserialized module, or [None] otherwise. An error is returned if:
    ///   1. the deserialization fails, or
    ///   2. there is an error from the underlying storage.
    fn fetch_deserialized_module(&self, idx: &ModuleIdx) -> VMResult<Option<Arc<CompiledModule>>>;

    /// Returns the deserialized module. An error is returned if:
    ///   1. the deserialization fails,
    ///   2. there is an error from the underlying storage,
    ///   3. module does not exist.
    fn fetch_existing_deserialized_module(&self, idx: &ModuleIdx) -> VMResult<Arc<CompiledModule>> {
        self.fetch_deserialized_module(idx)?
            .ok_or_else(|| module_linker_error!(idx, idx))
    }

    /// Returns the verified module if it exists, or [None] otherwise. The existing module can be
    /// either in a cached state (it is then returned) or newly constructed. The error is returned
    /// if the storage fails to fetch the deserialized module and verify it.
    fn fetch_verified_module(&self, idx: &ModuleIdx) -> VMResult<Option<Arc<Module>>>;

    /// Returns the verified module. If it does not exist, a linker error is returned. All other
    /// errors are mapped using [expect_no_verification_errors] - since on-chain code should not
    /// fail bytecode verification.
    fn fetch_existing_verified_module(&self, idx: &ModuleIdx) -> VMResult<Arc<Module>> {
        self.fetch_verified_module(idx)
            .map_err(expect_no_verification_errors)?
            .ok_or_else(|| module_linker_error!(idx, idx))
    }

    /// Returns a struct type corresponding to the specified name. The module containing the struct
    /// will be fetched and cached beforehand.
    fn fetch_struct_ty(&self, struct_idx: &StructIdx) -> PartialVMResult<Arc<StructType>> {
        let module = self
            .fetch_existing_verified_module(&struct_idx.module_idx())
            .map_err(|err| err.to_partial())?;
        Ok(module
            .struct_map
            .get(struct_idx)
            .and_then(|idx| module.structs.get(*idx))
            .ok_or_else(|| {
                let index_manager = self.runtime_environment().struct_name_index_map();
                let module_id = index_manager.module_id_from_struct_idx(struct_idx);
                let struct_name = index_manager.struct_name_from_idx(struct_idx);
                PartialVMError::new(StatusCode::TYPE_RESOLUTION_FAILURE).with_message(format!(
                    "Struct {}::{}::{} does not exist",
                    module_id.address(),
                    module_id.name(),
                    struct_name
                ))
            })?
            .definition_struct_type
            .clone())
    }

    /// Returns a runtime type corresponding to the specified type tag (file format type
    /// representation). If a struct type is constructed, the module containing the struct
    /// definition is fetched and cached.
    fn fetch_ty(&self, ty_tag: &TypeTag) -> PartialVMResult<Type> {
        // TODO(loader_v2): Loader V1 uses VMResults everywhere, but partial VM errors
        //                  seem better fit. Here we map error to VMError to reuse existing
        //                  type builder implementation, and then strip the location info.
        self.runtime_environment()
            .vm_config()
            .ty_builder
            .create_ty(ty_tag, |st| {
                let idx = self
                    .runtime_environment()
                    .struct_name_index_map()
                    .struct_idx_from_struct_tag(st);
                self.fetch_struct_ty(&idx)
                    .map_err(|err| err.finish(Location::Undefined))
            })
            .map_err(|err| err.to_partial())
    }

    /// Returns the function definition corresponding to the specified name, as well as the module
    /// where this function is defined. The returned function can contain uninstantiated generic
    /// types and its signature. The returned module is verified.
    fn fetch_function_definition(
        &self,
        function_idx: &FunctionIdx,
    ) -> VMResult<(Arc<Module>, Arc<Function>)> {
        let module = self.fetch_existing_verified_module(&function_idx.module_idx())?;
        let function = module
            .function_map
            .get(function_idx)
            .and_then(|idx| module.function_defs.get(*idx))
            .ok_or_else(|| {
                let index_manager = self.runtime_environment().struct_name_index_map();
                let module_id = index_manager.module_id_from_idx(function_idx);
                let function_name = index_manager.function_name_from_idx(function_idx);
                PartialVMError::new(StatusCode::FUNCTION_RESOLUTION_FAILURE)
                    .with_message(format!(
                        "Function {}::{}::{} does not exist",
                        module_id.address(),
                        module_id.name(),
                        function_name
                    ))
                    .finish(Location::Undefined)
            })?
            .clone();
        Ok((module, function))
    }
}

impl<T, E, V> ModuleStorage for T
where
    T: WithRuntimeEnvironment
        + ModuleCache<
            Key = ModuleIdx,
            Deserialized = DeserializedModule,
            Verified = Module,
            Extension = E,
            Version = V,
        > + ModuleCodeBuilder<
            Key = ModuleIdx,
            Deserialized = DeserializedModule,
            Verified = Module,
            Extension = E,
        >,
    E: WithBytes + WithSize + WithHash,
    V: Clone + Default + Ord,
{
    fn check_module_exists(&self, idx: &ModuleIdx) -> VMResult<bool> {
        Ok(self.get_module_or_build_with(idx, self)?.is_some())
    }

    fn fetch_module_bytes(&self, idx: &ModuleIdx) -> VMResult<Option<Bytes>> {
        Ok(self
            .get_module_or_build_with(idx, self)?
            .map(|(module, _)| module.extension().bytes().clone()))
    }

    fn fetch_module_size_in_bytes(&self, idx: &ModuleIdx) -> VMResult<Option<usize>> {
        Ok(self
            .get_module_or_build_with(idx, self)?
            .map(|(module, _)| module.extension().bytes().len()))
    }

    fn fetch_module_metadata(&self, idx: &ModuleIdx) -> VMResult<Option<Vec<Metadata>>> {
        Ok(self
            .get_module_or_build_with(idx, self)?
            .map(|(module, _)| {
                module
                    .code()
                    .deserialized()
                    .compiled_module
                    .metadata
                    .clone()
            }))
    }

    fn fetch_existing_module_dependencies(&self, idx: &ModuleIdx) -> VMResult<Arc<Vec<ModuleIdx>>> {
        let module = self
            .get_module_or_build_with(idx, self)?
            .ok_or_else(|| module_linker_error!(0, 0))?
            .0;
        Ok(module.code().deserialized().deps.clone())
    }

    fn fetch_existing_module_friends(&self, idx: &ModuleIdx) -> VMResult<Arc<Vec<ModuleIdx>>> {
        let module = self
            .get_module_or_build_with(idx, self)?
            .ok_or_else(|| module_linker_error!(0, 0))?
            .0;
        Ok(module.code().deserialized().friends.clone())
    }

    fn fetch_deserialized_module(&self, idx: &ModuleIdx) -> VMResult<Option<Arc<CompiledModule>>> {
        Ok(self
            .get_module_or_build_with(idx, self)?
            .map(|(module, _)| module.code().deserialized().compiled_module.clone()))
    }

    fn fetch_verified_module(&self, idx: &ModuleIdx) -> VMResult<Option<Arc<Module>>> {
        // Look up the verified module in cache, if it is not there, or if the module is not yet
        // verified, we need to load & verify its transitive dependencies.
        let (module, version) = match self.get_module_or_build_with(idx, self)? {
            Some(module_and_version) => module_and_version,
            None => return Ok(None),
        };

        if module.code().is_verified() {
            return Ok(Some(module.code().verified().clone()));
        }

        let _timer = VM_TIMER.timer_with_label("ModuleStorage::fetch_verified_module [cache miss]");

        let mut visited = HashSet::new();
        visited.insert(*idx);
        Ok(Some(visit_dependencies_and_verify(
            *idx,
            module,
            version,
            &mut visited,
            self,
        )?))
    }
}

/// Visits the dependencies of the given module. If dependencies form a cycle (which should not be
/// the case as we check this when modules are added to the module cache), an error is returned.
///
/// Note:
///   This implementation **does not** load transitive friends. While it is possible to view
///   friends as `used-by` relation, it cannot be checked fully. For example, consider the case
///   when we have four modules A, B, C, D and let `X --> Y` be a dependency relation (Y is a
///   dependency of X) and `X ==> Y ` a friend relation (X declares Y a friend). Then consider the
///   case `A --> B <== C --> D <== A`. Here, if we opt for `used-by` semantics, there is a cycle.
///   But it cannot be checked, since, A only sees B and D, and C sees B and D, but both B and D do
///   not see any dependencies or friends. Hence, A cannot discover C and vice-versa, making
///   detection of such corner cases only possible if **all existing modules are checked**, which
///   is clearly infeasible.
fn visit_dependencies_and_verify<T, E, V>(
    module_id: ModuleIdx,
    module: Arc<ModuleCode<DeserializedModule, Module, E>>,
    version: V,
    visited: &mut HashSet<ModuleIdx>,
    module_cache_with_context: &T,
) -> VMResult<Arc<Module>>
where
    T: WithRuntimeEnvironment
        + ModuleCache<
            Key = ModuleIdx,
            Deserialized = DeserializedModule,
            Verified = Module,
            Extension = E,
            Version = V,
        > + ModuleCodeBuilder<
            Key = ModuleIdx,
            Deserialized = DeserializedModule,
            Verified = Module,
            Extension = E,
        >,
    E: WithBytes + WithSize + WithHash,
    V: Clone + Default + Ord,
{
    let runtime_environment = module_cache_with_context.runtime_environment();

    // Step 1: Local verification.
    // FIXME
    // runtime_environment.paranoid_check_module_address_and_name(
    //     module.code().deserialized(),
    //     module_id.address(),
    //     module_id.name(),
    // )?;
    let locally_verified_code = runtime_environment.build_locally_verified_module(
        module.code().deserialized().clone(),
        module.extension().size_in_bytes(),
        module.extension().hash(),
    )?;

    // Step 2: Traverse and collect all verified immediate dependencies so that we can verify
    // non-local properties of the module.
    let mut verified_dependencies = vec![];
    for dependency_id in locally_verified_code.immediate_dependencies_iter() {
        let (dependency, dependency_version) = module_cache_with_context
            .get_module_or_build_with(dependency_id, module_cache_with_context)?
            .ok_or_else(|| module_linker_error!(0, 0))?;

        // Dependency is already verified!
        if dependency.code().is_verified() {
            verified_dependencies.push(dependency.code().verified().clone());
            continue;
        }

        if visited.insert(*dependency_id) {
            // Dependency is not verified, and we have not visited it yet.
            let verified_dependency = visit_dependencies_and_verify(
                *dependency_id,
                dependency,
                dependency_version,
                visited,
                module_cache_with_context,
            )?;
            verified_dependencies.push(verified_dependency);
        } else {
            // We must have found a cycle otherwise.
            return Err(module_cyclic_dependency_error!(0, 0));
        }
    }

    let verified_code =
        runtime_environment.build_verified_module(locally_verified_code, &verified_dependencies)?;
    let module = module_cache_with_context.insert_verified_module(
        module_id,
        verified_code,
        module.extension().clone(),
        version,
    )?;
    Ok(module.code().verified().clone())
}

/// Avoids the orphan rule to implement external [FunctionValueExtension] for any generic type that
/// implements [ModuleStorage].
pub struct FunctionValueExtensionAdapter<'a> {
    #[allow(dead_code)]
    pub(crate) module_storage: &'a dyn ModuleStorage,
}

pub trait AsFunctionValueExtension {
    fn as_function_value_extension(&self) -> FunctionValueExtensionAdapter;
}

impl<T: ModuleStorage> AsFunctionValueExtension for T {
    fn as_function_value_extension(&self) -> FunctionValueExtensionAdapter {
        FunctionValueExtensionAdapter {
            module_storage: self,
        }
    }
}

impl<'a> FunctionValueExtension for FunctionValueExtensionAdapter<'a> {
    fn get_function_arg_tys(
        &self,
        _module_id: &ModuleId,
        _function_name: &IdentStr,
        _substitution_ty_arg_tags: Vec<TypeTag>,
    ) -> PartialVMResult<Vec<Type>> {
        unimplemented!()
        // let substitution_ty_args = substitution_ty_arg_tags
        //     .into_iter()
        //     .map(|tag| self.module_storage.fetch_ty(&tag))
        //     .collect::<PartialVMResult<Vec<_>>>()?;
        //
        // let (_, function) = self
        //     .module_storage
        //     .fetch_function_definition(module_id.address(), module_id.name(), function_name)
        //     .map_err(|err| err.to_partial())?;
        //
        // let ty_builder = &self
        //     .module_storage
        //     .runtime_environment()
        //     .vm_config()
        //     .ty_builder;
        // function
        //     .param_tys()
        //     .iter()
        //     .map(|ty_to_substitute| {
        //         ty_builder.create_ty_with_subst(ty_to_substitute, &substitution_ty_args)
        //     })
        //     .collect::<PartialVMResult<Vec<_>>>()
    }
}
