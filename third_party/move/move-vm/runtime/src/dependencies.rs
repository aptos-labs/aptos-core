// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::module_storage::ModuleStorage;
use move_binary_format::errors::{Location, VMResult};
use move_core_types::{
    account_address::AccountAddress, gas_algebra::NumBytes, identifier::IdentStr,
    language_storage::ModuleId,
};
use move_vm_types::gas::GasMeter;
use std::collections::BTreeMap;

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
///
/// TODO: Revisit the order of traversal. Consider switching to alphabetical order.
pub fn check_dependencies_and_charge_gas<'a, I>(
    module_storage: &'a impl ModuleStorage,
    gas_meter: &mut impl GasMeter,
    visited: &mut BTreeMap<(&'a AccountAddress, &'a IdentStr), ()>,
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

    for (addr, name) in ids.into_iter().rev() {
        // TODO: Allow the check of special addresses to be customized.
        if !addr.is_special() && visited.insert((addr, name), ()).is_none() {
            stack.push((addr, name));
        }
    }

    while let Some((addr, name)) = stack.pop() {
        let size = module_storage
            .fetch_module_size_in_bytes(addr, name)
            .map_err(|e| e.finish(Location::Undefined))?;
        gas_meter
            .charge_dependency(false, addr, name, NumBytes::new(size as u64))
            .map_err(|err| err.finish(Location::Module(ModuleId::new(*addr, name.to_owned()))))?;

        let immediate_dependencies = module_storage
            .fetch_module_immediate_dependencies(addr, name)
            .map_err(|e| e.finish(Location::Undefined))?;
        let immediate_friends = module_storage
            .fetch_module_immediate_friends(addr, name)
            .map_err(|e| e.finish(Location::Undefined))?;

        // Explore all dependencies and friends that have been visited yet.
        for (addr, name) in immediate_dependencies
            .into_iter()
            .chain(immediate_friends.into_iter())
            .rev()
        {
            // TODO: Allow the check of special addresses to be customized.
            if !addr.is_special() && visited.insert((addr, name), ()).is_none() {
                stack.push((addr, name));
            }
        }
    }

    Ok(())
}
