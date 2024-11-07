// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::config::GlobalCacheConfig;
use aptos_crypto::HashValue;
use aptos_types::{
    read_only_module_cache::ReadOnlyModuleCache, state_store::StateView,
    vm::modules::AptosModuleExtension,
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_types::module_and_script_storage::AsAptosCodeStorage;
use move_binary_format::CompiledModule;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    language_storage::ModuleId,
    vm_status::{StatusCode, VMStatus},
};
use move_vm_runtime::{Module, ModuleStorage, WithRuntimeEnvironment};
use move_vm_types::code::WithSize;
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
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, PartialOrd, Ord)]
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
struct GlobalCacheManagerInner<K, DC, VC, E> {
    config: GlobalCacheConfig,

    /// Cache for modules. It is read-only for any concurrent execution, and can only be mutated
    /// when it is known that there are no concurrent accesses, e.g., at block boundaries.
    /// [GlobalCacheManagerInner] must ensure that these invariants always hold.
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
    E: WithSize,
{
    /// Returns a new instance of [GlobalCacheManagerInner] with default [GlobalCacheConfig].
    fn new_with_default_config() -> Self {
        Self::new_with_config(GlobalCacheConfig::default())
    }

    /// Returns a new instance of [GlobalCacheManagerInner] with the provided [GlobalCacheConfig].
    fn new_with_config(config: GlobalCacheConfig) -> Self {
        Self {
            config,
            module_cache: Arc::new(ReadOnlyModuleCache::empty()),
            previous_block_id: Mutex::new(BlockId::Unset),
            previous_environment: Mutex::new(None),
            ready_for_next_block: AtomicBool::new(true),
        }
    }

    /// See the documentation for [GlobalCacheManager::mark_block_execution_start]. The only
    /// difference here is that there is no framework prefetching.
    fn mark_block_execution_start(
        &self,
        state_view: &impl StateView,
        previous_block_id: Option<HashValue>,
    ) -> Result<(), VMStatus> {
        let recorded_previous_block_id = {
            // Acquire a lock, and check if we are ready to execute the next block.
            let previous_block_id = self.previous_block_id.lock();
            if !self.ready_for_next_block() {
                let msg = "Trying to execute blocks concurrently over shared global state";
                return Err(invariant_violation(msg));
            }

            // Prepare for execution. Set the flag as not ready to ensure that blocks are not
            // executed concurrently using the same cache.
            self.mark_not_ready_for_next_block();
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
        let struct_name_index_map_size = match runtime_environment.struct_name_index_map_size() {
            Err(err) => {
                // Unlock the cache, reset all states, and return.
                drop(previous_environment);
                let err = self.reset_and_return_invariant_violation(&format!(
                    "Error when getting struct name index map size: {:?}",
                    err
                ));
                return Err(err);
            },
            Ok(size) => size,
        };

        if struct_name_index_map_size > self.config.max_struct_name_index_map_size {
            flush_all_caches = true;
        }
        if self.module_cache.size_in_bytes() > self.config.max_module_cache_size_in_bytes {
            // Technically, if we flush modules we do not need to flush type caches, but we unify
            // flushing logic for easier reasoning.
            flush_all_caches = true;
        }

        // Finally, if flag is set, flush the caches.
        if flush_all_caches {
            runtime_environment.flush_struct_name_and_info_caches();
            self.module_cache.flush_unchecked();
        }

        Ok(())
    }

    /// See the documentation for [GlobalCacheManager::mark_block_execution_end].
    fn mark_block_execution_end(
        &self,
        executed_block_id: Option<HashValue>,
    ) -> Result<(), VMStatus> {
        // We are done executing a block, reset the previous block id. Do everything under lock to
        // ensure it is not possible to execute blocks concurrently.
        let mut previous_block_id = self.previous_block_id.lock();
        if self.ready_for_next_block() {
            // This means we are executing concurrently. If so, all-but-one thread will return an
            // error. Note that the caches are still consistent for that one thread.
            let msg = "Should not be possible to mark block execution end for execution-ready \
                             global cache, check if blocks are executed concurrently";
            return Err(invariant_violation(msg));
        }
        *previous_block_id = BlockId::Set(executed_block_id);

        // Set the flag that the global cache is ready for next execution.
        self.mark_ready_for_next_block();

        Ok(())
    }

    /// Returns true of a next block is ready be executed. This is the case only when:
    ///   1. the global caches have just been created, or
    ///   2. [GlobalCacheManagerInner::mark_block_execution_end] was called indicating that
    ///      previous block execution has finished.
    fn ready_for_next_block(&self) -> bool {
        self.ready_for_next_block.load(Ordering::SeqCst)
    }

    /// Marks caches as ready for next block execution.
    fn mark_ready_for_next_block(&self) {
        self.ready_for_next_block.store(true, Ordering::SeqCst);
    }

    /// Marks caches as not ready for next block execution.
    fn mark_not_ready_for_next_block(&self) {
        self.ready_for_next_block.store(false, Ordering::SeqCst);
    }

    /// Resets all states (under a lock) as if global caches are empty and no blocks have been
    /// executed so far. Returns an invariant violation error.
    fn reset_and_return_invariant_violation(&self, msg: &str) -> VMStatus {
        // Lock to reset the state under lock.
        let mut previous_block_id = self.previous_block_id.lock();

        // 1. Should be ready for next execution.
        self.mark_not_ready_for_next_block();
        // 2. Should contain no environment.
        *self.previous_environment.lock() = None;
        // 3. Module cache is empty.
        self.module_cache.flush_unchecked();
        // 4. Block ID is unset.
        *previous_block_id = BlockId::Unset;

        // State reset, unlock.
        drop(previous_block_id);

        invariant_violation(msg)
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

    /// Sets the state of global caches prior to block execution on top of the provided state (with
    /// the block ID). Should always sbe called prior to block execution.
    ///
    /// The caches stored globally (modules, struct name re-indexing map and type caches) are all
    /// flushed if:
    ///   1. Previously executed block ID does not match the provided value.
    ///   2. The environment has changed for this state.
    ///   3. The size of the struct name re-indexing map is too large.
    ///   4. The size (in bytes) of the module cache is too large.
    ///
    /// Additionally, if cache is empty, prefetches the framework code into it.
    ///
    /// Marks [GlobalCacheManager] as not ready for next block execution. If called concurrently,
    /// only a single invocation ever succeeds and other calls return an error.
    pub fn mark_block_execution_start(
        &self,
        state_view: &impl StateView,
        previous_block_id: Option<HashValue>,
    ) -> Result<(), VMStatus> {
        self.inner
            .mark_block_execution_start(state_view, previous_block_id)?;

        if self.inner.config.prefetch_framework_code && self.module_cache().num_modules() == 0 {
            let code_storage = state_view.as_aptos_code_storage(self.environment()?);

            // If framework code exists in storage, the transitive closure will be verified and
            // cached.
            let result = code_storage
                .fetch_verified_module(&AccountAddress::ONE, ident_str!("transaction_validation"));

            match result {
                Ok(Some(_)) => {
                    // Framework must have been loaded. Drain verified modules from local cache
                    // into global cache.
                    let verified_module_code_iter = code_storage
                        .into_verified_module_code_iter()
                        .map_err(|err| {
                        let msg = format!(
                            "Unable to convert cached modules into verified code: {:?}",
                            err
                        );
                        self.inner.reset_and_return_invariant_violation(&msg)
                    })?;
                    self.inner
                        .module_cache
                        .insert_verified_unchecked(verified_module_code_iter)
                        .map_err(|err| {
                            let msg = format!("Unable to cache verified framework: {:?}", err);
                            self.inner.reset_and_return_invariant_violation(&msg)
                        })?;
                },
                Ok(None) => {
                    // No framework in the state, do nothing.
                },
                Err(err) => {
                    // There should be no errors when pre-fetching the framework, if there are, we
                    // better return an error here.
                    let msg = format!("Error when pre-fetching the framework: {:?}", err);
                    return Err(self.inner.reset_and_return_invariant_violation(&msg));
                },
            }
        }
        Ok(())
    }

    /// Should always be called after block execution. Sets the [GlobalCacheManager] to be ready
    /// for execution (and if it is already execution-ready, returns an error). Sets the ID for the
    /// executed block so that the next execution can check it.
    pub fn mark_block_execution_end(
        &self,
        executed_block_id: Option<HashValue>,
    ) -> Result<(), VMStatus> {
        self.inner.mark_block_execution_end(executed_block_id)
    }

    /// Returns the cached environment set by [GlobalCacheManager::mark_block_execution_start]. If
    /// it has not been set, an invariant violation error is returned.
    pub fn environment(&self) -> Result<AptosEnvironment, VMStatus> {
        self.inner
            .previous_environment
            .lock()
            .clone()
            .ok_or_else(|| {
                // Note: we do not expect this to happen (this is really more of an unreachable).
                invariant_violation("Environment must always be set at block execution start")
            })
    }

    /// Returns the global module cache.
    pub fn module_cache(
        &self,
    ) -> Arc<ReadOnlyModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension>> {
        self.inner.module_cache.clone()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_types::{
        on_chain_config::{FeatureFlag, Features, OnChainConfig},
        state_store::{state_key::StateKey, state_value::StateValue, MockStateView},
    };
    use claims::{assert_err, assert_ok};
    use move_vm_types::code::{
        mock_verified_code, MockDeserializedCode, MockExtension, MockVerifiedCode,
    };
    use std::{collections::HashMap, thread, thread::JoinHandle};
    use test_case::test_case;

    /// Joins threads. Succeeds only if a single handle evaluates to [Ok] and the rest are [Err]s.
    fn join_and_assert_single_ok(handles: Vec<JoinHandle<Result<(), VMStatus>>>) {
        let mut num_oks = 0;
        let mut num_errs = 0;

        let num_handles = handles.len();
        for handle in handles {
            let result = handle.join().unwrap();
            if result.is_ok() {
                num_oks += 1;
            } else {
                num_errs += 1;
            }
        }
        assert_eq!(num_oks, 1);
        assert_eq!(num_errs, num_handles - 1);
    }

    #[test]
    fn environment_should_always_be_set() {
        let global_cache_manager = GlobalCacheManager::new_with_default_config();
        assert!(global_cache_manager.environment().is_err());

        let state_view = MockStateView::empty();
        assert_ok!(global_cache_manager.mark_block_execution_start(&state_view, None));
        assert_ok!(global_cache_manager.environment());
    }

    #[test]
    fn mark_ready() {
        let global_cache_manager = GlobalCacheManagerInner::<
            u32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new_with_default_config();
        assert!(global_cache_manager.ready_for_next_block());

        global_cache_manager.mark_not_ready_for_next_block();
        assert!(!global_cache_manager.ready_for_next_block());

        global_cache_manager.mark_ready_for_next_block();
        assert!(global_cache_manager.ready_for_next_block());
    }

    #[test]
    fn mark_execution_start_when_different_environment() {
        let state_view = MockStateView::empty();
        let global_cache_manager = GlobalCacheManagerInner::new_with_default_config();

        global_cache_manager
            .module_cache
            .insert(0, mock_verified_code(0, MockExtension::new(8)));
        global_cache_manager
            .module_cache
            .insert(1, mock_verified_code(1, MockExtension::new(8)));
        assert_eq!(global_cache_manager.module_cache.num_modules(), 2);

        assert_ok!(global_cache_manager.mark_block_execution_start(&state_view, None));
        let old_environment = global_cache_manager
            .previous_environment
            .lock()
            .clone()
            .unwrap();
        assert_ok!(global_cache_manager.mark_block_execution_end(Some(HashValue::zero())));
        assert_eq!(global_cache_manager.module_cache.num_modules(), 2);

        // Tweak feature flags to force a different config.
        let mut features = old_environment.features().clone();
        assert!(features.is_enabled(FeatureFlag::LIMIT_VM_TYPE_SIZE));
        features.disable(FeatureFlag::LIMIT_VM_TYPE_SIZE);
        let bytes = bcs::to_bytes(&features).unwrap();
        let state_key = StateKey::resource(Features::address(), &Features::struct_tag()).unwrap();

        let state_view = MockStateView::new(HashMap::from([(
            state_key,
            StateValue::new_legacy(bytes.into()),
        )]));

        // We use the same previous ID, but the cache is still flushed: the environment changed.
        assert_ok!(
            global_cache_manager.mark_block_execution_start(&state_view, Some(HashValue::zero()))
        );
        assert_eq!(global_cache_manager.module_cache.num_modules(), 0);

        let new_environment = global_cache_manager
            .previous_environment
            .lock()
            .clone()
            .unwrap();
        assert!(old_environment != new_environment);
    }

    #[test]
    fn mark_execution_start_when_too_many_types() {
        // TODO(loader_v2):
        //   Propagate type caches/struct name index map APIs to here so we can mock & test.
    }

    #[test]
    fn mark_execution_start_when_module_cache_is_too_large() {
        let state_view = MockStateView::empty();

        let config = GlobalCacheConfig {
            max_module_cache_size_in_bytes: 8,
            ..Default::default()
        };
        let global_cache_manager = GlobalCacheManagerInner::new_with_config(config);

        global_cache_manager
            .module_cache
            .insert(0, mock_verified_code(0, MockExtension::new(8)));
        global_cache_manager
            .module_cache
            .insert(1, mock_verified_code(1, MockExtension::new(24)));
        assert_eq!(global_cache_manager.module_cache.num_modules(), 2);
        assert_eq!(global_cache_manager.module_cache.size_in_bytes(), 32);

        // Cache is too large, should be flushed for next block.
        assert_ok!(
            global_cache_manager.mark_block_execution_start(&state_view, Some(HashValue::random()))
        );
        assert_eq!(global_cache_manager.module_cache.num_modules(), 0);
        assert_eq!(global_cache_manager.module_cache.size_in_bytes(), 0);
    }

    #[test_case(None)]
    #[test_case(Some(HashValue::zero()))]
    fn mark_execution_start_when_unset(previous_block_id: Option<HashValue>) {
        let state_view = MockStateView::empty();
        let global_cache_manager = GlobalCacheManagerInner::new_with_default_config();

        global_cache_manager
            .module_cache
            .insert(0, mock_verified_code(0, MockExtension::new(8)));
        assert_eq!(global_cache_manager.module_cache.num_modules(), 1);

        // If executed on top of unset state, or the state with matching previous hash, the cache
        // is not flushed.
        assert_ok!(global_cache_manager.mark_block_execution_start(&state_view, previous_block_id));
        assert_eq!(global_cache_manager.module_cache.num_modules(), 1);
        assert!(!global_cache_manager.ready_for_next_block());
    }

    #[test_case(None, None)]
    #[test_case(None, Some(HashValue::zero()))]
    #[test_case(Some(HashValue::zero()), None)]
    #[test_case(Some(HashValue::zero()), Some(HashValue::zero()))]
    #[test_case(Some(HashValue::from_u64(0)), Some(HashValue::from_u64(1)))]
    fn mark_execution_start_when_set(
        recorded_previous_block_id: Option<HashValue>,
        previous_block_id: Option<HashValue>,
    ) {
        let state_view = MockStateView::empty();
        let global_cache_manager = GlobalCacheManagerInner::new_with_default_config();

        assert_ok!(
            global_cache_manager.mark_block_execution_start(&state_view, Some(HashValue::random()))
        );
        assert_ok!(global_cache_manager.mark_block_execution_end(recorded_previous_block_id));

        global_cache_manager
            .module_cache
            .insert(0, mock_verified_code(0, MockExtension::new(8)));
        assert_eq!(global_cache_manager.module_cache.num_modules(), 1);

        assert_ok!(global_cache_manager.mark_block_execution_start(&state_view, previous_block_id));
        assert!(!global_cache_manager.ready_for_next_block());

        if recorded_previous_block_id.is_some() && recorded_previous_block_id == previous_block_id {
            // In this case both IDs match, no cache flushing.
            assert_eq!(global_cache_manager.module_cache.num_modules(), 1);
        } else {
            // If previous block IDs do not match, or are unknown, caches must be flushed!
            assert_eq!(global_cache_manager.module_cache.num_modules(), 0);
        }
    }

    #[test]
    fn mark_execution_start_concurrent() {
        let state_view = Box::new(MockStateView::empty());
        let state_view: &'static _ = Box::leak(state_view);

        let global_cache_manager = Arc::new(GlobalCacheManagerInner::<
            u32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new_with_default_config());
        assert!(global_cache_manager.ready_for_next_block());

        let mut handles = vec![];
        for _ in 0..32 {
            let handle = thread::spawn({
                let global_cache_manager = global_cache_manager.clone();
                move || global_cache_manager.mark_block_execution_start(state_view, None)
            });
            handles.push(handle);
        }
        join_and_assert_single_ok(handles);
    }

    #[test_case(None)]
    #[test_case(Some(HashValue::from_u64(0)))]
    fn mark_block_execution_end(block_id: Option<HashValue>) {
        let global_cache_manager = GlobalCacheManagerInner::<
            u32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new_with_default_config();
        assert!(global_cache_manager.previous_block_id.lock().is_unset());

        // The global cache is ready, so we cannot mark execution end.
        assert_err!(global_cache_manager.mark_block_execution_end(block_id));

        global_cache_manager.mark_not_ready_for_next_block();
        let previous_block_id = *global_cache_manager.previous_block_id.lock();
        assert!(previous_block_id.is_unset());
        assert_ok!(global_cache_manager.mark_block_execution_end(block_id));

        // The previous block ID should be set now, and the state is ready.
        let new_block_id = *global_cache_manager.previous_block_id.lock();
        assert_eq!(new_block_id, BlockId::Set(block_id));
        assert!(global_cache_manager.ready_for_next_block());

        global_cache_manager.mark_not_ready_for_next_block();
        let next_block_id = Some(HashValue::from_u64(1));
        assert_ok!(global_cache_manager.mark_block_execution_end(next_block_id));

        // Previous block ID is again reset.
        let new_block_id = *global_cache_manager.previous_block_id.lock();
        assert_eq!(new_block_id, BlockId::Set(next_block_id));
    }

    #[test]
    fn mark_block_execution_end_concurrent() {
        let global_cache_manager = Arc::new(GlobalCacheManagerInner::<
            u32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new_with_default_config());
        global_cache_manager.mark_not_ready_for_next_block();

        let mut handles = vec![];
        for _ in 0..32 {
            let handle = thread::spawn({
                let global_cache_manager = global_cache_manager.clone();
                move || global_cache_manager.mark_block_execution_end(None)
            });
            handles.push(handle);
        }
        join_and_assert_single_ok(handles);
    }
}
