// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    code_cache_global::GlobalModuleCache,
    counters::{
        GLOBAL_MODULE_CACHE_NUM_MODULES, GLOBAL_MODULE_CACHE_SIZE_IN_BYTES,
        STRUCT_NAME_INDEX_MAP_NUM_ENTRIES,
    },
};
use aptos_types::{
    block_executor::{
        config::BlockExecutorModuleCacheLocalConfig,
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    error::PanicError,
    state_store::StateView,
    vm::modules::AptosModuleExtension,
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::alert;
use aptos_vm_types::module_and_script_storage::{AptosCodeStorageAdapter, AsAptosCodeStorage};
use move_binary_format::{
    errors::{Location, VMError},
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress, ident_str, language_storage::ModuleId, vm_status::VMStatus,
};
use move_vm_runtime::{Module, ModuleStorage, WithRuntimeEnvironment};
use move_vm_types::code::WithSize;
use parking_lot::{Mutex, MutexGuard};
use std::{hash::Hash, ops::Deref, sync::Arc};

/// Raises an alert with the specified message. In case we run in testing mode, instead prints the
/// message to standard output.
macro_rules! alert_or_println {
    ($($arg:tt)*) => {
        if cfg!(any(test, feature = "testing")) {
            println!($($arg)*)
        } else {

            use aptos_vm_logging::{alert, prelude::CRITICAL_ERRORS};
            use aptos_logger::error;
            alert!($($arg)*);
        }
    };
}

/// Manages module caches and the execution environment, possibly across multiple blocks.
pub struct ModuleCacheManager<K, D, V, E> {
    /// Records the last observed metadata associated with a batch of executed transactions. When a
    /// new batch of transactions is about to be executed, the associated metadata can be checked
    /// to ensure that the execution history is linear.
    transaction_slice_metadata: TransactionSliceMetadata,

    /// The execution environment, initially set to [None]. The environment, as long as it does not
    /// change, can be kept for multiple block executions.
    environment: Option<AptosEnvironment>,
    /// Module cache, initially empty, that can be used for parallel block execution. It is the
    /// responsibility of [ModuleCacheManager] to ensure it stays in sync with the environment and
    /// the state.
    module_cache: GlobalModuleCache<K, D, V, E>,
}

impl<K, D, V, E> ModuleCacheManager<K, D, V, E>
where
    K: Hash + Eq + Clone,
    V: Deref<Target = Arc<D>>,
    E: WithSize,
{
    /// Returns a new instance of [ModuleCacheManager].
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            transaction_slice_metadata: TransactionSliceMetadata::unknown(),
            environment: None,
            module_cache: GlobalModuleCache::empty(),
        }
    }

    /// Checks if the manager is ready for execution. That is:
    ///   1. If previously recorded transaction metadata is not immediately before, flushes module
    ///      and environment.
    ///   2. Sets the metadata to the new one.
    ///   3. Checks if environment is set and is the same. If not, resets it. Module caches are
    ///      flushed in case of resets.
    ///   4. Checks sizes of type and module caches. If they are too large, caches are flushed.
    fn check_ready(
        &mut self,
        storage_environment: AptosEnvironment,
        config: &BlockExecutorModuleCacheLocalConfig,
        transaction_slice_metadata: TransactionSliceMetadata,
    ) -> Result<(), VMStatus> {
        // If we execute non-consecutive sequence of transactions, we need to flush everything.
        if !transaction_slice_metadata.is_immediately_after(&self.transaction_slice_metadata) {
            self.module_cache.flush();
            self.environment = None;
        }
        // Record the new metadata for this slice of transactions.
        self.transaction_slice_metadata = transaction_slice_metadata;

        // Next, check the environment. If the current environment has not been set, or is
        // different, we reset it to the new one, and flush the module cache.
        let environment_requires_update = self
            .environment
            .as_ref()
            .map_or(true, |environment| environment != &storage_environment);
        if environment_requires_update {
            self.environment = Some(storage_environment);
            self.module_cache.flush();
        }

        let environment = self.environment.as_ref().expect("Environment must be set");
        let runtime_environment = environment.runtime_environment();

        let struct_name_index_map_size = runtime_environment
            .struct_name_index_map_size()
            .map_err(|err| err.finish(Location::Undefined).into_vm_status())?;
        STRUCT_NAME_INDEX_MAP_NUM_ENTRIES.set(struct_name_index_map_size as i64);

        // If the environment caches too many struct names, flush type caches. Also flush module
        // caches because they contain indices for struct names.
        if struct_name_index_map_size > config.max_struct_name_index_map_num_entries {
            runtime_environment.flush_struct_name_and_info_caches();
            self.module_cache.flush();
        }

        let module_cache_size_in_bytes = self.module_cache.size_in_bytes();
        GLOBAL_MODULE_CACHE_SIZE_IN_BYTES.set(module_cache_size_in_bytes as i64);
        GLOBAL_MODULE_CACHE_NUM_MODULES.set(self.module_cache.num_modules() as i64);

        // If module cache stores too many modules, flush it as well.
        if module_cache_size_in_bytes > config.max_module_cache_size_in_bytes {
            self.module_cache.flush();
        }

        Ok(())
    }
}

