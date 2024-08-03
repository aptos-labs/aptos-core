// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::VMConfig,
    loader::{Function, LoadedFunctionOwner, Module, Script, TypeCache},
    module_traversal::TraversalContext,
    native_functions::NativeFunctions,
    storage::{
        module_storage::ModuleStorage, script_storage::ScriptStorage,
        struct_name_index_map::StructNameIndexMap, struct_type_storage::LoaderV2StructTypeStorage,
        verifier::Verifier,
    },
    unexpected_unimplemented_error, LoadedFunction,
};
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    errors::{Location, PartialVMError, PartialVMResult},
    file_format::CompiledScript,
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress, gas_algebra::NumBytes, identifier::IdentStr,
    language_storage::TypeTag, vm_status::StatusCode,
};
use move_vm_types::{
    gas::GasMeter,
    loaded_data::runtime_types::{StructType, Type, TypeBuilder},
};
use parking_lot::RwLock;
use std::{collections::BTreeMap, sync::Arc};
use typed_arena::Arena;

/// V2 implementation of loader, which is stateless - i.e., it does not contain
/// module or script cache. Instead, module and script storages are passed to all
/// APIs by reference.
pub(crate) struct LoaderV2<V: Clone + Verifier> {
    // Map to from struct names to indices, to save on unnecessary cloning and
    // reduce memory consumption.
    struct_name_index_map: StructNameIndexMap,
    // Configuration of the VM, which own this loader. Contains information about
    // enabled checks, etc.
    vm_config: VMConfig,
    // Verifier instance which runs passes when scripts or modules are loaded for
    // the first time.
    verifier: V,

    // All registered native functions the loader is aware of. When loaded modules
    // are constructed, existing native functions are inlined in the loaded module
    // representation, so that the interpreter can call them directly.
    natives: NativeFunctions,

    // Local caches:
    //   These caches are owned by this loader and are not affected by module
    //   upgrades. When a new cache is added, the safety guarantees (i.e., why
    //   it is safe for the loader to own this cache) MUST be documented.
    // TODO(George): Revisit type cache implementation. For now re-use the existing
    //               one to unblock upgradable module and script storage first.
    ty_cache: RwLock<TypeCache>,
}

impl<V: Clone + Verifier> LoaderV2<V> {
    pub(crate) fn vm_config(&self) -> &VMConfig {
        &self.vm_config
    }

    pub(crate) fn ty_builder(&self) -> &TypeBuilder {
        &self.vm_config.ty_builder
    }

    pub(crate) fn ty_cache(&self) -> &RwLock<TypeCache> {
        &self.ty_cache
    }

    pub(crate) fn struct_name_index_map(&self) -> &StructNameIndexMap {
        &self.struct_name_index_map
    }

    pub(crate) fn check_script_dependencies_and_check_gas(
        &self,
        module_storage: &impl ModuleStorage,
        script_storage: &impl ScriptStorage,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        serialized_script: &[u8],
    ) -> PartialVMResult<()> {
        let compiled_script = script_storage.fetch_deserialized_script(serialized_script)?;
        let compiled_script = traversal_context.referenced_scripts.alloc(compiled_script);

        // TODO(Gas): Should we charge dependency gas for the script itself?
        self.check_dependencies_and_charge_gas(
            module_storage,
            gas_meter,
            &mut traversal_context.visited,
            traversal_context.referenced_modules,
            compiled_script.immediate_dependencies_iter(),
        )?;

        Ok(())
    }

