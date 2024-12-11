// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::VMConfig,
    loader::{Function, LoadedFunctionOwner, Module},
    logging::expect_no_verification_errors,
    module_traversal::TraversalContext,
    storage::module_storage::ModuleStorage,
    CodeStorage, LoadedFunction,
};
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::NumBytes,
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    vm_status::StatusCode,
};
use move_vm_types::{
    gas::GasMeter,
    loaded_data::runtime_types::{StructType, Type, TypeBuilder},
    module_linker_error,
};
use std::{collections::BTreeMap, sync::Arc};
use typed_arena::Arena;

/// V2 implementation of loader, which is stateless - i.e., it does not contain module or script
/// cache. Instead, module and script storages are passed to all APIs by reference.
pub(crate) struct LoaderV2 {
    vm_config: VMConfig,
}

impl LoaderV2 {
    pub(crate) fn new(vm_config: VMConfig) -> Self {
        Self { vm_config }
    }

    pub(crate) fn vm_config(&self) -> &VMConfig {
        &self.vm_config
    }

    pub(crate) fn ty_builder(&self) -> &TypeBuilder {
        &self.vm_config.ty_builder
    }

    pub(crate) fn check_script_dependencies_and_check_gas(
        &self,
        code_storage: &impl CodeStorage,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        serialized_script: &[u8],
    ) -> VMResult<()> {
        let compiled_script = code_storage.deserialize_and_cache_script(serialized_script)?;
        let compiled_script = traversal_context.referenced_scripts.alloc(compiled_script);

        // TODO(Gas): Should we charge dependency gas for the script itself?
        self.check_dependencies_and_charge_gas(
            code_storage,
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
    ) -> VMResult<()>
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
            let size = module_storage
                .fetch_module_size_in_bytes(addr, name)?
                .ok_or_else(|| module_linker_error!(addr, name))?;
            gas_meter
                .charge_dependency(false, addr, name, NumBytes::new(size as u64))
                .map_err(|err| {
                    err.finish(Location::Module(ModuleId::new(*addr, name.to_owned())))
                })?;

            // Extend the lifetime of the module to the remainder of the function body
            // by storing it in an arena.
            //
            // This is needed because we need to store references derived from it in the
            // work list.
            let compiled_module = module_storage
                .fetch_deserialized_module(addr, name)?
                .ok_or_else(|| module_linker_error!(addr, name))?;
            let compiled_module = referenced_modules.alloc(compiled_module);

            // Explore all dependencies and friends that have been visited yet.
            let imm_deps_and_friends = compiled_module
                .immediate_dependencies_iter()
                .chain(compiled_module.immediate_friends_iter());
            push_next_ids_to_visit(&mut stack, visited, imm_deps_and_friends);
        }

        Ok(())
    }

    /// Loads the script:
    ///   1. Fetches it from the cache (or deserializes and verifies it if it is not cached).
    ///   2. Verifies type arguments (modules that define the type arguments are also loaded).
    /// If both steps are successful, returns a [LoadedFunction] corresponding to the script's
    /// entrypoint.
    pub(crate) fn load_script(
        &self,
        code_storage: &impl CodeStorage,
        serialized_script: &[u8],
        ty_tag_args: &[TypeTag],
    ) -> VMResult<LoadedFunction> {
        // Step 1: Load script. During the loading process, if script has not been previously
        // cached, it will be verified.
        let script = code_storage.verify_and_cache_script(serialized_script)?;

        // Step 2: Load & verify types used as type arguments passed to this script. Note that
        // arguments for scripts are verified on the client side.
        let ty_args = ty_tag_args
            .iter()
            .map(|ty_tag| self.load_ty(code_storage, ty_tag))
            .collect::<PartialVMResult<Vec<_>>>()
            .map_err(|err| err.finish(Location::Script))?;

        let main = script.entry_point();
        Type::verify_ty_arg_abilities(main.ty_param_abilities(), &ty_args)
            .map_err(|err| err.finish(Location::Script))?;

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
    ) -> VMResult<Arc<Module>> {
        module_storage
            .fetch_verified_module(address, module_name)
            .map_err(expect_no_verification_errors)?
            .ok_or_else(|| module_linker_error!(address, module_name))
    }

    /// Returns the function definition corresponding to the specified name, as well as the module
    /// where this function is defined.
    pub(crate) fn load_function_without_ty_args(
        &self,
        module_storage: &dyn ModuleStorage,
        address: &AccountAddress,
        module_name: &IdentStr,
        function_name: &IdentStr,
    ) -> VMResult<(Arc<Module>, Arc<Function>)> {
        let module = self.load_module(module_storage, address, module_name)?;
        let function = module
            .function_map
            .get(function_name)
            .and_then(|idx| module.function_defs.get(*idx))
            .ok_or_else(|| {
                PartialVMError::new(StatusCode::FUNCTION_RESOLUTION_FAILURE)
                    .with_message(format!(
                        "Function {}::{}::{} does not exist",
                        address, module_name, function_name
                    ))
                    .finish(Location::Undefined)
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
        let module = self
            .load_module(module_storage, address, module_name)
            .map_err(|err| err.to_partial())?;
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
        // TODO(loader_v2): Loader V1 uses VMResults everywhere, but partial VM errors
        //                  seem better fit. Here we map error to VMError to reuse existing
        //                  type builder implementation, and then strip the location info.
        self.ty_builder()
            .create_ty(ty_tag, |st| {
                self.load_struct_ty(
                    module_storage,
                    &st.address,
                    st.module.as_ident_str(),
                    st.name.as_ident_str(),
                )
                .map_err(|err| err.finish(Location::Undefined))
            })
            .map_err(|err| err.to_partial())
    }
}

impl Clone for LoaderV2 {
    fn clone(&self) -> Self {
        Self {
            vm_config: self.vm_config.clone(),
        }
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
