// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{CodeStorage, ModuleStorage};
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    errors::{Location, VMResult},
};
use move_core_types::{
    gas_algebra::NumBytes,
    language_storage::{ModuleId, TypeTag},
};
use move_vm_metrics::{Timer, VM_TIMER};
use move_vm_types::{
    gas::{DependencyGasMeter, GasMeter},
    module_linker_error,
};
use std::collections::BTreeSet;

pub fn check_script_dependencies_and_check_gas(
    code_storage: &impl CodeStorage,
    gas_meter: &mut impl GasMeter,
    serialized_script: &[u8],
) -> VMResult<()> {
    let compiled_script = code_storage.deserialize_and_cache_script(serialized_script)?;

    // TODO(Gas): Should we charge dependency gas for the script itself?
    check_dependencies_and_charge_gas(
        code_storage,
        gas_meter,
        compiled_script.immediate_dependencies_iter(),
    )?;

    Ok(())
}

pub fn check_type_tag_dependencies_and_charge_gas(
    module_storage: &impl ModuleStorage,
    gas_meter: &mut impl GasMeter,
    ty_tags: &[TypeTag],
) -> VMResult<()> {
    // Charge gas based on the distinct ordered module ids.
    let timer = VM_TIMER.timer_with_label("traverse_ty_tags_for_gas_charging");
    let ordered_ty_tags = ty_tags
        .iter()
        .flat_map(|ty_tag| ty_tag.preorder_traversal_iter())
        .filter_map(TypeTag::struct_tag)
        .map(|struct_tag| struct_tag.module_id())
        .collect::<BTreeSet<_>>();
    drop(timer);

    check_dependencies_and_charge_gas(module_storage, gas_meter, ordered_ty_tags)
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
pub fn check_dependencies_and_charge_gas<I>(
    module_storage: &dyn ModuleStorage,
    gas_meter: &mut dyn DependencyGasMeter,
    ids: I,
) -> VMResult<()>
where
    I: IntoIterator<Item = ModuleId>,
    I::IntoIter: DoubleEndedIterator,
{
    let _timer = VM_TIMER.timer_with_label("check_dependencies_and_charge_gas");

    // Initialize the work list (stack) and the map of visited modules.
    //
    // TODO: Determine the reserved capacity based on the max number of dependencies allowed.
    let mut stack = Vec::with_capacity(512);
    push_next_ids_to_visit(module_storage, gas_meter, &mut stack, ids)?;

    while let Some(id) = stack.pop() {
        let compiled_module = module_storage
            .fetch_deserialized_module(&id)?
            .ok_or_else(|| module_linker_error!(id.address(), id.name()))?;

        // Explore all dependencies and friends that have been visited yet.
        let imm_deps_and_friends = compiled_module
            .immediate_dependencies_iter()
            .chain(compiled_module.immediate_friends_iter());
        push_next_ids_to_visit(module_storage, gas_meter, &mut stack, imm_deps_and_friends)?;
    }

    Ok(())
}

fn push_next_ids_to_visit<I>(
    module_storage: &dyn ModuleStorage,
    gas_meter: &mut dyn DependencyGasMeter,
    stack: &mut Vec<ModuleId>,
    ids: I,
) -> VMResult<()>
where
    I: IntoIterator<Item = ModuleId>,
    I::IntoIter: DoubleEndedIterator,
{
    for module_id in ids.into_iter().rev() {
        if !module_id.address().is_special()
            && !gas_meter.is_existing_dependency_metered(&module_id)
        {
            let size = module_storage
                .fetch_module_size_in_bytes(&module_id)?
                .ok_or_else(|| module_linker_error!(module_id.address(), module_id.name()))?;
            gas_meter
                .charge_existing_dependency(&module_id, NumBytes::new(size as u64))
                .map_err(|err| err.finish(Location::Module(module_id.clone())))?;

            stack.push(module_id)
        }
    }
    Ok(())
}
