// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::environment::AptosEnvironment;
use aptos_types::{
    executable::ModulePath,
    state_store::{state_key::StateKey, StateView, TStateView},
    vm::modules::{ModuleStorageEntry, ModuleStorageEntryInterface},
};
use bytes::Bytes;
use move_binary_format::errors::{Location, PartialVMError, VMResult};
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, vm_status::StatusCode,
};
use move_vm_runtime::{Module, RuntimeEnvironment};
use move_vm_types::{module_cyclic_dependency_error, module_linker_error};
use once_cell::sync::Lazy;
use parking_lot::{Mutex, RwLock};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

/// Represents a unique identifier for an [AptosEnvironment] instance based on the features, gas
/// feature version, and other configs.
#[derive(Hash, Eq, PartialEq)]
struct EnvironmentID {
    bytes: Bytes,
}

impl EnvironmentID {
    /// Create a new identifier for the given environment.
    fn new(env: &AptosEnvironment) -> Self {
        // These are sufficient to distinguish different environments.
        let chain_id = env.chain_id();
        let features = env.features();
        // let timed_features = env.timed_features();
        let gas_feature_version = env.gas_feature_version();
        let vm_config = env.vm_config();

        let bytes = bcs::to_bytes(&(
            chain_id,
            features,
            // timed_features,
            gas_feature_version,
            vm_config,
        ))
        .expect("Should be able to serialize all configs")
        .into();
        Self { bytes }
    }
}

/// A cache of different environment that can be persisted across blocks. Used by block executor
/// only.
pub struct EnvironmentCache(Mutex<Option<(EnvironmentID, AptosEnvironment)>>);

impl EnvironmentCache {
    /// Returns the cached environment if it exists and has the same configuration as if it was
    /// created based on the current state, or creates a new one and caches it. Should only be
    /// called at the block boundaries.
    pub fn fetch_with_delayed_field_optimization_enabled(
        state_view: &impl StateView,
    ) -> AptosEnvironment {
        let env = AptosEnvironment::new_with_delayed_field_optimization_enabled(state_view);
        let id = EnvironmentID::new(&env);
        ENVIRONMENT_CACHE.get_or_fetch(id, env)
    }

    /// Returns new environment cache.
    fn empty() -> Self {
        Self(Mutex::new(None))
    }

    /// Returns the newly created environment, or the cached one.
    fn get_or_fetch(&self, id: EnvironmentID, env: AptosEnvironment) -> AptosEnvironment {
        let mut cache = self.0.lock();
        if let Some((cached_id, cached_env)) = cache.as_ref() {
            if &id == cached_id {
                return cached_env.clone();
            }
            // } else {
            //     println!("Change of env!!!");
            //     println!("Before: {:?}", cached_env);
            //     println!("After: {:?}", env);
            // }
        }

        let flush_cross_block_module_cache = cache.is_some();
        *cache = Some((id, env.clone()));
        drop(cache);
        if flush_cross_block_module_cache {
            // println!(" !!! FLUSH: flush_cross_block_module_cache in get_or_fetch");
            MODULE_CACHE.0.write().flush()
        }
        env
    }
}

/// Long-living environment cache to be used across blocks.
static ENVIRONMENT_CACHE: Lazy<EnvironmentCache> = Lazy::new(EnvironmentCache::empty);

struct ModuleCache {
    invalidated: bool,
    modules: HashMap<StateKey, Arc<ModuleStorageEntry>>,
}

impl ModuleCache {
    fn empty() -> Self {
        Self {
            invalidated: false,
            modules: HashMap::new(),
        }
    }

    fn flush(&mut self) {
        println!(" !!! FLUSH: flush");
        self.invalidated = false;
        self.modules.clear();
    }

    fn flush_if_invalidated_and_mark_valid(&mut self) {
        if self.invalidated {
            println!(" !!! FLUSH: flush_if_invalidated_and_mark_valid");
            self.invalidated = false;
            self.modules.clear();
        }
    }

