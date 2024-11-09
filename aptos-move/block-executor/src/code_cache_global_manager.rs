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
///   2. [State::Ready] --> [State::Executing].
///   3. [State::Executing] --> [State::Done].
///   4. [State::Done] --> [State::Ready].
#[derive(Clone, Debug, Eq, PartialEq)]
enum State<T> {
    Ready(Option<T>),
    Executing(Option<T>),
    Done(Option<T>),
}

impl<T: Clone + Debug + Eq> State<T> {
    /// If the state is [State::Ready], returns its value. Otherwise, returns [None].
    fn value_from_ready(&self) -> Option<Option<T>> {
        match self {
            State::Ready(v) => Some(v.clone()),
            _ => None,
        }
    }

    /// If the state is [State::Executing], returns its value. Otherwise, returns [None].
    fn value_from_executing(&self) -> Option<Option<T>> {
        match self {
            State::Executing(v) => Some(v.clone()),
            _ => None,
        }
    }

    /// If the state is [State::Done], returns its value. Otherwise, returns [None].
    fn value_from_done(&self) -> Option<Option<T>> {
        match self {
            State::Done(v) => Some(v.clone()),
            _ => None,
        }
    }

    /// Sets the current state to [State::Ready].
    fn set_ready(&mut self, value: Option<T>) {
        *self = Self::Ready(value);
    }

    /// Sets the current state to [State::Executing].
    fn set_executing(&mut self, value: Option<T>) {
        *self = Self::Executing(value);
    }

    /// Sets the current state to [State::Done].
    fn set_done(&mut self, value: Option<T>) {
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

        let recorded_previous = state.value_from_done();
        match (recorded_previous, previous) {
            (None, _) => {
                // We are not in the done state, this is an error.
                alert_or_println!(
                    "Unable to mark ready, state: {:?}, previous: {:?}, current: {:?}",
                    state,
                    previous,
                    current
                );
                false
            },
            (Some(Some(recorded_previous)), Some(previous)) if recorded_previous.eq(previous) => {
                // We are in done state with matching values. Can mark ready.
                state.set_ready(current);
                true
            },
            _ => {
                // If the state is done, but the values do not exist or do not match, we still set
                // the state as ready, but also flush global caches because they execute on top of
                // unknown state (or on top of some different to previous state).
                self.module_cache.flush_unchecked();
                if let Some(environment) = self.environment.acquire().as_ref() {
                    environment
                        .runtime_environment()
                        .flush_struct_name_and_info_caches();
                }

                state.set_ready(current);
                true
            },
        }
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
    pub fn get_or_initialize_environment(&self, state_view: &impl StateView) -> AptosEnvironment {
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

    #[test_case(None)]
    #[test_case(Some(0))]
    fn test_ready_state(value: Option<i32>) {
        let state = State::Ready(value);

        assert_eq!(state.value_from_ready(), Some(value));
        assert!(state.value_from_executing().is_none());
        assert!(state.value_from_done().is_none());
    }

    #[test_case(None)]
    #[test_case(Some(0))]
    fn test_executing_state(value: Option<i32>) {
        let state = State::Executing(value);

        assert!(state.value_from_ready().is_none());
        assert_eq!(state.value_from_executing(), Some(value));
        assert!(state.value_from_done().is_none());
    }

    #[test_case(None)]
    #[test_case(Some(0))]
    fn test_done_state(value: Option<i32>) {
        let state = State::Done(value);

        assert!(state.value_from_ready().is_none());
        assert!(state.value_from_executing().is_none());
        assert_eq!(state.value_from_done(), Some(value));
    }

    #[test]
    fn test_set_state() {
        let mut state = State::Done(None);

        state.set_ready(Some(0));
        assert_matches!(state, State::Ready(Some(0)));

        state.set_executing(Some(10));
        assert_matches!(state, State::Executing(Some(10)));

        state.set_done(Some(100));
        assert_matches!(state, State::Done(Some(100)));

        state.set_ready(Some(1000));
        assert_matches!(state, State::Ready(Some(1000)));
    }

    #[test_case(None, None)]
    #[test_case(None, Some(1))]
    #[test_case(Some(0), None)]
    #[test_case(Some(0), Some(1))]
    #[test_case(Some(0), Some(0))]
    fn test_mark_ready(recorded_previous: Option<i32>, previous: Option<i32>) {
        let module_cache_manager = ModuleCacheManager::new();
        module_cache_manager
            .state
            .lock()
            .set_done(recorded_previous);

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
    fn test_mark_executing() {
        let module_cache_manager = ModuleCacheManager::<
            _,
            u32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new();
        module_cache_manager.state.lock().set_ready(Some(100));

        assert!(!module_cache_manager.mark_ready(Some(&76), Some(77)));
        assert!(!module_cache_manager.mark_done());

        assert!(module_cache_manager.mark_executing());

        let state = module_cache_manager.state.lock().clone();
        assert_eq!(state, State::Executing(Some(100)))
    }

    #[test]
    fn test_mark_done() {
        let module_cache_manager = ModuleCacheManager::<
            _,
            u32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new();
        module_cache_manager.state.lock().set_executing(Some(100));

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
            _,
            u32,
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
            _,
            u32,
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
            _,
            u32,
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
        let environment = module_cache_manager.get_or_initialize_environment(&state_view);
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
        let environment = module_cache_manager.get_or_initialize_environment(&state_view);
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
        let new_environment = module_cache_manager.get_or_initialize_environment(&state_view);
        assert_eq!(module_cache_manager.module_cache.num_modules(), 1);
        assert!(environment == new_environment);
    }
}
