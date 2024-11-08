// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{code_cache_global::GlobalModuleCache, explicit_sync_wrapper::ExplicitSyncWrapper};
use aptos_types::state_store::StateView;
use aptos_vm_environment::environment::AptosEnvironment;
use move_vm_runtime::WithRuntimeEnvironment;
use move_vm_types::code::WithSize;
use parking_lot::Mutex;
use std::{
    fmt::Debug,
    hash::Hash,
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
///   1. [State::Clean] --> [State::Ready].
///   2. [State::Ready] --> [State::Executing].
///   3. [State::Executing] --> [State::Done].
///   4. [State::Done] --> [State::Ready].
#[derive(Clone, Debug, Eq, PartialEq)]
enum State<T> {
    Clean,
    Ready(T),
    Executing(T),
    Done(T),
}

impl<T: Clone + Debug + Eq> State<T> {
    /// If the state is [State::Clean] returns true, and false otherwise.
    fn is_clean(&self) -> bool {
        match self {
            State::Clean => true,
            State::Ready(_) | State::Executing(_) | State::Done(_) => false,
        }
    }

    /// If the state is [State::Done], returns true.
    fn is_done(&self) -> bool {
        match self {
            State::Done(_) => true,
            State::Clean | State::Ready(_) | State::Executing(_) => false,
        }
    }

    /// If the state is [State::Done] and its value equals the one provided, returns true. In other
    /// cases, returns false.
    fn is_done_with_value(&self, value: &T) -> bool {
        match self {
            State::Done(v) => v == value,
            State::Clean | State::Executing(_) | State::Ready(_) => false,
        }
    }

    /// If the state is [State::Ready], returns its value. Otherwise, returns [None].
    fn value_from_ready(&self) -> Option<T> {
        match self {
            State::Ready(v) => Some(v.clone()),
            State::Clean | State::Executing(_) | State::Done(_) => None,
        }
    }

    /// If the state is [State::Executing], returns its value. Otherwise, returns [None].
    fn value_from_executing(&self) -> Option<T> {
        match self {
            State::Executing(v) => Some(v.clone()),
            State::Clean | State::Ready(_) | State::Done(_) => None,
        }
    }

    /// Sets the current state to [State::Ready].
    fn set_ready(&mut self, value: T) {
        *self = Self::Ready(value);
    }

    /// Sets the current state to [State::Executing].
    fn set_executing(&mut self, value: T) {
        *self = Self::Executing(value);
    }

    /// Sets the current state to [State::Done].
    fn set_done(&mut self, value: T) {
        *self = Self::Done(value);
    }
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
    T: Clone + Debug + Eq,
    K: Hash + Eq + Clone,
    VC: Deref<Target = Arc<DC>>,
    E: WithSize,
{
    /// Returns a new instance of [ModuleCacheManager].
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State::Clean),
            module_cache: Arc::new(GlobalModuleCache::empty()),
            environment: ExplicitSyncWrapper::new(None),
        }
    }

    /// If state is [State::Clean], or [State::Ready] with matching previous value, sets the state
    /// to [State::Ready] with the current value and returns true. Otherwise, raises an alert and
    /// returns false.
    pub fn mark_ready(&self, previous: &T, current: T) -> bool {
        let mut state = self.state.lock();

        if state.is_clean() || state.is_done_with_value(previous) {
            state.set_ready(current);
            return true;
        }

        if state.is_done() {
            // If the state is done, but the values to not match, we still set the state as ready, but
            // also flush global caches because they execute not on top of the previous state.
            self.module_cache.flush_unchecked();
            if let Some(environment) = self.environment.acquire().as_ref() {
                environment
                    .runtime_environment()
                    .flush_struct_name_and_info_caches();
            }

            state.set_ready(current);
            return true;
        }

        alert_or_println!(
            "Unable to mark ready, state: {:?}, previous: {:?}, current: {:?}",
            state,
            previous,
            current
        );
        false
    }

    /// If state is [State::Ready], changes it to [State::Executing] with the same value, returning
    /// true. Otherwise, returns false indicating that state transition failed, also raising an
    /// alert.
    pub fn mark_executing(&self) -> bool {
        let mut state = self.state.lock();
        if let Some(value) = state.value_from_ready() {
            state.set_executing(value);
            return true;
        }

        alert_or_println!("Unable to mark executing, state: {:?}", state);
        false
    }

    /// If state is [State::Executing], changes it to [State::Done] with the same value, returning
    /// true. Otherwise, returns false indicating that state transition failed, also raising an
    /// alert.
    pub fn mark_done(&self) -> bool {
        let mut state = self.state.lock();
        if let Some(value) = state.value_from_executing() {
            state.set_done(value);
            return true;
        }

        alert_or_println!("Unable to mark done, state: {:?}", state);
        false
    }

    /// Returns the cached global environment if it already exists, and matches the one in storage.
    /// If it does not exist, or does not match, the new environment is initialized from the given
    /// state, cached, and returned.
    pub fn get_or_initialize_environment_unchecked(
        &self,
        state_view: &impl StateView,
    ) -> AptosEnvironment {
        let _lock = self.state.lock();

        let new_environment =
            AptosEnvironment::new_with_delayed_field_optimization_enabled(state_view);

        let mut guard = self.environment.acquire();
        let existing_environment = guard.deref_mut();

        let (environment, is_new) = match existing_environment.as_ref() {
            None => {
                *existing_environment = Some(new_environment.clone());
                (new_environment, true)
            },
            Some(environment) => {
                if environment == &new_environment {
                    (environment.clone(), false)
                } else {
                    *existing_environment = Some(new_environment.clone());
                    (new_environment, true)
                }
            },
        };

        // If this environment has been (re-)initialized, we need to flush the module cache because
        // it can contain now out-dated code.
        if is_new {
            self.module_cache.flush_unchecked();
        }

        environment
    }

    /// Returns the global module cache.
    pub fn module_cache(&self) -> Arc<GlobalModuleCache<K, DC, VC, E>> {
        self.module_cache.clone()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_types::{
        on_chain_config::{FeatureFlag, Features, OnChainConfig},
        state_store::{state_key::StateKey, state_value::StateValue, MockStateView},
    };
    use claims::assert_matches;
    use move_vm_types::code::{
        mock_verified_code, MockDeserializedCode, MockExtension, MockVerifiedCode,
    };
    use std::{collections::HashMap, thread, thread::JoinHandle};
    use test_case::test_case;

    #[test]
    fn test_clean_state() {
        let state = State::Clean;

        assert!(state.is_clean());
        assert!(!state.is_done());
        assert!(!state.is_done_with_value(&0));
        assert!(state.value_from_ready().is_none());
        assert!(state.value_from_executing().is_none());
    }

    #[test]
    fn test_ready_state() {
        let state = State::Ready(0);

        assert!(!state.is_clean());
        assert!(!state.is_done());
        assert!(!state.is_done_with_value(&0));
        assert_eq!(state.value_from_ready(), Some(0));
        assert!(state.value_from_executing().is_none());
    }

    #[test]
    fn test_executing_state() {
        let state = State::Executing(0);

        assert!(!state.is_clean());
        assert!(!state.is_done());
        assert!(!state.is_done_with_value(&0));
        assert!(state.value_from_ready().is_none());
        assert_eq!(state.value_from_executing(), Some(0));
    }

    #[test]
    fn test_done_state() {
        let state = State::Done(0);

        assert!(!state.is_clean());
        assert!(state.is_done());
        assert!(state.is_done_with_value(&0));
        assert!(!state.is_done_with_value(&10));
        assert!(state.value_from_ready().is_none());
        assert!(state.value_from_executing().is_none());
    }

    #[test]
    fn test_set_state() {
        let mut state = State::Clean;

        state.set_ready(0);
        assert_matches!(state, State::Ready(0));

        state.set_executing(10);
        assert_matches!(state, State::Executing(10));

        state.set_done(100);
        assert_matches!(state, State::Done(100));
    }

    #[test_case(true)]
    #[test_case(false)]
    fn test_marking(with_different_value_for_done: bool) {
        let module_cache_manager = ModuleCacheManager::new();
        assert!(module_cache_manager.state.lock().is_clean());

        // Pre-populate module cache to test flushing.
        module_cache_manager
            .module_cache
            .insert(0, mock_verified_code(0, MockExtension::new(8)));
        assert_eq!(module_cache_manager.module_cache.num_modules(), 1);

        // Can only go to ready state from clean state.
        assert!(!module_cache_manager.mark_executing());
        assert!(!module_cache_manager.mark_done());
        assert!(module_cache_manager.mark_ready(&0, 1));

        assert_matches!(module_cache_manager.state.lock().deref(), State::Ready(1));
        assert_eq!(module_cache_manager.module_cache.num_modules(), 1);

        // Can only go to executing state from ready state.
        assert!(!module_cache_manager.mark_done());
        assert!(!module_cache_manager.mark_ready(&0, 1));
        assert!(module_cache_manager.mark_executing());

        assert_matches!(
            module_cache_manager.state.lock().deref(),
            State::Executing(1)
        );
        assert_eq!(module_cache_manager.module_cache.num_modules(), 1);

        // Can only go to done state from executing state.
        assert!(!module_cache_manager.mark_executing());
        assert!(!module_cache_manager.mark_ready(&0, 1));
        assert!(module_cache_manager.mark_done());

        assert_matches!(module_cache_manager.state.lock().deref(), State::Done(1));
        assert_eq!(module_cache_manager.module_cache.num_modules(), 1);

        // Can only go to ready state from done state.
        assert!(!module_cache_manager.mark_executing());
        assert!(!module_cache_manager.mark_done());

        if with_different_value_for_done {
            // Does not match! Caches should be flushed, but state reset.
            assert!(module_cache_manager.mark_ready(&10, 11));
            assert_matches!(module_cache_manager.state.lock().deref(), State::Ready(11));
            assert_eq!(module_cache_manager.module_cache.num_modules(), 0);
        } else {
            assert!(module_cache_manager.mark_ready(&1, 2));
            assert_matches!(module_cache_manager.state.lock().deref(), State::Ready(2));
            assert_eq!(module_cache_manager.module_cache.num_modules(), 1);
        }
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

    #[test_case(true)]
    #[test_case(false)]
    fn test_mark_ready_concurrent(start_from_clean_state: bool) {
        let global_cache_manager = Arc::new(ModuleCacheManager::<
            _,
            u32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new());
        if !start_from_clean_state {
            assert!(global_cache_manager.mark_ready(&0, 1));
            assert!(global_cache_manager.mark_executing());
            assert!(global_cache_manager.mark_done());
            // We are at done with value of 1.
        }

        let mut handles = vec![];
        for _ in 0..32 {
            let handle = thread::spawn({
                let global_cache_manager = global_cache_manager.clone();
                move || global_cache_manager.mark_ready(&1, 2)
            });
            handles.push(handle);
        }
        join_and_assert_single_true(handles);
    }

    #[test]
    fn test_mark_executing_concurrent() {
        let global_cache_manager = Arc::new(ModuleCacheManager::<
            _,
            u32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new());
        assert!(global_cache_manager.mark_ready(&0, 1));

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
            _,
            u32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new());
        assert!(global_cache_manager.mark_ready(&0, 1));
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
    fn mark_execution_start_when_different_environment() {
        let module_cache_manager = ModuleCacheManager::<i32, _, _, _, _>::new();

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
        let environment = module_cache_manager.get_or_initialize_environment_unchecked(&state_view);
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
        let environment = module_cache_manager.get_or_initialize_environment_unchecked(&state_view);
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
        let new_environment =
            module_cache_manager.get_or_initialize_environment_unchecked(&state_view);
        assert_eq!(module_cache_manager.module_cache.num_modules(), 1);
        assert!(environment == new_environment);
    }
}