    pub fn traverse<K: ModulePath>(
        &mut self,
        entry: Arc<ModuleStorageEntry>,
        address: &AccountAddress,
        module_name: &IdentStr,
        base_view: &impl TStateView<Key = K>,
        visited: &mut HashSet<StateKey>,
        runtime_environment: &RuntimeEnvironment,
    ) -> VMResult<Arc<Module>> {
        let cm = entry.as_compiled_module();
        runtime_environment.paranoid_check_module_address_and_name(
            cm.as_ref(),
            address,
            module_name,
        )?;

        let size = entry.size_in_bytes();
        let hash = entry.hash();
        let locally_verified_module =
            runtime_environment.build_locally_verified_module(cm, size, hash)?;

        let mut verified_dependencies = vec![];
        for (addr, name) in locally_verified_module.immediate_dependencies_iter() {
            let dep_key = StateKey::from_address_and_module_name(addr, name);
            let dep_entry = match self.modules.get(&dep_key) {
                Some(dep_entry) => dep_entry.clone(),
                None => {
                    let k = K::from_address_and_module_name(addr, name);
                    let sv = base_view
                        .get_state_value(&k)
                        .map_err(|_| {
                            PartialVMError::new(StatusCode::STORAGE_ERROR)
                                .finish(Location::Undefined)
                        })?
                        .ok_or_else(|| module_linker_error!(addr, name))?;
                    ModuleStorageEntry::from_state_value(runtime_environment, sv).map(Arc::new)?
                },
            };
            if let Some(module) = dep_entry.try_as_verified_module() {
                verified_dependencies.push(module);
                continue;
            }
            assert!(!dep_entry.is_verified());

            if visited.insert(dep_key.clone()) {
                let module = self.traverse(
                    dep_entry,
                    addr,
                    name,
                    base_view,
                    visited,
                    runtime_environment,
                )?;
                verified_dependencies.push(module);
            } else {
                return Err(module_cyclic_dependency_error!(address, module_name));
            }
        }

        // At this point, all dependencies of the module are verified, so we can run final checks
        // and construct a verified module.
        let module = Arc::new(
            runtime_environment
                .build_verified_module(locally_verified_module, &verified_dependencies)?,
        );
        let verified_entry = Arc::new(entry.make_verified(module.clone()));

        // println!(
        //     "Cached {}::{} in CrossBlockModuleCache",
        //     address, module_name
        // );
        self.modules.insert(
            StateKey::from_address_and_module_name(address, module_name),
            verified_entry,
        );
        Ok(module)
    }
}

pub struct CrossBlockModuleCache(RwLock<ModuleCache>);

impl CrossBlockModuleCache {
    pub fn get_from_cross_block_module_cache(
        state_key: &StateKey,
    ) -> Option<Arc<ModuleStorageEntry>> {
        MODULE_CACHE.get_module_storage_entry(state_key)
    }

    pub fn store_to_cross_block_module_cache(state_key: StateKey, entry: Arc<ModuleStorageEntry>) {
        MODULE_CACHE.store_module_storage_entry(state_key, entry)
    }

    pub fn traverse<K: ModulePath>(
        entry: Arc<ModuleStorageEntry>,
        address: &AccountAddress,
        module_name: &IdentStr,
        base_view: &impl TStateView<Key = K>,
        runtime_environment: &RuntimeEnvironment,
    ) -> VMResult<Arc<Module>> {
        let mut cache = MODULE_CACHE.0.write();
        let mut visited = HashSet::new();
        cache.traverse(
            entry,
            address,
            module_name,
            base_view,
            &mut visited,
            runtime_environment,
        )
    }

    pub fn is_invalidated() -> bool {
        let cache = MODULE_CACHE.0.read();
        cache.invalidated
    }

    pub fn mark_invalid() {
        // println!(" !!! FLUSH: mark_invalid");
        let mut cache = MODULE_CACHE.0.write();
        cache.invalidated = true;
    }

    pub fn flush_cross_block_module_cache_if_invalidated() {
        MODULE_CACHE.0.write().flush_if_invalidated_and_mark_valid()
    }

    /// Returns new module cache.
    fn empty() -> Self {
        Self(RwLock::new(ModuleCache::empty()))
    }

    fn get_module_storage_entry(&self, state_key: &StateKey) -> Option<Arc<ModuleStorageEntry>> {
        self.0.read().modules.get(state_key).cloned()
    }

    fn store_module_storage_entry(&self, state_key: StateKey, entry: Arc<ModuleStorageEntry>) {
        let mut modules = self.0.write();
        modules.modules.insert(state_key, entry);
    }
}

static MODULE_CACHE: Lazy<CrossBlockModuleCache> = Lazy::new(CrossBlockModuleCache::empty);
