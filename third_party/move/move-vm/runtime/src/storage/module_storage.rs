// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    loader::{Function, LazyLoadedFunction, LazyLoadedFunctionState, LoadedFunctionOwner, Module},
    logging::expect_no_verification_errors,
    LayoutConverter, LoadedFunction, StorageLayoutConverter, WithRuntimeEnvironment,
};
use ambassador::delegatable_trait;
use bytes::Bytes;
use hashbrown::HashSet;
use move_binary_format::{
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
    CompiledModule,
};
#[cfg(fuzzing)]
use move_binary_format::access::ModuleAccess;
use move_core_types::{
    account_address::AccountAddress,
    function::FUNCTION_DATA_SERIALIZATION_FORMAT_V1,
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    metadata::Metadata,
    vm_status::StatusCode,
};
use move_vm_metrics::{Timer, VM_TIMER};
use move_vm_types::{
    code::{ModuleCache, ModuleCode, ModuleCodeBuilder, WithBytes, WithHash, WithSize},
    loaded_data::{
        runtime_types::{StructType, Type},
        struct_name_indexing::StructNameIndex,
    },
    module_cyclic_dependency_error, module_linker_error,
    value_serde::FunctionValueExtension,
    values::{AbstractFunction, SerializedFunctionData},
};
use std::sync::Arc;

/// Represents module storage backend, abstracting away any caching behaviour. The clients can
/// implement their own module storage to pass to the VM to resolve code.
#[delegatable_trait]
pub trait ModuleStorage: WithRuntimeEnvironment {
    /// Returns true if the module exists, and false otherwise. An error is returned if there is a
    /// storage error.
    fn check_module_exists(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<bool>;

    /// Returns module bytes if module exists, or [None] otherwise. An error is returned if there
    /// is a storage error.
    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>>;

    /// Returns the size of a module in bytes, or [None] otherwise. An error is returned if the
    /// there is a storage error.
    fn fetch_module_size_in_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<usize>>;

    /// Returns the metadata in the module, or [None] otherwise. An error is returned if there is
    /// a storage error or the module fails deserialization.
    fn fetch_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Vec<Metadata>>>;

    /// Returns the metadata in the module. An error is returned if there is a storage error,
    /// module fails deserialization, or does not exist.
    fn fetch_existing_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Vec<Metadata>> {
        self.fetch_module_metadata(address, module_name)?
            .ok_or_else(|| module_linker_error!(address, module_name))
    }

    /// Returns the deserialized module, or [None] otherwise. An error is returned if:
    ///   1. the deserialization fails, or
    ///   2. there is an error from the underlying storage.
    fn fetch_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<CompiledModule>>>;

    /// Returns the deserialized module. An error is returned if:
    ///   1. the deserialization fails,
    ///   2. there is an error from the underlying storage,
    ///   3. module does not exist.
    fn fetch_existing_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Arc<CompiledModule>> {
        self.fetch_deserialized_module(address, module_name)?
            .ok_or_else(|| module_linker_error!(address, module_name))
    }

    /// Returns the verified module if it exists, or [None] otherwise. The existing module can be
    /// either in a cached state (it is then returned) or newly constructed. The error is returned
    /// if the storage fails to fetch the deserialized module and verify it.
    fn fetch_verified_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<Module>>>;

    /// Returns the verified module. If it does not exist, a linker error is returned. All other
    /// errors are mapped using [expect_no_verification_errors] - since on-chain code should not
    /// fail bytecode verification.
    fn fetch_existing_verified_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Arc<Module>> {
        self.fetch_verified_module(address, module_name)
            .map_err(expect_no_verification_errors)?
            .ok_or_else(|| module_linker_error!(address, module_name))
    }

    /// Returns the module without verification, or [None] otherwise. The existing module can be
    /// either in a cached state (it is then returned) or newly constructed. The error is returned
    /// if the storage fails to fetch the deserialized module.
    #[cfg(fuzzing)]
    fn fetch_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<Module>>>;

    /// Returns the module without verification. If it does not exist, a linker error is returned.
    #[cfg(fuzzing)]
    fn fetch_existing_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Arc<Module>> {
        self.fetch_module(address, module_name)?
            .ok_or_else(|| module_linker_error!(address, module_name))
    }

    /// Returns a struct type corresponding to the specified name. The module containing the struct
    /// will be fetched and cached beforehand.
    fn fetch_struct_ty(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
        struct_name: &IdentStr,
    ) -> PartialVMResult<Arc<StructType>> {
        let module = self
            .fetch_existing_verified_module(address, module_name)
            .map_err(|err| err.to_partial())?;
        module
            .get_struct(struct_name)
            .map_err(|err| err.to_partial())
    }

    fn fetch_struct_ty_by_idx(&self, idx: &StructNameIndex) -> PartialVMResult<Arc<StructType>> {
        let struct_name = self
            .runtime_environment()
            .struct_name_index_map()
            .idx_to_struct_name_ref(*idx)?;

        self.fetch_struct_ty(
            struct_name.module.address(),
            struct_name.module.name(),
            struct_name.name.as_ident_str(),
        )
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
                self.fetch_struct_ty(
                    &st.address,
                    st.module.as_ident_str(),
                    st.name.as_ident_str(),
                )
                .map_err(|err| err.finish(Location::Undefined))
            })
            .map_err(|err| err.to_partial())
    }

    /// Returns the function definition corresponding to the specified name, as well as the module
    /// where this function is defined. The returned function can contain uninstantiated generic
    /// types and its signature. The returned module is verified.
    fn fetch_function_definition(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
        function_name: &IdentStr,
    ) -> VMResult<(Arc<Module>, Arc<Function>)> {
        let module = self.fetch_existing_verified_module(address, module_name)?;
        let function = module.get_function(function_name)?;
        Ok((module, function))
    }

    fn load_function(
        &self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: &[TypeTag],
    ) -> VMResult<LoadedFunction> {
        let _timer = VM_TIMER.timer_with_label("Loader::load_function");

        let (module, function) =
            self.fetch_function_definition(module_id.address(), module_id.name(), function_name)?;

        let ty_args = ty_args
            .iter()
            .map(|ty_arg| self.fetch_ty(ty_arg).map_err(|e| e.finish(Location::Undefined)))
            .collect::<VMResult<Vec<_>>>()
            .map_err(|mut err| {
                // User provided type argument failed to load. Set extra sub status to distinguish from internal type loading error.
                if StatusCode::TYPE_RESOLUTION_FAILURE == err.major_status() {
                    err.set_sub_status(move_core_types::vm_status::sub_status::type_resolution_failure::EUSER_TYPE_LOADING_FAILURE);
                }
                err
            })?;

        Type::verify_ty_arg_abilities(function.ty_param_abilities(), &ty_args)
            .map_err(|e| e.finish(Location::Module(module_id.clone())))?;

        Ok(LoadedFunction {
            owner: LoadedFunctionOwner::Module(module),
            ty_args,
            function,
        })
    }
}

