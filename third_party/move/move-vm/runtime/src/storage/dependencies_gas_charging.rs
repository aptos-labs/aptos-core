// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{module_traversal::TraversalContext, CodeStorage, ModuleStorage};
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    errors::{Location, VMResult},
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress, gas_algebra::NumBytes, identifier::IdentStr,
    language_storage::ModuleId,
};
use move_vm_metrics::{Timer, VM_TIMER};
use move_vm_types::{gas::GasMeter, module_linker_error};
use std::{collections::BTreeMap, sync::Arc};
use typed_arena::Arena;

pub(crate) fn check_script_dependencies_and_check_gas(
    code_storage: &impl CodeStorage,
    gas_meter: &mut impl GasMeter,
    traversal_context: &mut TraversalContext,
    serialized_script: &[u8],
) -> VMResult<()> {
    let compiled_script = code_storage.deserialize_and_cache_script(serialized_script)?;
    let compiled_script = traversal_context.referenced_scripts.alloc(compiled_script);

    // TODO(Gas): Should we charge dependency gas for the script itself?
    check_dependencies_and_charge_gas(
        code_storage,
        gas_meter,
        &mut traversal_context.visited,
        traversal_context.referenced_modules,
        compiled_script.immediate_dependencies_iter(),
    )?;

    Ok(())
}

pub(crate) fn check_dependencies_and_charge_gas<'a, I>(
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
    let _timer = VM_TIMER.timer_with_label("check_dependencies_and_charge_gas");

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
            .map_err(|err| err.finish(Location::Module(ModuleId::new(*addr, name.to_owned()))))?;

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
