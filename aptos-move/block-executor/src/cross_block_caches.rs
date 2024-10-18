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
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

/// The maximum size of struct name index map in runtime environment.
const MAX_STRUCT_NAME_INDEX_MAP_SIZE: usize = 100_000;

/// The maximum size of [CrossBlockModuleCache]. Checked at block boundaries.
const MAX_CROSS_BLOCK_MODULE_CACHE_SIZE: usize = 100_000;

static CROSS_BLOCK_ENVIRONMENT: Lazy<Mutex<Option<AptosEnvironment>>> =
    Lazy::new(|| Mutex::new(None));

/// A cached environment that can be persisted across blocks. Used by block executor only.
pub struct CachedAptosEnvironment;

impl CachedAptosEnvironment {
    /// Returns the cached environment if it exists and has the same configuration as if it was
    /// created based on the current state, or creates a new one and caches it. Should only be
    /// called at the block boundaries.
    pub fn get_with_delayed_field_optimization_enabled(
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
                    // Cache is too large, flush it. Also flush module cache.
                    runtime_env.flush_struct_name_and_info_caches();
                    CrossBlockModuleCache::flush_at_block_start();
                }
                return Ok(previous_env.clone());
            }
        }

        // It is not cached or has changed, so we have to reset it. As a result, we need to flush
        // the cross-block cache because we need to reload all modules with new configs.
        *cross_block_environment = Some(current_env.clone());
        drop(cross_block_environment);
        CrossBlockModuleCache::flush_at_block_start();

        Ok(current_env)
    }
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

type AptosImmutableModuleCode = ImmutableModuleCode<CompiledModule, Module, AptosModuleExtension>;
type SyncCrossBlockModuleCache = ExplicitSyncWrapper<HashMap<ModuleId, AptosImmutableModuleCode>>;
static CROSS_BLOCK_MODULE_CACHE: Lazy<SyncCrossBlockModuleCache> =
    Lazy::new(|| ExplicitSyncWrapper::new(HashMap::new()));

/// Represents an immutable cross-block cache. The size of the cache is fixed (modules cannot be
/// added or removed) within a single block, so it is only mutated at the block boundaries. At the
/// same time, modules in this cache can be marked as "invalid" so that block executor can decide
/// on whether to read the module from this cache or from elsewhere.
pub struct CrossBlockModuleCache;

impl CrossBlockModuleCache {
    /// Flushes the module cache. Should only be called at the start of the block.
    pub fn flush_at_block_start() {
        let mut cache = CROSS_BLOCK_MODULE_CACHE.acquire();
        cache.clear();
    }

    /// Adds new verified modules from block-level cache to the cross-block cache. Flushes the
    /// cache if its size is too large. Should only be called at block end.
    pub(crate) fn populate_from_code_cache_at_block_end(
        modules: impl Iterator<
            Item = (
                ModuleId,
                Arc<ModuleCode<CompiledModule, Module, AptosModuleExtension, Option<TxnIndex>>>,
            ),
        >,
    ) -> Result<(), PanicError> {
        let mut cache = CROSS_BLOCK_MODULE_CACHE.acquire();

        // For all modules that are verified, add them to cache. Also reset version to storage
        // version. Note that at this point it should be the case that all arced modules have the
        // reference count of exactly 1.
        for (id, module) in modules {
            if module.code().is_verified() {
                let mut module = Arc::into_inner(module).ok_or_else(|| {
                    let msg = format!(
                        "Module {}::{} has more than one strong reference count",
                        id.address(),
                        id.name()
                    );
                    PanicError::CodeInvariantError(msg)
                })?;

                module.set_version(None);
                cache.insert(id, ImmutableModuleCode::new(Arc::new(module))?);
            }
        }

        // To protect against running out of memory, keep the size limited to some constant. If it
        // is too large, flush the cache.
        if cache.len() > MAX_CROSS_BLOCK_MODULE_CACHE_SIZE {
            cache.clear();
        }

        Ok(())
    }

    /// Returns true if the module is stored in cross-block cache and is valid.
    pub(crate) fn is_valid(module_id: &ModuleId) -> bool {
        CROSS_BLOCK_MODULE_CACHE
            .acquire()
            .get(module_id)
            .is_some_and(|module| module.is_valid())
    }

    /// Marks the cached entry (if it exists) as invalid. As a result, all subsequent calls to the
    /// cache will result in a cache miss. It is fine for an entry not to exist: e.g., when a new
    /// module is published one can try to invalidate global cache (that does not have the module).
    pub(crate) fn mark_invalid(module_id: &ModuleId) {
        if let Some(module) = CROSS_BLOCK_MODULE_CACHE.acquire().get(module_id) {
            module.mark_invalid();
        }
    }

    /// Returns the module from the cross module cache. If the module has not been cached, or is
    /// no longer valid due to module publishing, [None] is returned.
    pub(crate) fn get(
        module_id: &ModuleId,
    ) -> Option<Arc<ModuleCode<CompiledModule, Module, AptosModuleExtension, Option<TxnIndex>>>>
    {
        CROSS_BLOCK_MODULE_CACHE
            .acquire()
            .get(module_id)
            .and_then(|module| {
                if module.is_valid() {
                    Some(module.inner().clone())
                } else {
                    None
                }
            })
    }

