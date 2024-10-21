// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::explicit_sync_wrapper::ExplicitSyncWrapper;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{error::PanicError, state_store::StateView, vm::modules::AptosModuleExtension};
use aptos_vm_environment::environment::AptosEnvironment;
use crossbeam::utils::CachePadded;
use hashbrown::HashMap;
use move_binary_format::{errors::Location, CompiledModule};
use move_core_types::{language_storage::ModuleId, vm_status::VMStatus};
use move_vm_runtime::{Module, WithRuntimeEnvironment};
use move_vm_types::code::ModuleCode;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::{
    hash::Hash,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

/// The maximum size of struct name index map in runtime environment. Checked at block boundaries
/// only.
const MAX_STRUCT_NAME_INDEX_MAP_SIZE: usize = 100_000;

/// A cached environment that can be persisted across blocks. Used by block executor only.
static CROSS_BLOCK_ENVIRONMENT: Lazy<Mutex<Option<AptosEnvironment>>> =
    Lazy::new(|| Mutex::new(None));

/// Returns the cached environment if it exists and has the same configuration as if it was
/// created based on the current state, or creates a new one and caches it. Should only be
/// called at the block boundaries.
pub fn get_environment_with_delayed_field_optimization_enabled(
    state_view: &impl StateView,
) -> Result<AptosEnvironment, VMStatus> {
    // Create a new environment.
    let current_env = AptosEnvironment::new_with_delayed_field_optimization_enabled(state_view);

    // Lock the cache, and check if the environment is the same.
    let mut cross_block_environment = CROSS_BLOCK_ENVIRONMENT.lock();
    if let Some(previous_env) = cross_block_environment.as_ref() {
        if &current_env == previous_env {
            let runtime_env = previous_env.runtime_environment();
            let struct_name_index_map_size = runtime_env
                .struct_name_index_map_size()
                .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
            if struct_name_index_map_size > MAX_STRUCT_NAME_INDEX_MAP_SIZE {
                // Cache is too large, flush it. Also flush the module cache.
                runtime_env.flush_struct_name_and_info_caches();
                get_global_module_cache().flush_unchecked();
            }
            return Ok(previous_env.clone());
        }
    }

    // It is not cached or has changed, so we have to reset it. As a result, we need to flush
    // the cross-block cache because we need to reload all modules with new configs.
    *cross_block_environment = Some(current_env.clone());
    drop(cross_block_environment);
    get_global_module_cache().flush_unchecked();

    Ok(current_env)
}

/// Module code stored in cross-block module cache.
struct ImmutableModuleCode<DC, VC, E> {
    /// True if this code is "valid" within the block execution context (i.e, there has been no
    /// republishing of this module so far). If false, executor needs to read the module from the
    /// sync/unsync module caches.
    valid: CachePadded<AtomicBool>,
    /// Cached verified module. While [ModuleCode] type is used, the following invariants always
    /// hold:
    ///    1. Module's version is [None] (storage version).
    ///    2. Module's code is always verified.
    module: CachePadded<Arc<ModuleCode<DC, VC, E, Option<TxnIndex>>>>,
}

impl<DC, VC, E> ImmutableModuleCode<DC, VC, E>
where
    VC: Deref<Target = Arc<DC>>,
{
    /// Returns a new valid module. Returns a (panic) error if the module is not verified or has
    /// non-storage version.
    fn new(module: Arc<ModuleCode<DC, VC, E, Option<TxnIndex>>>) -> Result<Self, PanicError> {
        if !module.code().is_verified() || module.version().is_some() {
            let msg = format!(
                "Invariant violated for immutable module code : verified ({}), version({:?})",
                module.code().is_verified(),
                module.version()
            );
            return Err(PanicError::CodeInvariantError(msg));
        }

        Ok(Self {
            valid: CachePadded::new(AtomicBool::new(true)),
            module: CachePadded::new(module),
        })
    }

    /// Marks the module as invalid.
    fn mark_invalid(&self) {
        self.valid.store(false, Ordering::Release)
    }

    /// Returns true if the module is valid.
    pub(crate) fn is_valid(&self) -> bool {
        self.valid.load(Ordering::Acquire)
    }

    /// Returns the module code stored is this [ImmutableModuleCode].
    fn inner(&self) -> &Arc<ModuleCode<DC, VC, E, Option<TxnIndex>>> {
        self.module.deref()
    }
}

/// An immutable cache for verified code, that can be accessed concurrently thought the block, and
/// only modified at block boundaries.
pub struct ImmutableModuleCache<K, DC, VC, E> {
    /// Module cache containing the verified code.
    module_cache: ExplicitSyncWrapper<HashMap<K, ImmutableModuleCode<DC, VC, E>>>,
    /// Maximum cache size. If the size is greater than this limit, the cache is flushed. Note that
    /// this can only be done at block boundaries.
    capacity: usize,
}

impl<K, DC, VC, E> ImmutableModuleCache<K, DC, VC, E>
where
    K: Hash + Eq + Clone,

    VC: Deref<Target = Arc<DC>>,
{
    /// Returns new empty module cache with default capacity.
    pub(crate) fn empty() -> Self {
        let default_capacity = 100_000;
        Self::with_capacity(default_capacity)
    }

    /// Returns new empty module cache with specified capacity.
    fn with_capacity(capacity: usize) -> Self {
        Self {
            module_cache: ExplicitSyncWrapper::new(HashMap::new()),
            capacity,
        }
    }

    /// Returns true if the key exists in immutable cache and the corresponding module is valid.
    pub(crate) fn contains_valid(&self, key: &K) -> bool {
        self.module_cache
            .acquire()
            .get(key)
            .is_some_and(|module| module.is_valid())
    }

    /// Marks the cached module (if it exists) as invalid. As a result, all subsequent calls to the
    /// cache for the associated key  will result in a cache miss. Note that it is fine for an
    /// entry not to exist, in which case this is a no-op.
    pub(crate) fn mark_invalid(&self, key: &K) {
        if let Some(module) = self.module_cache.acquire().get(key) {
            module.mark_invalid();
        }
    }

    /// Returns the module stored in cache. If the module has not been cached, or it exists but is
    /// not valid, [None] is returned.
    pub(crate) fn get(&self, key: &K) -> Option<Arc<ModuleCode<DC, VC, E, Option<TxnIndex>>>> {
        self.module_cache.acquire().get(key).and_then(|module| {
            if module.is_valid() {
                Some(module.inner().clone())
            } else {
                None
            }
        })
    }

    /// Flushes the cache. Should never be called throughout block-execution. Use with caution.
    pub fn flush_unchecked(&self) {
        self.module_cache.acquire().clear();
    }

    /// Inserts modules into the cache. Should never be called throughout block-execution. Use with
    /// caution.
    ///
    /// Notes:
    ///   1. Only verified modules are inserted.
    ///   2. Versions of inserted modules is set to [None] (storage version).
    ///   3. Valid modules should not be removed, and new modules should have unique ownership. If
    ///      these constraints are violated, a panic error is returned.
    ///   4. If the cache size exceeds its capacity after all verified modules have been inserted,
    ///      the cache is flushed.
    pub(crate) fn insert_verified_unchecked(
        &self,
        modules: impl Iterator<Item = (K, Arc<ModuleCode<DC, VC, E, Option<TxnIndex>>>)>,
    ) -> Result<(), PanicError> {
        let mut guard = self.module_cache.acquire();
        let module_cache = guard.dereference_mut();

        for (key, module) in modules {
            if module.code().is_verified() {
                let mut module = module.as_ref().clone();
                module.set_version(None);
                let prev =
                    module_cache.insert(key.clone(), ImmutableModuleCode::new(Arc::new(module))?);

                if prev.is_some_and(|prev_module| prev_module.is_valid()) {
                    return Err(PanicError::CodeInvariantError(
                        "Overwriting a valid module".to_string(),
                    ));
                }
            }
        }

        if module_cache.len() > self.capacity {
            module_cache.clear();
        }

        Ok(())
    }

    /// Insert the module to cache. Used for tests only.
    #[cfg(test)]
    pub(crate) fn insert(&self, key: K, module: Arc<ModuleCode<DC, VC, E, Option<TxnIndex>>>) {
        self.module_cache
            .acquire()
            .insert(key, ImmutableModuleCode::new(module).unwrap());
    }

    /// Removes the module from cache. Used for tests only.
    #[cfg(test)]
    pub(crate) fn remove(&self, key: &K) {
        self.module_cache.acquire().remove(key);
    }

    /// Returns the size of the cache. Used for tests only.
    #[cfg(test)]
    pub(crate) fn size(&self) -> usize {
        self.module_cache.acquire().len()
    }
}

/// Immutable global cache. The size of the cache is fixed within a single block (modules are not
/// inserted or removed) and it is only mutated at the block boundaries. At the same time, modules
/// in this cache can be marked as "invalid" so that block executor can decide on whether to read
/// the module from this cache or from elsewhere.
#[allow(clippy::redundant_closure)]
static CROSS_BLOCK_MODULE_CACHE: Lazy<
    ImmutableModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension>,
> = Lazy::new(|| ImmutableModuleCache::empty());

/// Returns the module from the cross module cache. If the module has not been cached, or is
/// no longer valid due to module publishing, [None] is returned.
pub fn get_global_module_cache(
) -> &'static ImmutableModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension> {
    &CROSS_BLOCK_MODULE_CACHE
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::types::test_types::{
        mock_deserialized_code, mock_verified_code, module_id, verified_code,
    };
    use aptos_types::{
        on_chain_config::{FeatureFlag, Features},
        state_store::{
            errors::StateviewError, state_key::StateKey, state_storage_usage::StateStorageUsage,
            state_value::StateValue, StateViewId, TStateView,
        },
    };
    use claims::{assert_err, assert_ok, assert_some};

    #[test]
    fn test_immutable_module_code() {
        assert!(ImmutableModuleCode::new(mock_deserialized_code(0, None)).is_err());
        assert!(ImmutableModuleCode::new(mock_deserialized_code(0, Some(22))).is_err());
        assert!(ImmutableModuleCode::new(mock_verified_code(0, Some(22))).is_err());
        assert!(ImmutableModuleCode::new(mock_verified_code(0, None)).is_ok());
    }

    #[test]
    fn test_immutable_module_code_validity() {
        let module_code = assert_ok!(ImmutableModuleCode::new(mock_verified_code(0, None)));
        assert!(module_code.is_valid());

        module_code.mark_invalid();
        assert!(!module_code.is_valid());
    }

    #[test]
    fn test_global_module_cache() {
        let global_cache = ImmutableModuleCache::empty();

        global_cache.insert(0, mock_verified_code(0, None));
        global_cache.insert(1, mock_verified_code(1, None));
        global_cache.mark_invalid(&1);

        assert_eq!(global_cache.size(), 2);

        assert!(global_cache.contains_valid(&0));
        assert!(!global_cache.contains_valid(&1));
        assert!(!global_cache.contains_valid(&3));

        assert!(global_cache.get(&0).is_some());
        assert!(global_cache.get(&1).is_none());
        assert!(global_cache.get(&3).is_none());
    }

    #[test]
    fn test_insert_verified_for_global_module_cache() {
        let capacity = 10;
        let global_cache = ImmutableModuleCache::with_capacity(capacity);

        let mut new_modules = vec![];
        for i in 0..capacity {
            new_modules.push((i, mock_verified_code(i, Some(i as TxnIndex))));
        }
        let result = global_cache.insert_verified_unchecked(new_modules.into_iter());
        assert!(result.is_ok());
        assert_eq!(global_cache.size(), capacity);

        // Versions should be set to storage.
        for key in 0..capacity {
            let code = assert_some!(global_cache.get(&key));
            assert!(code.version().is_none())
        }

        // Too many modules added, the cache should be flushed.
        let new_modules = vec![(11, mock_verified_code(11, None))];
        let result = global_cache.insert_verified_unchecked(new_modules.into_iter());
        assert!(result.is_ok());
        assert_eq!(global_cache.size(), 0);

        // Should not add deserialized code.
        let deserialized_modules = vec![(0, mock_deserialized_code(0, None))];
        assert_ok!(global_cache.insert_verified_unchecked(deserialized_modules.into_iter()));
        assert_eq!(global_cache.size(), 0);

        // Should not override valid modules.
        global_cache.insert(0, mock_verified_code(0, None));
        let new_modules = vec![(0, mock_verified_code(100, None))];
        assert_err!(global_cache.insert_verified_unchecked(new_modules.into_iter()));

        // Can override invalid modules.
        global_cache.mark_invalid(&0);
        let new_modules = vec![(0, mock_verified_code(100, None))];
        let result = global_cache.insert_verified_unchecked(new_modules.into_iter());
        assert!(result.is_ok());
        assert_eq!(global_cache.size(), 1);
    }

    #[derive(Default)]
    struct HashMapView {
        data: HashMap<StateKey, StateValue>,
    }

    impl TStateView for HashMapView {
        type Key = StateKey;

        fn get_state_value(
            &self,
            state_key: &Self::Key,
        ) -> Result<Option<StateValue>, StateviewError> {
            Ok(self.data.get(state_key).cloned())
        }

        fn id(&self) -> StateViewId {
            unreachable!("Not used in tests");
        }

        fn get_usage(&self) -> Result<StateStorageUsage, StateviewError> {
            unreachable!("Not used in tests");
        }
    }

    #[test]
    fn test_cross_block_module_cache_flush() {
        let c_id = module_id("c");
        get_global_module_cache().insert(c_id.clone(), verified_code("c", None));
        assert_eq!(get_global_module_cache().size(), 1);

        get_global_module_cache().flush_unchecked();
        assert_eq!(get_global_module_cache().size(), 0);

        // Now check that cache is flushed when the environment is flushed.
        let mut state_view = HashMapView::default();
        let env_old = AptosEnvironment::new_with_delayed_field_optimization_enabled(&state_view);

        for i in 0..10 {
            let name = format!("m_{}", i);
            let id = module_id(&name);
            get_global_module_cache().insert(id.clone(), verified_code(&name, None));
        }
        assert_eq!(get_global_module_cache().size(), 10);

        let state_key = StateKey::on_chain_config::<Features>().unwrap();
        let mut features = Features::default();
        features.disable(FeatureFlag::KEYLESS_ACCOUNTS);
        state_view.data.insert(
            state_key,
            StateValue::new_legacy(bcs::to_bytes(&features).unwrap().into()),
        );

        // New environment means we need to also flush global caches - to invalidate struct name
        // indices.
        let env_new = assert_ok!(get_environment_with_delayed_field_optimization_enabled(
            &state_view
        ));
        assert!(env_old != env_new);
        assert_eq!(get_global_module_cache().size(), 0);
    }
}
