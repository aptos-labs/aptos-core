// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Modules a replay needs: the module dependency closure of a read set, and the framework this
//! binary was built with.

use anyhow::{anyhow, Context, Result};
use aptos_types::state_store::{
    state_key::{inner::StateKeyInner, StateKey},
    state_value::StateValue,
};
use move_binary_format::{access::ModuleAccess, CompiledModule};
use move_core_types::{account_address::AccountAddress, language_storage::ModuleId};
use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    hash::BuildHasher,
};

/// The module `key` holds, if it is a module key.
pub fn module_id_of(key: &StateKey) -> Option<ModuleId> {
    match key.inner() {
        StateKeyInner::AccessPath(ap) => ap.try_get_module_id(),
        StateKeyInner::TableItem { .. }
        | StateKeyInner::Raw(_)
        | StateKeyInner::TradingNative(_) => None,
    }
}

/// The account `key` holds a module at, if it is a module key: cheaper than [`module_id_of`],
/// which decodes the module's name too.
pub fn module_address_of(key: &StateKey) -> Option<AccountAddress> {
    match key.inner() {
        StateKeyInner::AccessPath(ap) if ap.is_code() => Some(ap.address),
        StateKeyInner::AccessPath(_)
        | StateKeyInner::TableItem { .. }
        | StateKeyInner::Raw(_)
        | StateKeyInner::TradingNative(_) => None,
    }
}

/// A map of state values, as the replay tools keep a read set.
pub trait StateMap {
    fn value(&self, key: &StateKey) -> Option<&StateValue>;

    fn add(&mut self, key: StateKey, value: StateValue);

    fn state_keys(&self) -> impl Iterator<Item = &StateKey>;
}

impl<S: BuildHasher> StateMap for HashMap<StateKey, StateValue, S> {
    fn value(&self, key: &StateKey) -> Option<&StateValue> {
        self.get(key)
    }

    fn add(&mut self, key: StateKey, value: StateValue) {
        self.insert(key, value);
    }

    fn state_keys(&self) -> impl Iterator<Item = &StateKey> {
        self.keys()
    }
}

impl StateMap for BTreeMap<StateKey, StateValue> {
    fn value(&self, key: &StateKey) -> Option<&StateValue> {
        self.get(key)
    }

    fn add(&mut self, key: StateKey, value: StateValue) {
        self.insert(key, value);
    }

    fn state_keys(&self) -> impl Iterator<Item = &StateKey> {
        self.keys()
    }
}

/// What closing a module dependency graph did.
#[derive(Debug, Default)]
pub struct ModuleClosure {
    /// Modules added with `fetch`.
    pub fetched: usize,
    /// Dependencies `fetch` did not find.
    pub missing: Vec<ModuleId>,
}

/// Walks the module dependency graph of every module already in `state`, adding each missing
/// module that `fetch` finds, until the closure is complete. Modules for which `is_provided`
/// holds are supplied elsewhere with a complete closure, so they are neither walked nor fetched.
pub fn close_module_graph(
    state: &mut impl StateMap,
    is_provided: impl Fn(&ModuleId) -> bool,
    mut fetch: impl FnMut(&StateKey) -> Result<Option<StateValue>>,
) -> Result<ModuleClosure> {
    let mut closure = ModuleClosure::default();
    let mut visited: HashSet<ModuleId> = HashSet::new();
    let mut queue: VecDeque<ModuleId> = VecDeque::new();
    for key in state.state_keys() {
        if let Some(module_id) = module_id_of(key)
            && !is_provided(&module_id)
            && visited.insert(module_id.clone())
        {
            queue.push_back(module_id);
        }
    }

    while let Some(module_id) = queue.pop_front() {
        let deserialize = |value: &StateValue| {
            CompiledModule::deserialize(value.bytes())
                .map_err(|e| anyhow!("failed to deserialize module {}: {:?}", module_id, e))
        };
        let key = StateKey::module_id(&module_id);
        let module = match state.value(&key) {
            Some(value) => deserialize(value)?,
            None => {
                let Some(value) =
                    fetch(&key).with_context(|| format!("failed to fetch module {}", module_id))?
                else {
                    closure.missing.push(module_id);
                    continue;
                };
                let module = deserialize(&value)?;
                state.add(key, value);
                closure.fetched += 1;
                module
            },
        };
        for dep in module.immediate_dependencies() {
            if !is_provided(&dep) && visited.insert(dep.clone()) {
                queue.push_back(dep);
            }
        }
    }
    Ok(closure)
}

/// The modules of the framework this binary was built with.
pub fn head_framework() -> BTreeMap<StateKey, StateValue> {
    aptos_cached_packages::head_release_bundle()
        .code_and_compiled_modules()
        .into_iter()
        .map(|(code, module)| {
            (
                StateKey::module(module.self_addr(), module.self_name()),
                StateValue::new_legacy(code.to_vec().into()),
            )
        })
        .collect()
}

/// The keys of [`head_framework`], built once.
pub fn head_framework_keys() -> &'static [StateKey] {
    static KEYS: std::sync::OnceLock<Vec<StateKey>> = std::sync::OnceLock::new();
    KEYS.get_or_init(|| {
        aptos_cached_packages::head_release_bundle()
            .code_and_compiled_modules()
            .into_iter()
            .map(|(_, module)| StateKey::module(module.self_addr(), module.self_name()))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::state_store::TStateView;
    use mono_move_testsuite::compile_move_source;

    #[test]
    fn closes_module_dependency_graph() {
        // `a` depends on `b`; `b` has no dependencies.
        let modules = compile_move_source(
            r#"
            module 0xc0ffee::b { public fun f(): u64 { 1 } }
            module 0xc0ffee::a { use 0xc0ffee::b; public fun g(): u64 { b::f() } }
            "#,
        )
        .expect("compile");

        let bytes = |name: &str| {
            let m = modules
                .iter()
                .find(|m| m.self_id().name().as_str() == name)
                .expect("the compiled module");
            let mut v = vec![];
            m.serialize(&mut v).expect("a compiled module serializes");
            (m.self_id(), StateValue::new_legacy(v.into()))
        };
        let (a_id, a_val) = bytes("a");
        let (b_id, b_val) = bytes("b");

        // "Chain" has both modules; the read-set initially has only `a`.
        let chain = aptos_transaction_simulation::InMemoryStateStore::new_with_state_values([
            (StateKey::module_id(&a_id), a_val.clone()),
            (StateKey::module_id(&b_id), b_val),
        ]);
        let fetch = |key: &StateKey| chain.get_state_value(key).map_err(|e| anyhow!("{:?}", e));

        // As both tools keep their read sets.
        let mut read_set = BTreeMap::from([(StateKey::module_id(&a_id), a_val.clone())]);
        let closure = close_module_graph(&mut read_set, |_| false, fetch).expect("close");
        assert_eq!(closure.fetched, 1);
        assert!(closure.missing.is_empty());
        assert!(
            read_set.contains_key(&StateKey::module_id(&b_id)),
            "closing the graph should pull in the missing dependency `b`"
        );

        let mut read_set = HashMap::from([(StateKey::module_id(&a_id), a_val)]);
        let closure = close_module_graph(&mut read_set, |_| false, fetch).expect("close");
        assert_eq!(closure.fetched, 1);
        assert!(read_set.contains_key(&StateKey::module_id(&b_id)));
    }
}
