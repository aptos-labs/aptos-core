// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::explicit_sync_wrapper::ExplicitSyncWrapper;
use aptos_mvhashmap::code_cache::SyncCodeCache;
use aptos_types::{
    state_store::{state_value::StateValueMetadata, StateView},
    vm::modules::ModuleCacheEntry,
};
use aptos_vm_environment::environment::AptosEnvironment;
use bytes::Bytes;
use crossbeam::utils::CachePadded;
use hashbrown::HashMap;
use move_binary_format::CompiledModule;
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, language_storage::ModuleId,
    metadata::Metadata,
};
use move_vm_runtime::Module;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
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

/// A cached environment that can be persisted across blocks. Used by block executor only. Also
/// stores an identifier so that we can check when it changes.
pub struct CachedAptosEnvironment {
    id: EnvironmentID,
    env: AptosEnvironment,
}

impl CachedAptosEnvironment {
    /// Returns the cached environment if it exists and has the same configuration as if it was
    /// created based on the current state, or creates a new one and caches it. Should only be
    /// called at the block boundaries.
    pub fn fetch_with_delayed_field_optimization_enabled(
        state_view: &impl StateView,
    ) -> AptosEnvironment {
        // Create a new environment.
        let env = AptosEnvironment::new_with_delayed_field_optimization_enabled(state_view);
        let id = EnvironmentID::new(&env);

        // Lock the cache, and check if the environment is the same.
        let mut cross_block_environment = CROSS_BLOCK_ENVIRONMENT.lock();
        if let Some(cached_env) = cross_block_environment.as_ref() {
            if id == cached_env.id {
                return cached_env.env.clone();
            }
        }

        // It is not, so we have to reset it. Also flush the framework cache because we need to
        // re-load all the modules with new configs.
        *cross_block_environment = Some(CachedAptosEnvironment {
            id,
            env: env.clone(),
        });
        drop(cross_block_environment);
        CrossBlockModuleCache::flush_at_block_start();

        env
    }
}

static CROSS_BLOCK_ENVIRONMENT: Lazy<Mutex<Option<CachedAptosEnvironment>>> =
    Lazy::new(|| Mutex::new(None));

struct CrossBlockModuleCacheEntry {
    valid: CachePadded<AtomicBool>,
    verified_entry: ModuleCacheEntry,
}

impl CrossBlockModuleCacheEntry {
    /// Returns a new valid cache entry. Panics if provided module entry is not verified.
    fn new(entry: ModuleCacheEntry) -> Self {
        assert!(entry.is_verified());
        Self {
            valid: CachePadded::new(AtomicBool::new(true)),
            verified_entry: entry,
        }
    }

    /// Marks the entry as invalid.
    fn mark_invalid(&self) {
        self.valid.store(false, Ordering::Release)
    }

    /// Returns true if the entry is valid.
    #[inline(always)]
    fn is_valid(&self) -> bool {
        self.valid.load(Ordering::Acquire)
    }

    /// Returns the state value metadata if the entry is valid, and [None] otherwise.
    fn state_value_metadata(&self) -> Option<StateValueMetadata> {
        self.is_valid()
            .then(|| self.verified_entry.state_value_metadata().clone())
    }

    /// Returns the module bytes if the entry is valid, and [None] otherwise.
    fn bytes(&self) -> Option<Bytes> {
        self.is_valid().then(|| self.verified_entry.bytes().clone())
    }

    /// Returns the module size in bytes if the entry is valid, and [None] otherwise.
    fn size_in_bytes(&self) -> Option<usize> {
        self.is_valid().then(|| self.verified_entry.size_in_bytes())
    }

    /// Returns the module metadata if the entry is valid, and [None] otherwise.
    fn module_metadata(&self) -> Option<Vec<Metadata>> {
        self.is_valid()
            .then(|| self.verified_entry.metadata().to_vec())
    }

    /// Returns the deserialized module if the entry is valid, and [None] otherwise.
    fn deserialized_module(&self) -> Option<Arc<CompiledModule>> {
        self.is_valid()
            .then(|| self.verified_entry.as_compiled_module())
    }

    /// Returns the verified module if the entry is valid, and [None] otherwise. Panics if the
    /// entry is not verified.
    fn verified_module(&self) -> Option<Arc<Module>> {
        self.is_valid().then(|| {
            self.verified_entry
                .try_as_verified_module()
                .expect("Modules stored in cache are always verified")
        })
    }
}

