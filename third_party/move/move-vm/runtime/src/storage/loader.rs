// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::VMConfig, loader::LoadedFunctionOwner, module_traversal::TraversalContext,
    storage::module_storage::ModuleStorage, CodeStorage, LoadedFunction,
};
use move_binary_format::errors::{Location, PartialVMResult, VMResult};
use move_core_types::{gas_algebra::NumBytes, language_storage::TypeTag};
use move_vm_types::{
    gas::GasMeter,
    indices::ModuleIdx,
    loaded_data::runtime_types::{Type, TypeBuilder},
    module_linker_error,
};
use std::collections::BTreeMap;

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
        let deps = code_storage.deserialize_and_cache_script_dependencies(serialized_script)?;

        // TODO(Gas): Should we charge dependency gas for the script itself?
        self.check_dependencies_and_charge_gas(
            code_storage,
            gas_meter,
            &mut traversal_context.visited,
            deps.iter().copied(),
        )?;

        Ok(())
    }

    pub(crate) fn check_dependencies_and_charge_gas<I>(
        &self,
        module_storage: &dyn ModuleStorage,
        gas_meter: &mut impl GasMeter,
        visited: &mut BTreeMap<ModuleIdx, ()>,
        ids: I,
    ) -> VMResult<()>
    where
        I: IntoIterator<Item = ModuleIdx>,
        I::IntoIter: DoubleEndedIterator,
    {
        // Initialize the work list (stack) and the map of visited modules.
        //
        // TODO: Determine the reserved capacity based on the max number of dependencies allowed.
        let mut stack = Vec::with_capacity(512);
        push_next_ids_to_visit(&mut stack, visited, ids);

        while let Some(idx) = stack.pop() {
            let size = module_storage
                .fetch_module_size_in_bytes(&idx)?
                .ok_or_else(|| module_linker_error!(0, 0))?;
            gas_meter
                .charge_dependency(false, &idx, NumBytes::new(size as u64))
                .map_err(|err| err.finish(Location::Undefined))?;

            // Extend the lifetime of the module to the remainder of the function body
            // by storing it in an arena.
            //
            // This is needed because we need to store references derived from it in the
            // work list.
            let deps = module_storage.fetch_existing_module_dependencies(&idx)?;
            let friends = module_storage.fetch_existing_module_friends(&idx)?;

            // Explore all dependencies and friends that have been visited yet.
            let imm_deps_and_friends = deps.iter().chain(friends.iter()).copied();
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
            .map(|ty_tag| code_storage.fetch_ty(ty_tag))
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
pub(crate) fn push_next_ids_to_visit<I>(
    stack: &mut Vec<ModuleIdx>,
    visited: &mut BTreeMap<ModuleIdx, ()>,
    ids: I,
) where
    I: IntoIterator<Item = ModuleIdx>,
    I::IntoIter: DoubleEndedIterator,
{
    for idx in ids.into_iter().rev() {
        // TODO: Allow the check of special addresses to be customized.
        if !idx.is_special_addr() && visited.insert(idx, ()).is_none() {
            stack.push(idx);
        }
    }
}