impl<T, E, V> ModuleStorage for T
where
    T: WithRuntimeEnvironment
        + ModuleCache<
            Key = ModuleId,
            Deserialized = CompiledModule,
            Verified = Module,
            Extension = E,
            Version = V,
        > + ModuleCodeBuilder<
            Key = ModuleId,
            Deserialized = CompiledModule,
            Verified = Module,
            Extension = E,
        >,
    E: WithBytes + WithSize + WithHash,
    V: Clone + Default + Ord,
{
    fn check_module_exists(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<bool> {
        let id = ModuleId::new(*address, module_name.to_owned());
        Ok(self.get_module_or_build_with(&id, self)?.is_some())
    }

    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>> {
        let id = ModuleId::new(*address, module_name.to_owned());
        Ok(self
            .get_module_or_build_with(&id, self)?
            .map(|(module, _)| module.extension().bytes().clone()))
    }

    fn fetch_module_size_in_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<usize>> {
        let id = ModuleId::new(*address, module_name.to_owned());
        Ok(self
            .get_module_or_build_with(&id, self)?
            .map(|(module, _)| module.extension().bytes().len()))
    }

    fn fetch_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Vec<Metadata>>> {
        let id = ModuleId::new(*address, module_name.to_owned());
        Ok(self
            .get_module_or_build_with(&id, self)?
            .map(|(module, _)| module.code().deserialized().metadata.clone()))
    }

    fn fetch_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<CompiledModule>>> {
        let id = ModuleId::new(*address, module_name.to_owned());
        Ok(self
            .get_module_or_build_with(&id, self)?
            .map(|(module, _)| module.code().deserialized().clone()))
    }

    fn fetch_verified_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<Module>>> {
        let id = ModuleId::new(*address, module_name.to_owned());

        // Look up the verified module in cache, if it is not there, or if the module is not yet
        // verified, we need to load & verify its transitive dependencies.
        let (module, version) = match self.get_module_or_build_with(&id, self)? {
            Some(module_and_version) => module_and_version,
            None => return Ok(None),
        };

        if module.code().is_verified() {
            return Ok(Some(module.code().verified().clone()));
        }

        let _timer = VM_TIMER.timer_with_label("ModuleStorage::fetch_verified_module [cache miss]");

        let mut visited = HashSet::new();
        visited.insert(id.clone());
        Ok(Some(visit_dependencies_and_verify(
            id,
            module,
            version,
            &mut visited,
            self,
        )?))
    }

    #[cfg(fuzzing)]
    fn fetch_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<Module>>> {
        let id = ModuleId::new(*address, module_name.to_owned());

        // Look up the module in cache, if it is not there, we need to load it
        let (module, version) = match self.get_module_or_build_with(&id, self)? {
            Some(module_and_version) => module_and_version,
            None => return Ok(None),
        };

        // If module is already verified, return it
        if module.code().is_verified() {
            return Ok(Some(module.code().verified().clone()));
        }

        // Otherwise, load the module and its dependencies without verification
        let mut visited = HashSet::new();
        visited.insert(id.clone());
        Ok(Some(visit_dependencies_and_load(
            id,
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
    module_id: ModuleId,
    module: Arc<ModuleCode<CompiledModule, Module, E>>,
    version: V,
    visited: &mut HashSet<ModuleId>,
    module_cache_with_context: &T,
) -> VMResult<Arc<Module>>
where
    T: WithRuntimeEnvironment
        + ModuleCache<
            Key = ModuleId,
            Deserialized = CompiledModule,
            Verified = Module,
            Extension = E,
            Version = V,
        > + ModuleCodeBuilder<
            Key = ModuleId,
            Deserialized = CompiledModule,
            Verified = Module,
            Extension = E,
        >,
    E: WithBytes + WithSize + WithHash,
    V: Clone + Default + Ord,
{
    let runtime_environment = module_cache_with_context.runtime_environment();

    // Step 1: Local verification.
    runtime_environment.paranoid_check_module_address_and_name(
        module.code().deserialized(),
        module_id.address(),
        module_id.name(),
    )?;
    let locally_verified_code = runtime_environment.build_locally_verified_module(
        module.code().deserialized().clone(),
        module.extension().size_in_bytes(),
        module.extension().hash(),
    )?;

    // Step 2: Traverse and collect all verified immediate dependencies so that we can verify
    // non-local properties of the module.
    let mut verified_dependencies = vec![];
    for (addr, name) in locally_verified_code.immediate_dependencies_iter() {
        let dependency_id = ModuleId::new(*addr, name.to_owned());

        let (dependency, dependency_version) = module_cache_with_context
            .get_module_or_build_with(&dependency_id, module_cache_with_context)?
            .ok_or_else(|| module_linker_error!(addr, name))?;

        // Dependency is already verified!
        if dependency.code().is_verified() {
            verified_dependencies.push(dependency.code().verified().clone());
            continue;
        }

        if visited.insert(dependency_id.clone()) {
            // Dependency is not verified, and we have not visited it yet.
            let verified_dependency = visit_dependencies_and_verify(
                dependency_id.clone(),
                dependency,
                dependency_version,
                visited,
                module_cache_with_context,
            )?;
            verified_dependencies.push(verified_dependency);
        } else {
            // We must have found a cycle otherwise.
            return Err(module_cyclic_dependency_error!(
                dependency_id.address(),
                dependency_id.name()
            ));
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

/// Visits the dependencies of the given module and loads them without verification. If dependencies form a cycle,
/// an error is returned.
#[cfg(fuzzing)]
fn visit_dependencies_and_load<T, E, V>(
    module_id: ModuleId,
    module: Arc<ModuleCode<CompiledModule, Module, E>>,
    version: V,
    visited: &mut HashSet<ModuleId>,
    module_cache_with_context: &T,
) -> VMResult<Arc<Module>>
where
    T: WithRuntimeEnvironment
        + ModuleCache<
            Key = ModuleId,
            Deserialized = CompiledModule,
            Verified = Module,
            Extension = E,
            Version = V,
        > + ModuleCodeBuilder<
            Key = ModuleId,
            Deserialized = CompiledModule,
            Verified = Module,
            Extension = E,
        >,
    E: WithBytes + WithSize + WithHash,
    V: Clone + Default + Ord,
{
    let runtime_environment = module_cache_with_context.runtime_environment();

    // Step 2: Traverse and collect all immediate dependencies
    let mut loaded_dependencies = vec![];
    for (addr, name) in module.code().deserialized().immediate_dependencies_iter() {
        let dependency_id = ModuleId::new(*addr, name.to_owned());

        let (dependency, dependency_version) = module_cache_with_context
            .get_module_or_build_with(&dependency_id, module_cache_with_context)?
            .ok_or_else(|| module_linker_error!(addr, name))?;

        // If dependency is already loaded, use it
        if dependency.code().is_verified() {
            loaded_dependencies.push(dependency.code().verified().clone());
            continue;
        }

        if visited.insert(dependency_id.clone()) {
            // Dependency is not loaded, and we have not visited it yet
            let loaded_dependency = visit_dependencies_and_load(
                dependency_id.clone(),
                dependency,
                dependency_version,
                visited,
                module_cache_with_context,
            )?;
            loaded_dependencies.push(loaded_dependency);
        } else {
            // We must have found a cycle otherwise
            return Err(module_cyclic_dependency_error!(
                dependency_id.address(),
                dependency_id.name()
            ));
        }
    }

    // Create a new Module without verification
    let code = Module::new(
        runtime_environment.natives(),
        module.extension().size_in_bytes(),
        module.code().deserialized().clone(),
        runtime_environment.struct_name_index_map(),
    )
    .map_err(|e| e.finish(Location::Undefined))?;

    let verified_module = module_cache_with_context.insert_verified_module(
        module_id,
        code,
        module.extension().clone(),
        version,
    )?;

    Ok(verified_module.code().verified().clone())
}

/// Avoids the orphan rule to implement external [FunctionValueExtension] for any generic type that
/// implements [ModuleStorage].
pub struct FunctionValueExtensionAdapter<'a> {
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
    fn create_from_serialization_data(
        &self,
        data: SerializedFunctionData,
    ) -> PartialVMResult<Box<dyn AbstractFunction>> {
        Ok(Box::new(LazyLoadedFunction::new_unresolved(data)))
    }

    fn get_serialization_data(
        &self,
        fun: &dyn AbstractFunction,
    ) -> PartialVMResult<SerializedFunctionData> {
        match &*LazyLoadedFunction::expect_this_impl(fun)?.0.borrow() {
            LazyLoadedFunctionState::Unresolved { data, .. } => Ok(data.clone()),
            LazyLoadedFunctionState::Resolved { fun, mask, ty_args } => {
                let ty_converter = StorageLayoutConverter::new(self.module_storage);
                let ty_builder = &self
                    .module_storage
                    .runtime_environment()
                    .vm_config()
                    .ty_builder;
                let instantiate = |ty: &Type| -> PartialVMResult<Type> {
                    if fun.ty_args.is_empty() {
                        Ok(ty.clone())
                    } else {
                        ty_builder.create_ty_with_subst(ty, &fun.ty_args)
                    }
                };
                let captured_layouts = mask
                    .extract(fun.param_tys(), true)
                    .into_iter()
                    .map(|t| ty_converter.type_to_type_layout(&instantiate(t)?))
                    .collect::<PartialVMResult<Vec<_>>>()?;
                Ok(SerializedFunctionData {
                    format_version: FUNCTION_DATA_SERIALIZATION_FORMAT_V1,
                    module_id: fun
                        .module_id()
                        .ok_or_else(|| {
                            PartialVMError::new_invariant_violation(
                                "attempt to serialize a script function",
                            )
                        })?
                        .clone(),
                    fun_id: fun.function.name.clone(),
                    ty_args: ty_args.clone(),
                    mask: *mask,
                    captured_layouts,
                })
            },
        }
    }
}