const CROSS_BLOCK_MODULE_CACHE_SIZE: usize = 100_000;

/// Represents an immutable cross-block cache.
pub(crate) struct CrossBlockModuleCache;

impl CrossBlockModuleCache {
    /// Flushes the module cache. Should only be called at the start of the block.
    fn flush_at_block_start() {
        let mut cache = CROSS_BLOCK_MODULE_CACHE.acquire();
        cache.clear();
    }

    /// Adds new verified entries from block-level cache to the cross-block cache. Flushes the
    /// cache if its size is too large. Should only be called at block end.
    pub(crate) fn populate_cache_at_block_end(code_cache: &SyncCodeCache) {
        let mut cache = CROSS_BLOCK_MODULE_CACHE.acquire();
        if cache.len() > CROSS_BLOCK_MODULE_CACHE_SIZE {
            cache.clear();
        }

        code_cache
            .module_cache()
            .collect_verified_entries_into(cache.dereference_mut(), |e| {
                CrossBlockModuleCacheEntry::new(e.clone())
            });
    }

    /// Marks the cached entry (if it exists) as invalid. As a result, all subsequent calls to the
    /// cache will result in a cache miss.
    pub(crate) fn mark_invalid(module_id: &ModuleId) {
        if let Some(entry) = CROSS_BLOCK_MODULE_CACHE.acquire().get(module_id) {
            entry.mark_invalid();
        }
    }

    /// Returns the state value metadata from the cross module cache. If the module has not been
    /// cached, or is no longer valid due to module publishing, [None] is returned.
    pub(crate) fn fetch_module_state_value_metadata(
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> Option<StateValueMetadata> {
        CROSS_BLOCK_MODULE_CACHE
            .acquire()
            .get(&(address, module_name))?
            .state_value_metadata()
    }

    /// Returns the true if the module exists in the cross module framework cache. If the module
    /// has not been cached, false is returned. Note that even if a module has been republished, we
    /// can still check the cache because modules cannot be deleted.
    pub(crate) fn check_module_exists(address: &AccountAddress, module_name: &IdentStr) -> bool {
        CROSS_BLOCK_MODULE_CACHE
            .acquire()
            .contains_key(&(address, module_name))
    }

    /// Returns the module size in bytes from the cross module cache. If the module has not been
    /// cached, or is no longer valid due to module publishing, [None] is returned.
    pub(crate) fn fetch_module_size_in_bytes(
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> Option<usize> {
        CROSS_BLOCK_MODULE_CACHE
            .acquire()
            .get(&(address, module_name))?
            .size_in_bytes()
    }

    /// Returns the module bytes from the cross module cache. If the module has not been cached, or
    /// is no longer valid due to module publishing, [None] is returned.
    pub(crate) fn fetch_module_bytes(
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> Option<Bytes> {
        CROSS_BLOCK_MODULE_CACHE
            .acquire()
            .get(&(address, module_name))?
            .bytes()
    }

    /// Returns the module metadata from the cross module cache. If the module has not been cached,
    /// or is no longer valid due to module publishing, [None] is returned.
    pub(crate) fn fetch_module_metadata(
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> Option<Vec<Metadata>> {
        CROSS_BLOCK_MODULE_CACHE
            .acquire()
            .get(&(address, module_name))?
            .module_metadata()
    }

    /// Returns the deserialized module from the cross module cache. If the module has not been
    /// cached, or is no longer valid due to module publishing, [None] is returned.
    pub(crate) fn fetch_deserialized_module(
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> Option<Arc<CompiledModule>> {
        CROSS_BLOCK_MODULE_CACHE
            .acquire()
            .get(&(address, module_name))?
            .deserialized_module()
    }

    /// Returns the verified module from the cross module cache. If the module has not been cached,
    /// or is no longer valid due to module publishing, [None] is returned.
    ///
    /// Panics if cache contains a non-verified entry.
    pub(crate) fn fetch_verified_module(
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> Option<Arc<Module>> {
        CROSS_BLOCK_MODULE_CACHE
            .acquire()
            .get(&(address, module_name))?
            .verified_module()
    }
}

type SyncCrossBlockModuleCache = ExplicitSyncWrapper<HashMap<ModuleId, CrossBlockModuleCacheEntry>>;
static CROSS_BLOCK_MODULE_CACHE: Lazy<SyncCrossBlockModuleCache> =
    Lazy::new(|| ExplicitSyncWrapper::new(HashMap::new()));
