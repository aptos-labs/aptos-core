// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::environment::AptosEnvironment;
use aptos_types::{
    state_store::{state_key::StateKey, StateView},
    vm::modules::{ModuleStorageEntry, ModuleStorageEntryInterface},
};
use bytes::Bytes;
use once_cell::sync::Lazy;
use parking_lot::{Mutex, RwLock};
use std::{collections::HashMap, sync::Arc};

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
        let timed_features = env.timed_features();
        let gas_feature_version = env.gas_feature_version();
        let vm_config = env.vm_config();

        let bytes = bcs::to_bytes(&(
            chain_id,
            features,
            timed_features,
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
        }

        let flush_cross_block_module_cache = cache.is_some();
        *cache = Some((id, env.clone()));
        drop(cache);
        if flush_cross_block_module_cache {
            CrossBlockModuleCache::flush_cross_block_module_cache();
        }
        env
    }
}

/// Long-living environment cache to be used across blocks.
static ENVIRONMENT_CACHE: Lazy<EnvironmentCache> = Lazy::new(EnvironmentCache::empty);

pub struct CrossBlockModuleCache {
    // TODO: some eviction policy?
    modules: RwLock<HashMap<StateKey, Arc<ModuleStorageEntry>>>,
}

impl CrossBlockModuleCache {
    pub fn get_from_cross_block_module_cache(
        state_key: &StateKey,
    ) -> Option<Arc<ModuleStorageEntry>> {
        MODULE_CACHE.get_module_storage_entry(state_key)
    }

    pub fn store_to_cross_block_module_cache(state_key: StateKey, entry: Arc<ModuleStorageEntry>) {
        MODULE_CACHE.store_module_storage_entry(state_key, entry)
    }

    pub fn flush_cross_block_module_cache() {
        MODULE_CACHE.flush()
    }

    /// Returns new module cache.
    fn empty() -> Self {
        Self {
            modules: RwLock::new(HashMap::new()),
        }
    }

    fn flush(&self) {
        self.modules.write().clear()
    }

    fn get_module_storage_entry(&self, state_key: &StateKey) -> Option<Arc<ModuleStorageEntry>> {
        self.modules.read().get(state_key).cloned()
    }

    fn store_module_storage_entry(&self, state_key: StateKey, entry: Arc<ModuleStorageEntry>) {
        let needs_update = match self.modules.read().get(&state_key) {
            None => true,
            Some(existing_entry) => existing_entry.hash() != entry.hash(),
        };
        if needs_update {
            let mut modules = self.modules.write();
            modules.insert(state_key, entry);
        }
    }
}

static MODULE_CACHE: Lazy<CrossBlockModuleCache> = Lazy::new(CrossBlockModuleCache::empty);
