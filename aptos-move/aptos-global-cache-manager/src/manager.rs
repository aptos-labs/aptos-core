// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::config::GlobalCacheConfig;
use aptos_crypto::HashValue;
use aptos_types::{
    read_only_module_cache::ReadOnlyModuleCache, state_store::StateView,
    vm::modules::AptosModuleExtension,
};
use aptos_vm_environment::environment::AptosEnvironment;
use move_binary_format::{errors::Location, CompiledModule};
use move_core_types::{
    language_storage::ModuleId,
    vm_status::{StatusCode, VMStatus},
};
use move_vm_runtime::{Module, WithRuntimeEnvironment};
use parking_lot::Mutex;
use std::{
    hash::Hash,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

/// Returns an invariant violation [VMStatus].
fn invariant_violation(msg: &str) -> VMStatus {
    VMStatus::error(
        StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
        Some(msg.to_string()),
    )
}

/// Represents previously executed block, recorded by [GlobalCacheManager].
#[derive(Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Ord)]
enum BlockId {
    /// No block has been executed yet.
    Unset,
    /// Block of transactions has been executed, with known or unknown hash. Usually, the hash is
    /// [None] in tests, replay, etc.
    Set(Option<HashValue>),
}

impl BlockId {
    /// Returns true if the ID corresponds to no executed blocks.
    fn is_unset(&self) -> bool {
        matches!(self, Self::Unset)
    }
}

/// Manages global caches, e.g., modules or execution environment. Should not be used concurrently.
pub struct GlobalCacheManagerInner<K, DC, VC, E> {
    /// Different configurations used for handling global caches.
    config: GlobalCacheConfig,
    /// Cache for modules. It is read-only for any concurrent execution, and can only be mutated
    /// when it is known that there are no concurrent accesses, e.g., at blok boundaries.
    /// [GlobalCacheManager] tries to ensure that these invariants always hold.
    module_cache: Arc<ReadOnlyModuleCache<K, DC, VC, E>>,

    /// Identifies previously executed block, initially [BlockId::Unset].
    previous_block_id: Mutex<BlockId>,
    /// Identifies the previously used execution environment, initially [None]. The environment, as
    /// long as it does not change, it maintained across multiple block executions.
    previous_environment: Mutex<Option<AptosEnvironment>>,

    /// A marker that indicates that the state of global caches is ready for block execution. Used
    /// to prevent concurrent block executions.
    ready_for_next_block: AtomicBool,
}

impl<K, DC, VC, E> GlobalCacheManagerInner<K, DC, VC, E>
where
    K: Hash + Eq + Clone,
    VC: Deref<Target = Arc<DC>>,
{
    /// Returns a new instance of [GlobalCacheManagerInner] with default [GlobalCacheConfig].
    pub fn new_with_default_config() -> Self {
        Self::new_with_config(GlobalCacheConfig::default())
    }

    /// Returns a new instance of [GlobalCacheManagerInner] with the provided [GlobalCacheConfig].
    pub fn new_with_config(config: GlobalCacheConfig) -> Self {
        Self {
            config,
            module_cache: Arc::new(ReadOnlyModuleCache::empty()),
            previous_block_id: Mutex::new(BlockId::Unset),
            previous_environment: Mutex::new(None),
            ready_for_next_block: AtomicBool::new(true),
        }
    }

    /// Sets the state of global caches prior to block execution on top of the provided state (with
    /// the block ID). Should always sbe called prior to block execution.
    ///
    /// The caches stored globally (modules, struct name re-indexing map and type caches) are all
    /// flushed if:
    ///   1. Previously executed block ID does not match the provided value.
    ///   2. The environment has changed for this state.
    ///   3. The size of the struct name re-indexing map is too large.
    ///   4. The size of the module cache is too large.
    ///
    /// Marks [GlobalCacheManagerInner] as not ready for next block execution. If called
    /// concurrently, only a single invocation ever succeeds and other calls return an error.
    pub fn mark_block_execution_start(
        &self,
        state_view: &impl StateView,
        previous_block_id: Option<HashValue>,
    ) -> Result<(), VMStatus> {
        let recorded_previous_block_id = {
            // Acquire a lock, and check if we are ready to execute the next block.
            let previous_block_id = self.previous_block_id.lock();
            if !self.ready_for_next_block.load(Ordering::SeqCst) {
                let msg = "Trying to execute blocks concurrently over shared global state";
                return Err(invariant_violation(msg));
            }

            // Prepare for execution. Set the flag as not ready to ensure that blocks are not
            // executed concurrently using the same cache.
            self.ready_for_next_block.store(false, Ordering::SeqCst);
            *previous_block_id
        };

        // From here, we perform checks if we need to flush the global caches. If so, this variable
        // is set to true.
        let mut flush_all_caches = false;

        // Check 1: We must be executing on top of the state we have seen just before.
        use BlockId::*;
        match (recorded_previous_block_id, previous_block_id) {
            // We execute on top of empty state, everything is ok.
            (Unset, None) | (Unset, Some(_)) => {},

            // We execute on top of different (maybe also unspecified) state. In this case, caches
            // need to be reset.
            (Set(None), None) | (Set(None), Some(_)) | (Set(Some(_)), None) => {
                flush_all_caches = true;
            },

            // Otherwise, just check if block hashes do not match.
            (Set(Some(recorded_hash)), Some(hash)) => {
                if recorded_hash != hash {
                    flush_all_caches = true;
                };
            },
        };

        // Check 2: Reset global environment if it has changed. If so, caches needs to be flushed.
        let new_environment =
            AptosEnvironment::new_with_delayed_field_optimization_enabled(state_view);
        let mut previous_environment = self.previous_environment.lock();
        match previous_environment.as_ref() {
            Some(environment) => {
                if environment != &new_environment {
                    *previous_environment = Some(new_environment);
                    flush_all_caches = true;
                }
            },
            None => {
                // If the environment is not yet set, set it.
                debug_assert!(self.previous_block_id.lock().is_unset());
                *previous_environment = Some(new_environment);
            },
        }

        // Check 3: At this point, environment is set to the most-up-to-date value. Check the size
        // of caches is within bounds.
        let runtime_environment = previous_environment
            .as_ref()
            .expect("Environment has to be set")
            .runtime_environment();
        let struct_name_index_map_size = runtime_environment
            .struct_name_index_map_size()
            .map_err(|err| err.finish(Location::Undefined).into_vm_status())?;
        if struct_name_index_map_size > self.config.struct_name_index_map_capacity {
            flush_all_caches = true;
        }
        if self.module_cache.size() > self.config.module_cache_capacity {
            flush_all_caches = true;
        }

        // Finally, if flag is set, flush the caches.
        if flush_all_caches {
            runtime_environment.flush_struct_name_and_info_caches();
            self.module_cache.flush_unchecked();
        }

        Ok(())
    }

    /// Should always be called after block execution. Sets the [GlobalCacheManagerInner] to be
    /// execution-ready (and if it is already execution-ready, returns an error). Sets the ID for
    /// the executed block so that the next execution can check it.
    pub fn mark_block_execution_end(
        &self,
        executed_block_id: Option<HashValue>,
    ) -> Result<(), VMStatus> {
        // We are done executing a block, reset the previous block id. Do everything under lock to
        // ensure it is not possible to execute blocks concurrently.
        let mut previous_block_id = self.previous_block_id.lock();
        if self.ready_for_next_block.load(Ordering::SeqCst) {
            let msg = "Should not be possible to mark block execution end for execution-ready \
                             global cache, check if blocks are executed concurrently";
            return Err(invariant_violation(msg));
        }
        *previous_block_id = BlockId::Set(executed_block_id);

        // Set the flag that the global cache is ready for next execution.
        self.ready_for_next_block.store(true, Ordering::SeqCst);

        Ok(())
    }

    /// Returns the cached environment that [GlobalCacheManagerInner::mark_block_execution_start]
    /// must set. If it has not been set, an invariant violation error is returned.
    pub fn environment(&self) -> Result<AptosEnvironment, VMStatus> {
        self.previous_environment.lock().clone().ok_or_else(|| {
            invariant_violation("Environment must always be set at block execution start")
        })
    }

    /// Returns the global module cache.
    pub fn module_cache(&self) -> Arc<ReadOnlyModuleCache<K, DC, VC, E>> {
        self.module_cache.clone()
    }
}

/// Same as [GlobalCacheManagerInner], but uses concrete types used by execution on Aptos instead
/// of generics. Allows us not to propagate generic type parameters everywhere (for now), but be
/// able to mock and test.
pub struct GlobalCacheManager {
    inner: GlobalCacheManagerInner<ModuleId, CompiledModule, Module, AptosModuleExtension>,
}

impl GlobalCacheManager {
    /// Returns a new instance of [GlobalCacheManager] with default [GlobalCacheConfig].
    pub fn new_with_default_config() -> Self {
        Self {
            inner: GlobalCacheManagerInner::new_with_default_config(),
        }
    }
}

impl Deref for GlobalCacheManager {
    type Target = GlobalCacheManagerInner<ModuleId, CompiledModule, Module, AptosModuleExtension>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(test)]
mod test {
    // use super::*;
    // use aptos_language_e2e_tests::data_store::FakeDataStore;
    // use aptos_types::on_chain_config::{FeatureFlag, Features};
    // use aptos_vm_environment::environment::AptosEnvironment;
    // use claims::assert_ok;
    // use move_vm_types::code::mock_verified_code;
    //
    // #[test]
    // fn test_cross_block_module_cache_flush() {
    //     let global_module_cache = ReadOnlyModuleCache::empty();
    //
    //     global_module_cache.insert(0, mock_verified_code(0, None));
    //     assert_eq!(global_module_cache.size(), 1);
    //
    //     global_module_cache.flush_unchecked();
    //     assert_eq!(global_module_cache.size(), 0);
    //
    //     // Now check that cache is flushed when the environment is flushed.
    //     let mut state_view = FakeDataStore::default();
    //     let env_old = AptosEnvironment::new_with_delayed_field_optimization_enabled(&state_view);
    //
    //     for i in 0..10 {
    //         global_module_cache.insert(i, mock_verified_code(i, None));
    //     }
    //     assert_eq!(global_module_cache.size(), 10);
    //
    //     let mut features = Features::default();
    //     features.disable(FeatureFlag::KEYLESS_ACCOUNTS);
    //     state_view.set_features(features);
    //
    //     // New environment means we need to also flush global caches - to invalidate struct name
    //     // indices.
    //     let env_new = assert_ok!(get_environment_with_delayed_field_optimization_enabled(
    //         &state_view,
    //         &global_module_cache,
    //     ));
    //     assert!(env_old != env_new);
    //     assert_eq!(global_module_cache.size(), 0);
    // }
}