/// Module cache manager used by Aptos block executor. Ensures that only one thread has exclusive
/// access to it at a time.
pub struct AptosModuleCacheManager {
    inner: Mutex<ModuleCacheManager<ModuleId, CompiledModule, Module, AptosModuleExtension>>,
}

impl AptosModuleCacheManager {
    /// Returns a new manager in its default (empty) state.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(ModuleCacheManager::new()),
        }
    }

    /// Tries to lock the manager. If succeeds, checks if the manager (caches, environment, etc.)
    /// is ready for execution and updates states. If fails, [AptosModuleCacheManagerGuard::None]
    /// is returned with an empty module cache.
    fn try_lock_inner(
        &self,
        state_view: &impl StateView,
        config: &BlockExecutorModuleCacheLocalConfig,
        transaction_slice_metadata: TransactionSliceMetadata,
    ) -> Result<AptosModuleCacheManagerGuard<'_>, VMStatus> {
        // Get the current environment from storage.
        let storage_environment =
            AptosEnvironment::new_with_delayed_field_optimization_enabled(&state_view);

        Ok(match self.inner.try_lock() {
            Some(mut guard) => {
                guard.check_ready(storage_environment, config, transaction_slice_metadata)?;
                AptosModuleCacheManagerGuard::Guard { guard }
            },
            None => {
                // TODO(loader_v2): Should we return an error here instead?
                alert_or_println!("Locking module cache manager failed, fallback to empty caches");

                // If this is true, we failed to acquire a lock, and so default storage environment
                // and empty (thread-local) module caches will be used.
                AptosModuleCacheManagerGuard::None {
                    environment: storage_environment,
                    module_cache: GlobalModuleCache::empty(),
                }
            },
        })
    }

    /// Tries to lock the manager using [AptosModuleCacheManager::try_lock_inner]. Additionally, if
    /// the module cache is empty, can prefetch Aptos framework into it.
    pub fn try_lock(
        &self,
        state_view: &impl StateView,
        config: &BlockExecutorModuleCacheLocalConfig,
        transaction_slice_metadata: TransactionSliceMetadata,
    ) -> Result<AptosModuleCacheManagerGuard<'_>, VMStatus> {
        let mut guard = self.try_lock_inner(state_view, config, transaction_slice_metadata)?;

        // To avoid cold starts, fetch the framework code. This ensures the state with 0 modules
        // cached is not possible for block execution (as long as the config enables the framework
        // prefetch).
        let environment = guard.environment();
        if environment.features().is_loader_v2_enabled()
            && guard.module_cache().num_modules() == 0
            && config.prefetch_framework_code
        {
            let code_storage = state_view.as_aptos_code_storage(environment.clone());
            prefetch_aptos_framework(code_storage, guard.module_cache_mut()).map_err(|err| {
                alert_or_println!("Failed to load Aptos framework to module cache: {:?}", err);
                VMError::from(err).into_vm_status()
            })?;
        }

        Ok(guard)
    }
}

/// A guard that can be acquired from [AptosModuleCacheManager]. Variants represent successful and
/// no-successful lock acquisition.
pub enum AptosModuleCacheManagerGuard<'a> {
    /// Holds the guard to the [AptosModuleCacheManager], and has exclusive access to it.
    Guard {
        guard: MutexGuard<
            'a,
            ModuleCacheManager<ModuleId, CompiledModule, Module, AptosModuleExtension>,
        >,
    },
    /// Either there is no [AptosModuleCacheManager], or acquiring the lock for it failed.
    None {
        environment: AptosEnvironment,
        module_cache: GlobalModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension>,
    },
}

impl<'a> AptosModuleCacheManagerGuard<'a> {
    /// Returns the references to the environment. If environment is not set, panics.
    pub fn environment(&self) -> &AptosEnvironment {
        use AptosModuleCacheManagerGuard::*;
        match self {
            Guard { guard } => guard
                .environment
                .as_ref()
                .expect("Guard always has environment set"),
            None { environment, .. } => environment,
        }
    }

    /// Returns the references to the module cache.
    pub fn module_cache(
        &self,
    ) -> &GlobalModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension> {
        use AptosModuleCacheManagerGuard::*;
        match self {
            Guard { guard } => &guard.module_cache,
            None { module_cache, .. } => module_cache,
        }
    }

    /// Returns the mutable references to the module cache.
    pub fn module_cache_mut(
        &mut self,
    ) -> &mut GlobalModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension> {
        use AptosModuleCacheManagerGuard::*;
        match self {
            Guard { guard } => &mut guard.module_cache,
            None { module_cache, .. } => module_cache,
        }
    }

