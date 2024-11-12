// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{code_cache_global::GlobalModuleCache, explicit_sync_wrapper::ExplicitSyncWrapper};
use aptos_types::block_executor::config::BlockExecutorModuleCacheLocalConfig;
use aptos_vm_environment::environment::AptosEnvironment;
use move_binary_format::errors::Location;
use move_core_types::vm_status::{StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR, VMStatus};
use move_vm_runtime::WithRuntimeEnvironment;
use move_vm_types::code::WithSize;
use parking_lot::Mutex;
use std::{
    fmt::Debug,
    hash::Hash,
    mem,
    ops::{Deref, DerefMut},
    sync::Arc,
};

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

/// Represents the state of [GlobalModuleCache]. The following transitions are allowed:
///   2. [State::Ready] --> [State::Executing].
///   3. [State::Executing] --> [State::Done].
///   4. [State::Done] --> [State::Ready].
/// The optional value stored in variants is propagated during state transitions. When a full cycle
/// is reached (just before [State::Done] to [State::Ready] transition), the user can check if the
/// value is expected and continue with a new one. For instance:
/// ```text
/// Ready(Some(0)) --> Executing(Some(0)) --> Done(Some(0)) --> Ready(Some(1)) is allowed.
/// Ready(Some(0)) --> Executing(Some(0)) --> Done(Some(0)) --> Ready(Some(2)) is not allowed.
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
enum State<T> {
    Ready(Option<T>),
    Executing(Option<T>),
    Done(Option<T>),
}

/// Manages module caches and the execution environment, possible across multiple blocks.
pub struct ModuleCacheManager<T, K, DC, VC, E> {
    /// The state of global caches.
    state: Mutex<State<T>>,

    /// During concurrent executions, this module cache is read-only. However, it can be mutated
    /// when it is known that there are no concurrent accesses. [ModuleCacheManager] must ensure
    /// the safety.
    module_cache: Arc<GlobalModuleCache<K, DC, VC, E>>,
    /// The execution environment, initially set to [None]. The environment, as long as it does not
    /// change, can be kept for multiple block executions.
    environment: ExplicitSyncWrapper<Option<AptosEnvironment>>,
}

