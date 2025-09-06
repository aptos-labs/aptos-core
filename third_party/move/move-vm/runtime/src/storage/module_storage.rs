// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    loader::{LazyLoadedFunction, LazyLoadedFunctionState, Module},
    logging::expect_no_verification_errors,
    WithRuntimeEnvironment,
};
use ambassador::delegatable_trait;
use bytes::Bytes;
use hashbrown::HashSet;
#[cfg(fuzzing)]
use move_binary_format::{access::ModuleAccess, errors::Location};
use move_binary_format::{
    errors::{PartialVMError, PartialVMResult, VMResult},
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress, function::FUNCTION_DATA_SERIALIZATION_FORMAT_V1,
    identifier::IdentStr, language_storage::ModuleId, metadata::Metadata, vm_status::StatusCode,
};
use move_vm_metrics::{Timer, VM_TIMER};
use move_vm_types::{
    code::{ModuleCache, ModuleCode, ModuleCodeBuilder, WithBytes, WithHash, WithSize},
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
    ///
    /// Note: this API is not metered!
    fn unmetered_check_module_exists(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<bool>;

    /// Returns module bytes if module exists, or [None] otherwise. An error is returned if there
    /// is a storage error.
    ///
    /// Note: this API is not metered!
    fn unmetered_get_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>>;

    /// Returns the size of a module in bytes, or [None] otherwise. An error is returned if the
    /// there is a storage error.
    ///
    /// Note: this API is not metered! It is only used to get the size of a module so that metering
    /// can actually be implemented before loading a module.
    fn unmetered_get_module_size(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<usize>>;

    /// Returns the size of a module in bytes, or an error if it does not exist. An error is also
    /// returned if the there is a storage error.
    ///
    /// Note: this API is not metered!
    fn unmetered_get_existing_module_size(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<usize> {
        self.unmetered_get_module_size(address, module_name)?
            .ok_or_else(|| module_linker_error!(address, module_name))
    }

    /// Returns the metadata in the module, or [None] otherwise. An error is returned if there is
    /// a storage error or the module fails deserialization.
    ///
    /// Note: this API is not metered!
    fn unmetered_get_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Vec<Metadata>>>;

    /// Returns the metadata in the module. An error is returned if there is a storage error,
    /// module fails deserialization, or does not exist.
    ///
    /// Note: this API is not metered!
    fn unmetered_get_existing_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Vec<Metadata>> {
        self.unmetered_get_module_metadata(address, module_name)?
            .ok_or_else(|| module_linker_error!(address, module_name))
    }

    /// Returns the deserialized module, or [None] otherwise. An error is returned if:
    ///   1. the deserialization fails, or
    ///   2. there is an error from the underlying storage.
    ///
    /// Note: this API is not metered!
    fn unmetered_get_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<CompiledModule>>>;

    /// Returns the deserialized module. An error is returned if:
    ///   1. the deserialization fails,
    ///   2. there is an error from the underlying storage,
    ///   3. module does not exist.
    ///
    /// Note: this API is not metered!
    fn unmetered_get_existing_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Arc<CompiledModule>> {
        self.unmetered_get_deserialized_module(address, module_name)?
            .ok_or_else(|| module_linker_error!(address, module_name))
    }

    /// Returns the verified module if it exists, or [None] otherwise. The existing module can be
    /// either in a cached state (it is then returned) or newly constructed. The error is returned
    /// if the storage fails to fetch the deserialized module and verify it. The verification is
    /// eager: i.e., it addition to local module verification there are also linking checks and
    /// verification of transitive dependencies.
    ///
    /// Note 1: this API is not metered!
    /// Note 2: this API is used before lazy loading was enabled!
    fn unmetered_get_eagerly_verified_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<Module>>>;

    /// Returns an eagerly verified module. If it does not exist, a linker error is returned. All
    /// other errors are mapped using [expect_no_verification_errors] - since on-chain code should
    /// not fail bytecode verification.
    ///
    /// Note 1: this API is not metered!
    /// Note 2: this API is used before lazy loading was enabled!
    fn unmetered_get_existing_eagerly_verified_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Arc<Module>> {
        self.unmetered_get_eagerly_verified_module(address, module_name)
            .map_err(expect_no_verification_errors)?
            .ok_or_else(|| module_linker_error!(address, module_name))
    }

    /// Returns the module without verification, or [None] otherwise. Also loads the transitive
    /// closure of dependencies (also without verification).
    #[cfg(fuzzing)]
    fn unmetered_get_module_skip_verification(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<Module>>>;

    /// Returns the verified module if it exists, or [None] otherwise. The existing module can be
    /// either in a cached state (it is then returned) or newly constructed. The error is returned
    /// if the storage fails to fetch the deserialized module and verify it. The verification is
    /// lazy: i.e., it is only local to the module without any linking checks.
    ///
    /// Note 1: this API is not metered!
    /// Note 2: this API is used after lazy loading was enabled!
    fn unmetered_get_lazily_verified_module(
        &self,
        module_id: &ModuleId,
    ) -> VMResult<Option<Arc<Module>>>;

    /// Returns an eagerly verified module. If it does not exist, a linker error is returned. All
    /// other errors are mapped using [expect_no_verification_errors] - since on-chain code should
    /// not fail bytecode verification.
    ///
    /// Note 1: this API is not metered!
    /// Note 2: this API is used after lazy loading was enabled!
    fn unmetered_get_existing_lazily_verified_module(
        &self,
        module_id: &ModuleId,
    ) -> VMResult<Arc<Module>> {
        self.unmetered_get_lazily_verified_module(module_id)
            .map_err(expect_no_verification_errors)?
            .ok_or_else(|| module_linker_error!(module_id.address(), module_id.name()))
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
    fn unmetered_check_module_exists(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<bool> {
        let id = ModuleId::new(*address, module_name.to_owned());
        Ok(self.get_module_or_build_with(&id, self)?.is_some())
    }

    fn unmetered_get_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>> {
        let id = ModuleId::new(*address, module_name.to_owned());
        Ok(self
            .get_module_or_build_with(&id, self)?
            .map(|(module, _)| module.extension().bytes().clone()))
    }

    fn unmetered_get_module_size(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<usize>> {
        let id = ModuleId::new(*address, module_name.to_owned());
        Ok(self
            .get_module_or_build_with(&id, self)?
            .map(|(module, _)| module.extension().bytes().len()))
    }

    fn unmetered_get_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Vec<Metadata>>> {
        let id = ModuleId::new(*address, module_name.to_owned());
        Ok(self
            .get_module_or_build_with(&id, self)?
            .map(|(module, _)| module.code().deserialized().metadata.clone()))
    }

    fn unmetered_get_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<CompiledModule>>> {
        let id = ModuleId::new(*address, module_name.to_owned());
        Ok(self
            .get_module_or_build_with(&id, self)?
            .map(|(module, _)| module.code().deserialized().clone()))
    }

    fn unmetered_get_eagerly_verified_module(
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

        let _timer =
            VM_TIMER.timer_with_label("unmetered_get_eagerly_verified_module [cache miss]");
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
    fn unmetered_get_module_skip_verification(
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
        Ok(Some(visit_dependencies_and_skip_verification(
            id,
            module,
            version,
            &mut visited,
            self,
        )?))
    }

    fn unmetered_get_lazily_verified_module(
        &self,
        module_id: &ModuleId,
    ) -> VMResult<Option<Arc<Module>>> {
        let (module, version) = match self.get_module_or_build_with(module_id, self)? {
            Some(module_and_version) => module_and_version,
            None => return Ok(None),
        };

        if module.code().is_verified() {
            return Ok(Some(module.code().verified().clone()));
        }

        let _timer = VM_TIMER.timer_with_label("unmetered_get_lazily_verified_module [cache miss]");
        let runtime_environment = self.runtime_environment();
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
        let verified_code =
            runtime_environment.build_verified_module_skip_linking_checks(locally_verified_code)?;
        let verified_module = self.insert_verified_module(
            module_id.clone(),
            verified_code,
            module.extension().clone(),
            version,
        )?;
        Ok(Some(verified_module.code().verified().clone()))
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

    let verified_code = runtime_environment
        .build_verified_module_with_linking_checks(locally_verified_code, &verified_dependencies)?;
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
fn visit_dependencies_and_skip_verification<T, E, V>(
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
            let loaded_dependency = visit_dependencies_and_skip_verification(
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
    #[allow(dead_code)]
    pub module_storage: &'a dyn ModuleStorage,
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

impl FunctionValueExtension for FunctionValueExtensionAdapter<'_> {
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
        match &*LazyLoadedFunction::expect_this_impl(fun)?.state.borrow() {
            LazyLoadedFunctionState::Unresolved { data, .. } => Ok(data.clone()),
            LazyLoadedFunctionState::Resolved {
                fun,
                mask,
                ty_args,
                captured_layouts,
            } => {
                // If there are no captured layouts, then this closure is non-storable, i.e., the
                // function is not persistent (not public or not private with #[persistent]
                // attribute). This means that anonymous lambda-lifted functions are cannot be
                // serialized as well.
                let captured_layouts = captured_layouts.as_ref().cloned().ok_or_else(|| {
                    let msg = "Captured layouts must always be computed for storable closures";
                    PartialVMError::new(StatusCode::VALUE_SERIALIZATION_ERROR)
                        .with_message(msg.to_string())
                })?;

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

    fn max_value_nest_depth(&self) -> Option<u64> {
        let vm_config = self.module_storage.runtime_environment().vm_config();
        vm_config
            .enable_depth_checks
            .then_some(vm_config.max_value_nest_depth)
            .flatten()
    }
}