    /// Inserts a module to the cross-block module cache. Used for tests only.
    #[cfg(test)]
    pub fn insert(
        module_id: ModuleId,
        module: Arc<ModuleCode<CompiledModule, Module, AptosModuleExtension, Option<TxnIndex>>>,
    ) {
        let mut cache = CROSS_BLOCK_MODULE_CACHE.acquire();
        cache.insert(module_id, ImmutableModuleCode::new(module).unwrap());
    }

    /// Removes the specified module from cross-block module cache. Used for tests only.
    #[cfg(test)]
    pub fn remove(module_id: &ModuleId) {
        let mut cache = CROSS_BLOCK_MODULE_CACHE.acquire();
        cache.remove(module_id);
    }

    /// Returns the size of the cross-block module cache.
    #[cfg(test)]
    pub fn size() -> usize {
        CROSS_BLOCK_MODULE_CACHE.acquire().len()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::types::test_types::{module_id, verified_code};
    use aptos_types::{
        on_chain_config::{FeatureFlag, Features},
        state_store::{
            errors::StateviewError, state_key::StateKey, state_storage_usage::StateStorageUsage,
            state_value::StateValue, StateViewId, TStateView,
        },
    };
    use claims::assert_ok;
    use move_vm_types::code::{MockDeserializedCode, MockVerifiedCode};

    #[test]
    fn test_immutable_module_code() {
        let module_code: ModuleCode<_, MockVerifiedCode, _, _> =
            ModuleCode::from_deserialized(MockDeserializedCode::new(0), Arc::new(()), None);
        assert!(ImmutableModuleCode::new(Arc::new(module_code)).is_err());

        let module_code =
            ModuleCode::from_verified(MockVerifiedCode::new(0), Arc::new(()), Some(22));
        assert!(ImmutableModuleCode::new(Arc::new(module_code)).is_err());

        let module_code = ModuleCode::from_verified(MockVerifiedCode::new(0), Arc::new(()), None);
        assert!(ImmutableModuleCode::new(Arc::new(module_code)).is_ok());
    }

    #[test]
    fn test_immutable_module_code_validity() {
        let module_code = ModuleCode::from_verified(MockVerifiedCode::new(0), Arc::new(()), None);
        let module_code = assert_ok!(ImmutableModuleCode::new(Arc::new(module_code)));
        assert!(module_code.is_valid());

        module_code.mark_invalid();
        assert!(!module_code.is_valid());
    }

    #[test]
    fn test_cross_block_module_cache() {
        let valid_module_id = module_id("a");
        let valid_module_code = verified_code("a", None);
        CrossBlockModuleCache::insert(valid_module_id.clone(), valid_module_code);

        let invalid_module_id = module_id("b");
        let invalid_module_code = verified_code("b", None);
        CrossBlockModuleCache::insert(invalid_module_id.clone(), invalid_module_code);
        CrossBlockModuleCache::mark_invalid(&invalid_module_id);

        assert_eq!(CrossBlockModuleCache::size(), 2);
        assert!(CrossBlockModuleCache::is_valid(&valid_module_id));
        assert!(!CrossBlockModuleCache::is_valid(&invalid_module_id));

        assert!(CrossBlockModuleCache::get(&valid_module_id).is_some());
        assert!(CrossBlockModuleCache::get(&invalid_module_id).is_none());

        let non_existing_id = module_id("c");
        assert!(CrossBlockModuleCache::get(&non_existing_id).is_none());

        CrossBlockModuleCache::remove(&valid_module_id);
        CrossBlockModuleCache::remove(&invalid_module_id);
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
        CrossBlockModuleCache::insert(c_id.clone(), verified_code("c", None));
        assert_eq!(CrossBlockModuleCache::size(), 1);

        CrossBlockModuleCache::flush_at_block_start();
        assert_eq!(CrossBlockModuleCache::size(), 0);

        // Now check that cache is flushed when the environment is flushed.
        let mut state_view = HashMapView::default();
        let env_old = AptosEnvironment::new_with_delayed_field_optimization_enabled(&state_view);

        for i in 0..10 {
            let name = format!("m_{}", i);
            let id = module_id(&name);
            CrossBlockModuleCache::insert(id.clone(), verified_code(&name, None));
        }
        assert_eq!(CrossBlockModuleCache::size(), 10);

        let state_key = StateKey::on_chain_config::<Features>().unwrap();
        let mut features = Features::default();
        features.disable(FeatureFlag::KEYLESS_ACCOUNTS);
        state_view.data.insert(
            state_key,
            StateValue::new_legacy(bcs::to_bytes(&features).unwrap().into()),
        );

        // New environment means we need to also flush global caches - to invalidate struct name
        // indices.
        let env_new = assert_ok!(
            CachedAptosEnvironment::get_with_delayed_field_optimization_enabled(&state_view)
        );
        assert!(env_old != env_new);
        assert_eq!(CrossBlockModuleCache::size(), 0);
    }
}
