// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{module_traversal::TraversalContext, CodeStorage, ModuleStorage};
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    errors::{Location, VMResult},
};
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::NumBytes,
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
};
use move_vm_metrics::{Timer, VM_TIMER};
use move_vm_types::{gas::GasMeter, module_linker_error};
use std::collections::BTreeSet;

pub fn check_script_dependencies_and_check_gas(
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
        traversal_context,
        compiled_script.immediate_dependencies_iter(),
    )?;

    Ok(())
}

pub fn check_type_tag_dependencies_and_charge_gas(
    module_storage: &impl ModuleStorage,
    gas_meter: &mut impl GasMeter,
    traversal_context: &mut TraversalContext,
    ty_tags: &[TypeTag],
) -> VMResult<()> {
    // Charge gas based on the distinct ordered module ids.
    let timer = VM_TIMER.timer_with_label("traverse_ty_tags_for_gas_charging");
    let ordered_ty_tags = ty_tags
        .iter()
        .flat_map(|ty_tag| ty_tag.preorder_traversal_iter())
        .filter_map(TypeTag::struct_tag)
        .map(|struct_tag| {
            let module_id = traversal_context
                .referenced_module_ids
                .alloc(struct_tag.module_id());
            (module_id.address(), module_id.name())
        })
        .collect::<BTreeSet<_>>();
    drop(timer);

    check_dependencies_and_charge_gas(
        module_storage,
        gas_meter,
        traversal_context,
        ordered_ty_tags,
    )
}

/// Traverses the whole transitive closure of dependencies, starting from the specified
/// modules and performs gas metering.
///
/// The traversal follows a depth-first order, with the module itself being visited first,
/// followed by its dependencies, and finally its friends.
/// DO NOT CHANGE THE ORDER unless you have a good reason, or otherwise this could introduce
/// a breaking change to the gas semantics.
///
/// This will result in the shallow-loading of the modules -- they will be read from the
/// storage as bytes and then deserialized, but NOT converted into the runtime representation.
///
/// It should also be noted that this is implemented in a way that avoids the cloning of
/// `ModuleId`, a.k.a. heap allocations, as much as possible, which is critical for
/// performance.
pub fn check_dependencies_and_charge_gas<'a, I>(
    module_storage: &impl ModuleStorage,
    gas_meter: &mut impl GasMeter,
    traversal_context: &mut TraversalContext<'a>,
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
    traversal_context.push_next_ids_to_visit(&mut stack, ids);

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
        let compiled_module = traversal_context.referenced_modules.alloc(compiled_module);

        // Explore all dependencies and friends that have been visited yet.
        let imm_deps_and_friends = compiled_module
            .immediate_dependencies_iter()
            .chain(compiled_module.immediate_friends_iter());
        traversal_context.push_next_ids_to_visit(&mut stack, imm_deps_and_friends);
    }

    Ok(())
}