impl<T, K, DC, VC, E> ModuleCacheManager<T, K, DC, VC, E>
where
    T: Debug + Eq,
    K: Hash + Eq + Clone,
    VC: Deref<Target = Arc<DC>>,
    E: WithSize,
{
    /// Returns a new instance of [ModuleCacheManager] in a [State::Done] state with uninitialized
    /// current value.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State::Done(None)),
            module_cache: Arc::new(GlobalModuleCache::empty()),
            environment: ExplicitSyncWrapper::new(None),
        }
    }

    /// If state is [State::Done], sets the state to [State::Ready] with the current value and
    /// returns true. Otherwise, raises an alert and returns false. Additionally, synchronizes
    /// module and environment caches based on the provided previous value.
    pub fn mark_ready(&self, previous: Option<&T>, current: Option<T>) -> bool {
        let mut state = self.state.lock();

        if let State::Done(recorded_previous) = state.deref() {
            // If the state is done, but the values do not exist or do not match, we flush global
            // caches because they execute on top of unknown state (or on top of some different to
            // the previous state).
            if !recorded_previous
                .as_ref()
                .is_some_and(|r| previous.is_some_and(|p| r == p))
            {
                if let Some(environment) = self.environment.acquire().as_ref() {
                    environment
                        .runtime_environment()
                        .flush_struct_name_and_info_caches();
                }
                self.module_cache.flush_unsync();
            }

            *state = State::Ready(current);
            true
        } else {
            // We are not in the done state, this is an error.
            alert_or_println!(
                "Unable to mark ready, state: {:?}, previous: {:?}, current: {:?}",
                state,
                previous,
                current
            );
            false
        }
    }

    /// When in [State::Ready], runs different checks on cached modules and environment:
    ///   1. If the environment is not initialized, or is different from the one in storage, it is
    ///      re-initialized, and module caches are flushed.
    ///   2. If too many struct names have been cached in re-indexing map in runtime environment,
    ///      struct type caches and module caches are flushed.
    ///   3. If module cache size is too large (in bytes), it is flushed.
    /// The final environment and module caches are returned.
    pub fn check_ready_and_get_caches(
        &self,
        storage_environment: AptosEnvironment,
        config: &BlockExecutorModuleCacheLocalConfig,
    ) -> Result<(AptosEnvironment, Arc<GlobalModuleCache<K, DC, VC, E>>), VMStatus> {
        let state = self.state.lock();
        if !matches!(state.deref(), State::Ready(_)) {
            let msg = format!(
                "Expected ready state to check caches, got {:?}",
                state.deref()
            );
            return Err(VMStatus::error(
                UNKNOWN_INVARIANT_VIOLATION_ERROR,
                Some(msg),
            ));
        }

        let environment = self.get_or_initialize_environment(storage_environment);
        let module_cache = self.module_cache.clone();

        // Check 1: struct re-indexing map is not too large. If it is, we flush the cache. Also, we
        // need to flush modules because they store indices into re-indexing map.
        let runtime_environment = environment.runtime_environment();
        let struct_name_index_map_size = runtime_environment
            .struct_name_index_map_size()
            .map_err(|err| err.finish(Location::Undefined).into_vm_status())?;
        if struct_name_index_map_size > config.max_struct_name_index_map_num_entries {
            module_cache.flush_unsync();
            runtime_environment.flush_struct_name_and_info_caches();
        }

        // Check 2: If the module cache is too big, flush it.
        if module_cache.size_in_bytes() > config.max_module_cache_size_in_bytes {
            module_cache.flush_unsync();
        }

        Ok((environment, module_cache))
    }

    /// If state is [State::Ready], changes it to [State::Executing] with the same value, returning
    /// true. Otherwise, returns false indicating that state transition failed, also raising an
    /// alert.
    pub fn mark_executing(&self) -> bool {
        let mut state = self.state.lock();
        if let State::Ready(v) = state.deref_mut() {
            *state = State::Executing(mem::take(v));
            true
        } else {
            alert_or_println!("Unable to mark executing, state: {:?}", state);
            false
        }
    }

    /// If state is [State::Executing], changes it to [State::Done] with the same value, returning
    /// true. Otherwise, returns false indicating that state transition failed, also raising an
    /// alert.
    pub fn mark_done(&self) -> bool {
        let mut state = self.state.lock();
        if let State::Executing(v) = state.deref_mut() {
            *state = State::Done(mem::take(v));
            true
        } else {
            alert_or_println!("Unable to mark done, state: {:?}", state);
            false
        }
    }

    /// Returns the cached global environment if it already exists, and matches the one in storage.
    /// If it does not exist, or does not match, the new environment is initialized from the given
    /// state, cached, and returned. Should be called when in [State::Ready] state, under lock.
    fn get_or_initialize_environment(
        &self,
        storage_environment: AptosEnvironment,
    ) -> AptosEnvironment {
        let mut guard = self.environment.acquire();
        let existing_environment = guard.deref_mut();

        let environment_requires_update = existing_environment
            .as_ref()
            .map_or(true, |environment| environment != &storage_environment);
        if environment_requires_update {
            *existing_environment = Some(storage_environment);

            // If this environment has been (re-)initialized, we need to flush the module cache
            // because it can contain now out-dated code.
            self.module_cache.flush_unsync();
        }

        existing_environment
            .clone()
            .expect("Environment must be set")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_types::{
        on_chain_config::{FeatureFlag, Features, OnChainConfig},
        state_store::{state_key::StateKey, state_value::StateValue, MockStateView},
    };
    use claims::assert_ok;
    use move_core_types::{
        account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
    };
    use move_vm_types::{
        code::{mock_verified_code, MockDeserializedCode, MockExtension, MockVerifiedCode},
        loaded_data::runtime_types::StructIdentifier,
    };
    use std::{collections::HashMap, thread, thread::JoinHandle};
    use test_case::test_case;

    #[test_case(None, None)]
    #[test_case(None, Some(1))]
    #[test_case(Some(0), None)]
    #[test_case(Some(0), Some(1))]
    #[test_case(Some(0), Some(0))]
    fn test_mark_ready(recorded_previous: Option<i32>, previous: Option<i32>) {
        let module_cache_manager = ModuleCacheManager::new();
        *module_cache_manager.state.lock() = State::Done(recorded_previous);

        // Pre-populate module cache to test flushing.
        module_cache_manager
            .module_cache
            .insert(0, mock_verified_code(0, MockExtension::new(8)));
        assert_eq!(module_cache_manager.module_cache.num_modules(), 1);

        assert!(!module_cache_manager.mark_executing());
        assert!(!module_cache_manager.mark_done());
        assert!(module_cache_manager.mark_ready(previous.as_ref(), Some(77)));

        // Only in matching case the module cache is not flushed.
        if recorded_previous.is_some() && recorded_previous == previous {
            assert_eq!(module_cache_manager.module_cache.num_modules(), 1);
        } else {
            assert_eq!(module_cache_manager.module_cache.num_modules(), 0);
        }

        let state = module_cache_manager.state.lock().clone();
        assert_eq!(state, State::Ready(Some(77)))
    }

    #[test]
    fn test_check_ready() {
        let state_view = MockStateView::empty();
        let config = BlockExecutorModuleCacheLocalConfig {
            prefetch_framework_code: false,
            max_module_cache_size_in_bytes: 8,
            max_struct_name_index_map_num_entries: 2,
        };

        let module_cache_manager = ModuleCacheManager::<
            i32,
            i32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new();

        // Set up the state and the environment.
        *module_cache_manager.state.lock() = State::Ready(None);
        let environment = module_cache_manager.get_or_initialize_environment(
            AptosEnvironment::new_with_delayed_field_optimization_enabled(&state_view),
        );

        module_cache_manager
            .module_cache
            .insert(0, mock_verified_code(0, MockExtension::new(16)));
        assert_eq!(module_cache_manager.module_cache.num_modules(), 1);

        let runtime_environment = environment.runtime_environment();
        let dummy_struct_name = StructIdentifier {
            module: ModuleId::new(AccountAddress::ONE, Identifier::new("foo").unwrap()),
            name: Identifier::new("Bar").unwrap(),
        };
        assert!(runtime_environment
            .struct_name_to_idx_for_test(dummy_struct_name)
            .is_ok());
        assert_eq!(
            assert_ok!(runtime_environment.struct_name_index_map_size()),
            1
        );

        // Module cache size in bytes is too large, should be flushed (but not struct types).
        assert!(module_cache_manager
            .check_ready_and_get_caches(environment.clone(), &config)
            .is_ok());
        assert_eq!(module_cache_manager.module_cache.num_modules(), 0);
        assert_eq!(
            assert_ok!(runtime_environment.struct_name_index_map_size()),
            1
        );

        module_cache_manager
            .module_cache
            .insert(0, mock_verified_code(0, MockExtension::new(4)));

        // This time size is less than the one specified in config. No flushing.
        assert!(module_cache_manager
            .check_ready_and_get_caches(environment.clone(), &config)
            .is_ok());
        assert_eq!(module_cache_manager.module_cache.num_modules(), 1);
        assert_eq!(
            assert_ok!(runtime_environment.struct_name_index_map_size()),
            1
        );

        let dummy_struct_names = [
            StructIdentifier {
                module: ModuleId::new(AccountAddress::ONE, Identifier::new("foo").unwrap()),
                name: Identifier::new("Foo").unwrap(),
            },
            StructIdentifier {
                module: ModuleId::new(AccountAddress::ONE, Identifier::new("foo").unwrap()),
                name: Identifier::new("Baz").unwrap(),
            },
        ];
        for dummy_struct_name in dummy_struct_names {
            assert!(runtime_environment
                .struct_name_to_idx_for_test(dummy_struct_name)
                .is_ok());
        }
        assert_eq!(
            assert_ok!(runtime_environment.struct_name_index_map_size()),
            3
        );

        // Too many struct names cached.
        assert!(module_cache_manager
            .check_ready_and_get_caches(environment.clone(), &config)
            .is_ok());
        assert_eq!(module_cache_manager.module_cache.num_modules(), 0);
        assert_eq!(
            assert_ok!(runtime_environment.struct_name_index_map_size()),
            0
        );
    }

    #[test]
    fn test_mark_executing() {
        let module_cache_manager = ModuleCacheManager::<
            i32,
            i32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new();
        *module_cache_manager.state.lock() = State::Ready(Some(100));

        assert!(!module_cache_manager.mark_ready(Some(&76), Some(77)));
        assert!(!module_cache_manager.mark_done());

        assert!(module_cache_manager.mark_executing());

        let state = module_cache_manager.state.lock().clone();
        assert_eq!(state, State::Executing(Some(100)))
    }

    #[test]
    fn test_mark_done() {
        let module_cache_manager = ModuleCacheManager::<
            i32,
            i32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new();
        *module_cache_manager.state.lock() = State::Executing(Some(100));

        assert!(!module_cache_manager.mark_ready(Some(&76), Some(77)));
        assert!(!module_cache_manager.mark_executing());

        assert!(module_cache_manager.mark_done());

        let state = module_cache_manager.state.lock().clone();
        assert_eq!(state, State::Done(Some(100)))
    }

    /// Joins threads. Succeeds only if a single handle evaluates to [Ok] and the rest are [Err]s.
    fn join_and_assert_single_true(handles: Vec<JoinHandle<bool>>) {
        let mut num_true = 0;
        let mut num_false = 0;

        let num_handles = handles.len();
        for handle in handles {
            if handle.join().unwrap() {
                num_true += 1;
            } else {
                num_false += 1;
            }
        }
        assert_eq!(num_true, 1);
        assert_eq!(num_false, num_handles - 1);
    }

    #[test]
    fn test_mark_ready_concurrent() {
        let global_cache_manager = Arc::new(ModuleCacheManager::<
            i32,
            i32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new());

        let mut handles = vec![];
        for _ in 0..32 {
            let handle = thread::spawn({
                let global_cache_manager = global_cache_manager.clone();
                move || global_cache_manager.mark_ready(Some(&1), Some(2))
            });
            handles.push(handle);
        }
        join_and_assert_single_true(handles);
    }

    #[test]
    fn test_mark_executing_concurrent() {
        let global_cache_manager = Arc::new(ModuleCacheManager::<
            i32,
            i32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new());
        assert!(global_cache_manager.mark_ready(Some(&0), Some(1)));

        let mut handles = vec![];
        for _ in 0..32 {
            let handle = thread::spawn({
                let global_cache_manager = global_cache_manager.clone();
                move || global_cache_manager.mark_executing()
            });
            handles.push(handle);
        }
        join_and_assert_single_true(handles);
    }

    #[test]
    fn test_mark_done_concurrent() {
        let global_cache_manager = Arc::new(ModuleCacheManager::<
            i32,
            i32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new());
        assert!(global_cache_manager.mark_ready(Some(&0), Some(1)));
        assert!(global_cache_manager.mark_executing());

        let mut handles = vec![];
        for _ in 0..32 {
            let handle = thread::spawn({
                let global_cache_manager = global_cache_manager.clone();
                move || global_cache_manager.mark_done()
            });
            handles.push(handle);
        }
        join_and_assert_single_true(handles);
    }

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
    fn test_get_or_initialize_environment() {
        let module_cache_manager = ModuleCacheManager::<
            i32,
            i32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new();
        *module_cache_manager.state.lock() = State::Ready(None);

        module_cache_manager
            .module_cache
            .insert(0, mock_verified_code(0, MockExtension::new(8)));
        module_cache_manager
            .module_cache
            .insert(1, mock_verified_code(1, MockExtension::new(8)));
        assert_eq!(module_cache_manager.module_cache.num_modules(), 2);
        assert!(module_cache_manager.environment.acquire().is_none());

        // Environment has to be set to the same value, cache flushed.
        let state_view = state_view_with_changed_feature_flag(None);
        let environment = module_cache_manager.get_or_initialize_environment(
            AptosEnvironment::new_with_delayed_field_optimization_enabled(&state_view),
        );
        assert_eq!(module_cache_manager.module_cache.num_modules(), 0);
        assert!(module_cache_manager
            .environment
            .acquire()
            .as_ref()
            .is_some_and(|cached_environment| cached_environment == &environment));

        module_cache_manager
            .module_cache
            .insert(2, mock_verified_code(2, MockExtension::new(8)));
        assert_eq!(module_cache_manager.module_cache.num_modules(), 1);
        assert!(module_cache_manager.environment.acquire().is_some());

        // Environment has to be re-set to the new value, cache flushed.
        let state_view =
            state_view_with_changed_feature_flag(Some(FeatureFlag::CODE_DEPENDENCY_CHECK));
        let environment = module_cache_manager.get_or_initialize_environment(
            AptosEnvironment::new_with_delayed_field_optimization_enabled(&state_view),
        );
        assert_eq!(module_cache_manager.module_cache.num_modules(), 0);
        assert!(module_cache_manager
            .environment
            .acquire()
            .as_ref()
            .is_some_and(|cached_environment| cached_environment == &environment));

        module_cache_manager
            .module_cache
            .insert(3, mock_verified_code(3, MockExtension::new(8)));
        assert_eq!(module_cache_manager.module_cache.num_modules(), 1);
        assert!(module_cache_manager.environment.acquire().is_some());

        // Environment is kept, and module caches are not flushed.
        let new_environment = module_cache_manager.get_or_initialize_environment(
            AptosEnvironment::new_with_delayed_field_optimization_enabled(&state_view),
        );
        assert_eq!(module_cache_manager.module_cache.num_modules(), 1);
        assert!(environment == new_environment);
    }
}