    /// A guard in [AptosModuleCacheManagerGuard::None] state with empty module cache and default
    /// environment. Use for testing only.
    #[cfg(test)]
    pub(crate) fn none() -> Self {
        use aptos_types::state_store::MockStateView;
        AptosModuleCacheManagerGuard::None {
            environment: AptosEnvironment::new(&MockStateView::empty()),
            module_cache: GlobalModuleCache::empty(),
        }
    }
}

/// If Aptos framework exists, loads "transaction_validation.move" and all its transitive
/// dependencies from storage into provided module cache. If loading fails for any reason, a panic
/// error is returned.
fn prefetch_aptos_framework<S: StateView>(
    code_storage: AptosCodeStorageAdapter<S, AptosEnvironment>,
    module_cache: &mut GlobalModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension>,
) -> Result<(), PanicError> {
    // If framework code exists in storage, the transitive closure will be verified and cached.
    let maybe_loaded = code_storage
        .fetch_verified_module(&AccountAddress::ONE, ident_str!("transaction_validation"))
        .map_err(|err| {
            // There should be no errors when pre-fetching the framework, if there are, we
            // better return an error here.
            PanicError::CodeInvariantError(format!("Unable to fetch Aptos framework: {:?}", err))
        })?;

    if maybe_loaded.is_some() {
        // Framework must have been loaded. Drain verified modules from local cache into
        // global cache.
        let verified_module_code_iter = code_storage.into_verified_module_code_iter()?;
        module_cache.insert_verified(verified_module_code_iter)?;
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_language_e2e_tests::{data_store::FakeDataStore, executor::FakeExecutor};
    use aptos_types::{
        on_chain_config::{FeatureFlag, Features, OnChainConfig},
        state_store::{state_key::StateKey, state_value::StateValue, MockStateView},
    };
    use claims::assert_ok;
    use std::collections::HashMap;

    #[test]
    fn test_prefetch_existing_aptos_framework() {
        let executor = FakeExecutor::from_head_genesis();
        let state_view = executor.get_state_view();

        let environment = AptosEnvironment::new_with_delayed_field_optimization_enabled(state_view);
        let code_storage = state_view.as_aptos_code_storage(environment);

        let mut module_cache = GlobalModuleCache::empty();
        assert_eq!(module_cache.num_modules(), 0);

        let result = prefetch_aptos_framework(code_storage, &mut module_cache);
        assert!(result.is_ok());
        assert!(module_cache.num_modules() > 0);
    }

    #[test]
    fn test_prefetch_non_existing_aptos_framework() {
        let state_view = FakeDataStore::default();

        let environment =
            AptosEnvironment::new_with_delayed_field_optimization_enabled(&state_view);
        let code_storage = state_view.as_aptos_code_storage(environment);

        let mut module_cache = GlobalModuleCache::empty();
        assert_eq!(module_cache.num_modules(), 0);

        let result = prefetch_aptos_framework(code_storage, &mut module_cache);
        assert!(result.is_ok());
        assert_eq!(module_cache.num_modules(), 0);
    }

    #[allow(dead_code)]
    fn state_view_with_changed_feature_flag(
        feature_flag: Option<FeatureFlag>,
    ) -> MockStateView<StateKey> {
        // Tweak feature flags to force a different config.
        let mut features = Features::default();

        if let Some(feature_flag) = feature_flag {
            if features.is_enabled(feature_flag) {
                features.disable(feature_flag);
            } else {
                features.enable(feature_flag);
            }
        }

        MockStateView::new(HashMap::from([(
            StateKey::resource(Features::address(), &Features::struct_tag()).unwrap(),
            StateValue::new_legacy(bcs::to_bytes(&features).unwrap().into()),
        )]))
    }

    #[test]
    fn test_check_ready_sets_transaction_slice_metadata() {
        let state_view = MockStateView::empty();
        let config = BlockExecutorModuleCacheLocalConfig {
            prefetch_framework_code: false,
            max_module_cache_size_in_bytes: 8,
            max_struct_name_index_map_num_entries: 2,
        };

        let manager = AptosModuleCacheManager::new();
        assert_eq!(
            manager.inner.lock().transaction_slice_metadata,
            TransactionSliceMetadata::Unknown
        );

        let metadata_1 = TransactionSliceMetadata::block_from_u64(0, 1);
        assert_ok!(manager.try_lock(&state_view, &config, metadata_1));
        assert_eq!(manager.inner.lock().transaction_slice_metadata, metadata_1);

        let metadata_2 = TransactionSliceMetadata::block_from_u64(1, 2);
        assert_ok!(manager.try_lock(&state_view, &config, metadata_2));
        assert_eq!(manager.inner.lock().transaction_slice_metadata, metadata_2);
    }

    // TODO(loader_v2): Add more unit tests like with previous commits.
}