    pub(crate) fn check_dependencies_and_charge_gas<'a, I>(
        &self,
        module_storage: &dyn ModuleStorage,
        gas_meter: &mut impl GasMeter,
        visited: &mut BTreeMap<(&'a AccountAddress, &'a IdentStr), ()>,
        referenced_modules: &'a Arena<Arc<CompiledModule>>,
        ids: I,
    ) -> PartialVMResult<()>
    where
        I: IntoIterator<Item = (&'a AccountAddress, &'a IdentStr)>,
        I::IntoIter: DoubleEndedIterator,
    {
        // Initialize the work list (stack) and the map of visited modules.
        //
        // TODO: Determine the reserved capacity based on the max number of dependencies allowed.
        let mut stack = Vec::with_capacity(512);
        push_next_ids_to_visit(&mut stack, visited, ids);

        while let Some((addr, name)) = stack.pop() {
            let size = module_storage.fetch_module_size_in_bytes(addr, name)?;
            gas_meter.charge_dependency(false, addr, name, NumBytes::new(size as u64))?;

            // Extend the lifetime of the module to the remainder of the function body
            // by storing it in an arena.
            //
            // This is needed because we need to store references derived from it in the
            // work list.
            let compiled_module = module_storage.fetch_deserialized_module(addr, name)?;
            let compiled_module = referenced_modules.alloc(compiled_module);

            // Explore all dependencies and friends that have been visited yet.
            let imm_deps_and_friends = compiled_module
                .immediate_dependencies_iter()
                .chain(compiled_module.immediate_friends_iter());
            push_next_ids_to_visit(&mut stack, visited, imm_deps_and_friends);
        }

        Ok(())
    }

    pub(crate) fn load_script(
        &self,
        module_storage: &impl ModuleStorage,
        script_storage: &impl ScriptStorage,
        serialized_script: &[u8],
        ty_args: &[TypeTag],
    ) -> PartialVMResult<LoadedFunction> {
        // Step 1: Load script. During the loading process, if script has not been previously
        // cached, it will be verified.
        let script = script_storage.fetch_or_create_verified_script(serialized_script, &|cs| {
            self.build_script(module_storage, cs)
        })?;

        // Step 2: Load & verify types used as type arguments passed to this script. Note that
        // arguments for scripts are verified on the client side.
        let ty_args = ty_args
            .iter()
            .map(|ty| self.load_ty(module_storage, ty))
            .collect::<PartialVMResult<Vec<_>>>()?;

        let main = script.entry_point();
        Type::verify_ty_arg_abilities(main.ty_param_abilities(), &ty_args)?;

        Ok(LoadedFunction {
            owner: LoadedFunctionOwner::Script(script),
            ty_args,
            function: main,
        })
    }

    /// Returns a loaded & verified module corresponding to the specified name.
    pub(crate) fn load_module(
        &self,
        module_storage: &dyn ModuleStorage,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Arc<Module>> {
        module_storage.fetch_or_create_verified_module(address, module_name, &|cm| {
            self.build_module(module_storage, cm)
        })
    }

    /// Returns a function definition corresponding to the specified name. The module
    /// containing the function is loaded.
    pub(crate) fn load_function_without_ty_args(
        &self,
        module_storage: &dyn ModuleStorage,
        address: &AccountAddress,
        module_name: &IdentStr,
        function_name: &IdentStr,
    ) -> PartialVMResult<(Arc<Module>, Arc<Function>)> {
        let module = self.load_module(module_storage, address, module_name)?;
        let function = module
            .function_map
            .get(function_name)
            .and_then(|idx| module.function_defs.get(*idx))
            .ok_or_else(|| {
                PartialVMError::new(StatusCode::FUNCTION_RESOLUTION_FAILURE).with_message(format!(
                    "Function {}::{}::{} does not exist",
                    address, module_name, function_name
                ))
            })?
            .clone();
        Ok((module, function))
    }

    /// Returns a struct type corresponding to the specified name. The module
    /// containing the struct is loaded.
    pub(crate) fn load_struct_ty(
        &self,
        module_storage: &dyn ModuleStorage,
        address: &AccountAddress,
        module_name: &IdentStr,
        struct_name: &IdentStr,
    ) -> PartialVMResult<Arc<StructType>> {
        let module = self.load_module(module_storage, address, module_name)?;
        Ok(module
            .struct_map
            .get(struct_name)
            .and_then(|idx| module.structs.get(*idx))
            .ok_or_else(|| {
                PartialVMError::new(StatusCode::TYPE_RESOLUTION_FAILURE).with_message(format!(
                    "Struct {}::{}::{} does not exist",
                    address, module_name, struct_name
                ))
            })?
            .definition_struct_type
            .clone())
    }

    /// Returns a runtime type corresponding to the specified type tag (file format type
    /// representation). In case struct types are transitively loaded, the module containing
    /// the struct definition is also loaded.
    pub(crate) fn load_ty(
        &self,
        module_storage: &impl ModuleStorage,
        ty_tag: &TypeTag,
    ) -> PartialVMResult<Type> {
        // TODO(George): Loader V1 uses VMResults everywhere, but partial VM errors
        //               seem better fit. Here we map error to VMError to reuse existing
        //               type builder implementation, and then strip the location info.
        self.ty_builder()
            .create_ty(ty_tag, |st| {
                self.load_struct_ty(
                    module_storage,
                    &st.address,
                    st.module.as_ident_str(),
                    st.name.as_ident_str(),
                )
                .map_err(|e| e.finish(Location::Undefined))
            })
            .map_err(|e| e.to_partial())
    }

    pub(crate) fn verify_modules_for_publication(
        &self,
        _module_storage: &impl ModuleStorage,
        _published_modules: &[CompiledModule],
    ) -> PartialVMResult<()> {
        unexpected_unimplemented_error!()
    }
}

