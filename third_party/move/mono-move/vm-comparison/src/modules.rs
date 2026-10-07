// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Replaces framework modules in the replayed state with the ones built from
//! this checkout.

use anyhow::{bail, Result};
use aptos_transaction_simulation::{InMemoryStateStore, SimulationStateStore};
use aptos_types::state_store::{state_key::StateKey, TStateView};
use move_binary_format::access::ModuleAccess;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

/// Modules overridden to their newest version from HEAD. This is the minimal
/// set of patches that preserves the historical behaviour as much as possible.
const OVERRIDDEN: &[&str] = &["dispatchable_fungible_asset", "features", "function_info"];

/// Overwrites [`OVERRIDDEN`] with this checkout's bytecode, together with any of
/// their dependencies the captured chain did not have. Both VMs read the
/// rewritten state, so a divergence after this is still a real one.
pub fn override_framework(state: &InMemoryStateStore) -> Result<()> {
    let bundle = aptos_cached_packages::head_release_bundle().code_and_compiled_modules();
    let bundle = bundle
        .into_iter()
        .map(|(bytes, module)| (module.self_id(), (bytes, module)))
        .collect::<HashMap<_, _>>();

    let mut remaining = OVERRIDDEN.iter().copied().collect::<BTreeSet<_>>();
    let roots = bundle
        .keys()
        .filter(|id| remaining.remove(id.name().as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !remaining.is_empty() {
        bail!(
            "Framework modules not found in the release bundle: {:?}",
            remaining
        );
    }

    // An overridden module can have dependencies the captured chain does not
    // have, so walk the closure: without them, neither VM can resolve it.
    let mut visited = roots.iter().cloned().collect::<HashSet<_>>();
    let mut queue = roots.iter().cloned().collect::<VecDeque<_>>();
    while let Some(id) = queue.pop_front() {
        let Some((bytes, module)) = bundle.get(&id) else {
            // Not from the framework, so the dump already carries it.
            continue;
        };

        let key = StateKey::module_id(&id);
        // Adding a module the captured chain lacks cannot change the semantics
        // of captured code, since nothing on that chain can reference it.
        if OVERRIDDEN.contains(&id.name().as_str()) || state.get_state_value(&key)?.is_none() {
            state.add_module_blob(&id, bytes.to_vec())?;
        }

        for dep in module.immediate_dependencies() {
            if visited.insert(dep.clone()) {
                queue.push_back(dep);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::state_store::state_value::StateValue;
    use move_core_types::{
        account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
    };

    fn framework_module(name: &str) -> ModuleId {
        ModuleId::new(AccountAddress::ONE, Identifier::new(name).unwrap())
    }

    #[test]
    fn test_override_publishes_dependency_closure() {
        let state = InMemoryStateStore::new();
        override_framework(&state).unwrap();

        for name in OVERRIDDEN.iter().copied().chain(["reflect"]) {
            let key = StateKey::module_id(&framework_module(name));
            assert!(
                state.get_state_value(&key).unwrap().is_some(),
                "{name} was not written"
            );
        }
    }

    #[test]
    fn test_captured_dependencies_are_left_alone() {
        let state = InMemoryStateStore::new();
        // `string` is a dependency of the overridden set, but not overridden.
        let key = StateKey::module_id(&framework_module("string"));
        let captured = StateValue::new_legacy(vec![1, 2, 3].into());
        state.set_state_value(key.clone(), captured.clone()).unwrap();

        override_framework(&state).unwrap();

        assert_eq!(state.get_state_value(&key).unwrap().unwrap(), captured);
    }
}