impl<V: Clone + Verifier> Clone for LoaderV2<V> {
    fn clone(&self) -> Self {
        Self {
            struct_name_index_map: self.struct_name_index_map().clone(),
            vm_config: self.vm_config().clone(),
            verifier: self.verifier.clone(),
            natives: self.natives.clone(),
            ty_cache: RwLock::new(self.ty_cache().read().clone()),
        }
    }
}

// Loader is the only structure that can create runtime representations of modules and
// scripts. The following builder methods can be used to create these, or passed as
// callbacks externally. These functions should remain private at all times.
impl<V: Clone + Verifier> LoaderV2<V> {
    /// Given loader's context, builds a new verified script instance.
    fn build_script(
        &self,
        module_storage: &dyn ModuleStorage,
        compiled_script: Arc<CompiledScript>,
    ) -> PartialVMResult<Script> {
        // Verify local properties of the script.
        self.verifier.verify_script(compiled_script.as_ref())?;

        // Fetch all dependencies of this script, and verify them as well.
        let imm_dependencies = compiled_script
            .immediate_dependencies_iter()
            .map(|(addr, name)| {
                module_storage.fetch_or_create_verified_module(addr, name, &|cm| {
                    self.build_module(module_storage, cm)
                })
            })
            .collect::<PartialVMResult<Vec<_>>>()?;

        // Perform checks on script and its dependencies.
        self.verifier.verify_script_with_dependencies(
            compiled_script.as_ref(),
            imm_dependencies.iter().map(|m| m.as_ref()),
        )?;

        let struct_ty_storage = LoaderV2StructTypeStorage {
            loader: self,
            module_storage,
        };
        Script::new(
            compiled_script,
            &struct_ty_storage,
            self.struct_name_index_map(),
        )
    }

    /// Given loader's context, builds a new verified module instance.
    fn build_module(
        &self,
        module_storage: &dyn ModuleStorage,
        compiled_module: Arc<CompiledModule>,
    ) -> PartialVMResult<Module> {
        // Verify local properties of the module.
        self.verifier.verify_module(compiled_module.as_ref())?;

        // Fetch all dependencies of this module, ensuring they are verified as well.
        let f = |cm| self.build_module(module_storage, cm);
        let imm_dependencies = compiled_module
            .immediate_dependencies_iter()
            .map(|(addr, name)| module_storage.fetch_or_create_verified_module(addr, name, &f))
            .collect::<PartialVMResult<Vec<_>>>()?;

        // Perform checks on the module with its immediate dependencies.
        self.verifier.verify_module_with_dependencies(
            compiled_module.as_ref(),
            imm_dependencies.iter().map(|m| m.as_ref()),
        )?;

        let struct_ty_storage = LoaderV2StructTypeStorage {
            loader: self,
            module_storage,
        };

        // TODO(George): While we do not need size anymore, fetch the correct value just in case.
        //               Once V1 API is no longer used, we can remove this.
        let size = module_storage
            .fetch_module_size_in_bytes(compiled_module.self_addr(), compiled_module.self_name())?;
        Module::new(
            &self.natives,
            size,
            compiled_module,
            &struct_ty_storage,
            self.struct_name_index_map(),
        )
    }
}

/// Given a list of addresses and module names, pushes them onto stack unless they have been
/// already visited or if the address is special.
#[inline]
pub(crate) fn push_next_ids_to_visit<'a, I>(
    stack: &mut Vec<(&'a AccountAddress, &'a IdentStr)>,
    visited: &mut BTreeMap<(&'a AccountAddress, &'a IdentStr), ()>,
    ids: I,
) where
    I: IntoIterator<Item = (&'a AccountAddress, &'a IdentStr)>,
    I::IntoIter: DoubleEndedIterator,
{
    for (addr, name) in ids.into_iter().rev() {
        // TODO: Allow the check of special addresses to be customized.
        if !addr.is_special() && visited.insert((addr, name), ()).is_none() {
            stack.push((addr, name));
        }
    }
}
