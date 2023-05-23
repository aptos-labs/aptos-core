// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_infallible::Mutex;
use aptos_logger::{error, info};
use aptos_types::executable::Executable;
use dashmap::DashMap;
use std::{
    fmt::Debug,
    hash::Hash,
    sync::atomic::{AtomicUsize, Ordering},
};

/// Represents the state of the ExecutableStore. Introduced to ensure the proper usage
/// of the ExecutableStore throughout the system, as due to the current codebase structure
/// constraints, different important steps are performed in different crates. Such
/// defensive programming here ensures correctness (clearing the cache on any unexpected
/// state updates) and observability to quickly identify any incorrect usage.
///
/// Initial state is Empty, and it is also possible to get to the Empty state by clearing
/// the ExecutableStore at any time.
///
/// Execution in the system happens piece-wiese, where each piece contains a continuous
/// sub-sequence of ledger of the ordered transactions. For example, in our codebase,
/// pieces are called 'blocks' during the original execution or 'chunks' during state-sync.
///
/// Most of the methods, e.g. to manage the state, are designed to be called in quiescence,
/// i.e. not concurrently with other methods, and in between the piecewise execution. An
/// exception to this occurs while the cache in the Before(id) state is updated, adding and
/// removing elements (potentially concurrently) to convert it to contain the executables
/// at a version corresponding to after the execution of the piece. When the update is done,
/// it is the user's responsibility to mark the state as Updated. Currently, this happens
/// right at the end of parallel execution inside block-executor (sequential execution mode
/// currently does not re-use the cache acrosss pieces).
///
/// ExecutableStore must be pruned to control the memory usage. Currently this is done after
/// the cache is updated, from outside the block-executor (in aptos-vm). Pruning happens
/// based on a threshold parameter, and it simply clears the store if the size exceeds
/// the threshold. It is the caller's responsibility to properly set the status to Pruned
/// (ensuring we will never use the ExecutableStore without pruning).
///
/// After pruning, the state is supposed to change to After(id), with the id of the most
/// recent piece that was executed. To change the state from After(id) to Before(id'), the
/// parent of piece id' (as provided by the caller) is conformed to be id, ensuring
/// correctness. These states are currently set in execution/executor, as the information
/// about pieces (ids and their relation to each other) is only available there.
///
/// The full state transition diagram is below:
///
/// Empty ----------> Updated(None)
///                      |
///                      |
///                      ↓
///                   Pruned(None)
///                      |
///                      |
///                      ↓
///   --------------- After(id) -------------> Before(id')
///   |                                            |
///   |                                            |
///   ↓                                            |
/// Empty ----------> Updated(Option<ID>) <---------
///                     ...
///                     ...
/// Note: After transitioning to Updated from Empty, the exact state is Updated(None),
/// while after transitioning from Before(id'), the state is Updated(Some(id')).
/// Similar principle applies to the Pruned state. Keeping id' allows to also confirm
/// the consecutive nature of piece ids when marking After state.
///
#[derive(Copy, Clone, PartialEq, Debug)]
enum ExecutableStoreState<ID: Clone + Debug + PartialEq> {
    // Cache is empty and ready to be used.
    Empty,
    // The cache is ready-to-use at a ledger state before a piece of txns with ID id.
    // We could represent empty as Before(None), but separating for clarity.
    Before(ID),
    // The cache is updated / synchronized after a piece is executed.
    Updated(Option<ID>),
    // The cache is pruned to control memory usage.
    Pruned(Option<ID>),
    // Ledger state after piece with ID id. Allows us to ensure that the state will
    // not transition to Before(id') unless the ID of the parent of piece id' is id.
    After(ID),
}

/// struct that represents a simple cache implementation for Executables that, intended
/// to be re-used when executing transactions in separate but consecutive pieces.
pub struct ExecutableStore<K: Eq + Hash, X: Executable, ID: Clone + Debug + PartialEq> {
    executables: DashMap<K, X>,
    total_size: AtomicUsize,
    state: Mutex<ExecutableStoreState<ID>>,
}

impl<K: Eq + Hash, X: Executable, ID: Clone + Debug + PartialEq> Default
    for ExecutableStore<K, X, ID>
{
    fn default() -> Self {
        Self {
            executables: DashMap::new(),
            total_size: AtomicUsize::new(0),
            state: Mutex::new(ExecutableStoreState::<ID>::Empty),
        }
    }
}

impl<K: Eq + Hash, X: Executable, ID: Clone + Debug + PartialEq> ExecutableStore<K, X, ID> {
    ///
    /// Quiescent interfaces:
    /// The following methods should be called in quiescence. These are intended to
    /// process the state of the Store between piece executions and must be called
    /// single-threaded. As such, no extra atomicity of ops within methods is needed.
    ///

    /// Flushes the cache and marks the state as Empty. This happens for error handling
    /// in cases when the empty cache is sufficient to proceed despite the error. O.w.
    /// clearing an ExecutableStore is just an alternative to creating a new empty one.
    pub fn clear(&self) {
        self.executables.clear();
        self.total_size.store(0, Ordering::Relaxed);
        *self.state.lock() = ExecutableStoreState::Empty;
    }

    /// Should be invoked after the store is updated to correspond to the ledger state
    /// after a piece is executed. Ensures that the previous state was Empty / Before,
    /// otherwise the store is cleared.
    pub fn set_state_updated(&self) {
        let mut state = self.state.lock();

        match state.clone() {
            ExecutableStoreState::Empty => {
                *state = ExecutableStoreState::Updated(None);
            },
            ExecutableStoreState::Before(id) => {
                *state = ExecutableStoreState::Updated(Some(id));
            },
            _ => {
                error!("Updating ExecutableStore in a wrong state = {:?}", state);

                // Avoid re-entry on state mutex (clear acquires the lock as well).
                drop(state);

                self.clear();
            },
        }
    }

    /// Must be invoked after piece execution is complete, and after the cache has been
    /// updated to contain executables at a ledger state after the piece execution. If
    /// the previous status isn't Updated, the store is cleared.
    ///
    /// Current implementation uses a basic eviction policy: if total size > provided
    /// threshold, the store is cleared. TODO: more complex eviction policy.
    ///
    /// Note that the threshold is not provided at the construction time due to current
    /// codebase constraints, and is instead provided as an argument to each 'prune'
    /// invocation. TODO: provide threshold at construction, which will also allow
    /// merging the 'Updated' and 'Pruned' states.
    pub fn prune(&self, size_threshold: usize) {
        if self.size() > size_threshold {
            info!(
                "Clear ExecutableStore on prune size = {} > threshold = {}",
                self.size(),
                size_threshold
            );
            self.clear();
        } else {
            let mut state = self.state.lock();

            if let ExecutableStoreState::Updated(maybe_id) = state.clone() {
                *state = ExecutableStoreState::Pruned(maybe_id)
            } else {
                error!("Pruning ExecutableStore in a wrong state = {:?}", state);

                // Avoid re-entry on state mutex (clear acquires the lock as well).
                drop(state);

                self.clear();
            }
        };
    }

    /// Should be invoked after pruning, otherwise the cache will be cleared. Ensures
    /// that no usage will bypass the pruning step, which would lead to OOM issues.
    pub fn set_state_after(&self, id: ID) {
        let mut state = self.state.lock();
        let must_clear = if let ExecutableStoreState::Pruned(maybe_id) = state.clone() {
            *state = ExecutableStoreState::After(id.clone());

            // If piece ID exists, it must match the provided id (or if the id is
            // unexpected, the cache must be cleared).
            maybe_id.map_or(false, |cur_id| cur_id != id)
        } else {
            // Unexpected previous state, must clear.
            true
        };

        if must_clear {
            error!(
                "Setting state to After (id = {:?}) from a wrong state {:?}",
                id, state
            );

            // Avoid re-entry on state mutex (clear acquires the lock as well).
            drop(state);

            self.clear();
        }
    }

    /// Should be invoked to set the state before executing a piece with id ID.
    /// The prior state must be After(parent_id), otherwise the store is cleared
    /// to protect against incorrect execution with out-of-order pieces.
    pub fn set_state_before(&self, parent_id: ID, id: ID) {
        let mut state = self.state.lock();
        let must_clear = if let ExecutableStoreState::After(prev_id) = state.clone() {
            *state = ExecutableStoreState::Before(id.clone());

            // The cache will be cleared if prev_id doesn't match the parent_id.
            prev_id != parent_id
        } else {
            // Unexpected previous state, must clear.
            true
        };

        if must_clear {
            error!(
                "Setting state to Before (id = {:?}) from a wrong state {:?}",
                id, state
            );

            // Avoid re-entry on state mutex (clear acquires the lock as well).
            drop(state);

            self.clear();
        }
    }

    ///
    /// The following methods can be concurrent when invoked during the block execution.
    ///

    pub fn get(&self, key: &K) -> Option<X> {
        debug_assert!(
            matches!(
                self.state(),
                ExecutableStoreState::Empty | ExecutableStoreState::Before(_)
            ),
            "Getting from an Executable Cache in a not Ready state"
        );

        self.executables.get(key).map(|x| x.clone())
    }

    pub fn insert(&self, key: K, executable: X) {
        debug_assert!(
            matches!(
                self.state(),
                ExecutableStoreState::Empty | ExecutableStoreState::Before(_)
            ),
            "Inserting to an Executable Cache in a not Ready state"
        );

        // Add size first so subtract on concurrent remove does not underflow.
        // Note: We could insert the executable first and then adjust the size and perform
        // 1 atomic operation instead of 2 when an executable is already in the cache.
        // However, (1) this requires signed type for size to avoid underflow (confusing),
        // and (2) should not be frequent (caused only by a module upgrade, or by a race
        // between parallel executor threads concurrently preparing the same executable).
        self.total_size
            .fetch_add(executable.size_bytes(), Ordering::Relaxed);

        if let Some(x) = self.executables.insert(key, executable) {
            // Adjust total size if the cache already contained an executable.
            self.total_size.fetch_sub(x.size_bytes(), Ordering::Relaxed);
        }
    }

    pub fn remove(&self, key: &K) {
        debug_assert!(
            matches!(
                self.state(),
                ExecutableStoreState::Empty | ExecutableStoreState::Before(_)
            ),
            "Removing from an Executable Cache in a not Ready state"
        );

        if let Some((_, x)) = self.executables.remove(key) {
            // Since the size was added to total size
            self.total_size.fetch_sub(x.size_bytes(), Ordering::Relaxed);
        };
    }

    //
    // Private utility methods.
    //

    fn state(&self) -> ExecutableStoreState<ID> {
        self.state.lock().clone()
    }

    fn size(&self) -> usize {
        self.total_size.load(Ordering::Relaxed)
    }

    // Computes size from the self.executables DashMap data-structure. For testing only.
    #[cfg(test)]
    fn size_for_test(&self) -> usize {
        let mut ret = 0;
        for x in self.executables.iter() {
            ret += x.size_bytes();
        }
        ret
    }
}

#[cfg(test)]
mod tests {
    use crate::{ExecutableStore, ExecutableStoreState};
    use aptos_types::executable::Executable;
    use claims::{assert_none, assert_some_eq};
    use proptest::{collection::vec, prelude::*};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Clone, Debug, PartialEq)]
    struct MockExecutable(usize);

    impl Executable for MockExecutable {
        fn size_bytes(&self) -> usize {
            self.0
        }
    }

    // Execute a piece w. given id, consisting of a single executable with given size.
    fn execute_singleton_piece(
        store: &ExecutableStore<usize, MockExecutable, usize>,
        id: usize,
        size: usize,
        started_empty: bool,
    ) {
        let maybe_id = (!started_empty).then_some(id);

        // Insert at a fixed key, so it might overwrite previous executable.
        store.insert(0, MockExecutable(size));
        let new_size = store.size();

        store.set_state_updated();
        assert_eq!(store.state(), ExecutableStoreState::Updated(maybe_id));

        // Provide new size as threshold so normal pruning won't clear (must be >).
        store.prune(new_size);
        assert_eq!(store.state(), ExecutableStoreState::Pruned(maybe_id));
        // We will use the size of the cache as a proxy to know whether the flushing / reset
        // has occurred, e.g. for unexpected state or block id.
        assert_eq!(store.size(), new_size);

        store.set_state_after(id);
        assert_eq!(store.state(), ExecutableStoreState::After(id));
        assert_eq!(store.size(), new_size);
    }

    #[test]
    fn executable_store_state_loop() {
        let store = ExecutableStore::<usize, MockExecutable, usize>::default();

        assert_eq!(store.state(), ExecutableStoreState::Empty);
        execute_singleton_piece(&store, 1, 5, true);

        store.set_state_before(1, 2);
        assert_eq!(store.state(), ExecutableStoreState::Before(2));
        assert_eq!(store.size(), 5);

        execute_singleton_piece(&store, 2, 5, false);
        store.set_state_before(2, 3);
        assert_eq!(store.state(), ExecutableStoreState::Before(3));
        assert_eq!(store.size(), 5);
    }

    #[test]
    fn executable_store_prune() {
        let store = ExecutableStore::<usize, MockExecutable, ()>::default();

        store.insert(0, MockExecutable(3));
        assert_eq!(store.size(), 3);
        store.insert(1, MockExecutable(3));

        store.set_state_updated();

        assert_eq!(store.size(), 6);
        store.prune(5);
        assert_eq!(store.size(), 0);
    }

    #[test]
    fn unexpected_state_clear_prune() {
        let store = ExecutableStore::<usize, MockExecutable, ()>::default();

        store.insert(0, MockExecutable(3));
        assert_eq!(store.size(), 3);

        store.prune(5);
        // Cleared due to state mismatch (prev state != Updated), not threshold.
        assert_eq!(store.state(), ExecutableStoreState::Empty);
        assert_eq!(store.size(), 0);
    }

    #[test]
    fn unexpected_state_clear_update() {
        let store = ExecutableStore::<usize, MockExecutable, ()>::default();

        store.insert(0, MockExecutable(3));
        assert_eq!(store.size(), 3);
        store.set_state_updated();

        store.prune(5);
        assert_eq!(store.size(), 3);

        store.set_state_updated();
        // Cleared due to state mismatch (prev state != Empty or Before).
        assert_eq!(store.state(), ExecutableStoreState::Empty);
        assert_eq!(store.size(), 0);
    }

    #[test]
    fn unexpected_state_clear_after() {
        let store = ExecutableStore::<usize, MockExecutable, usize>::default();

        store.insert(0, MockExecutable(3));
        store.set_state_updated();

        // Not cleared.
        assert_eq!(store.size(), 3);
        store.set_state_after(1);
        assert_eq!(store.state(), ExecutableStoreState::Empty);
        assert_eq!(store.size(), 0);
        // Cleared due to state mismatch (previous state != Pruned).

        // Start over.
        store.insert(0, MockExecutable(3));
        store.set_state_updated();
        store.prune(5);
        store.set_state_after(1);
        store.set_state_before(1, 2);
        store.set_state_updated();
        store.prune(5);
        // Not cleared so far.
        assert_eq!(store.size(), 3);

        store.set_state_after(3);
        // Cleared due to ID mismatch (3 != 2, piece id).
        assert_eq!(store.state(), ExecutableStoreState::Empty);
        assert_eq!(store.size(), 0);
    }

    #[test]
    fn unexpected_state_clear_before() {
        let store = ExecutableStore::<usize, MockExecutable, usize>::default();

        store.insert(0, MockExecutable(3));
        store.set_state_updated();
        store.prune(5);
        store.set_state_after(1);
        store.set_state_before(2, 3);
        // Cleared due to ID mismatch (1 != 2, prev piece id).
        assert_eq!(store.state(), ExecutableStoreState::Empty);
        assert_eq!(store.size(), 0);

        // Start over.
        store.insert(0, MockExecutable(3));
        store.set_state_updated();
        store.prune(5);
        store.set_state_after(1);
        store.set_state_before(1, 2);
        store.set_state_updated();
        // Not cleared so far.
        assert_eq!(store.size(), 3);

        store.set_state_before(2, 3);
        // Cleared due to state mismatch (prev state != Before)
        assert_eq!(store.state(), ExecutableStoreState::Empty);
        assert_eq!(store.size(), 0);
    }

    #[test]
    #[should_panic]
    fn insert_not_ready() {
        let store = ExecutableStore::<usize, MockExecutable, ()>::default();
        store.set_state_updated();
        store.insert(0, MockExecutable(3));
    }

    #[test]
    #[should_panic]
    fn get_not_ready() {
        let store = ExecutableStore::<usize, MockExecutable, ()>::default();
        store.set_state_updated();
        store.get(&0);
    }

    #[test]
    #[should_panic]
    fn remove_not_ready() {
        let store = ExecutableStore::<usize, MockExecutable, ()>::default();
        store.set_state_updated();
        store.remove(&0);
    }

    #[test]
    fn total_size_simple() {
        let store = ExecutableStore::<usize, MockExecutable, ()>::default();

        // Insert to keys 0,1,2.
        store.insert(0, MockExecutable(3));
        store.insert(1, MockExecutable(10));
        store.insert(2, MockExecutable(100));
        assert_eq!(store.size(), 113);

        // Remove keys 1 & 2.
        store.remove(&1);
        assert_eq!(store.size(), 103);
        store.remove(&2);
        assert_eq!(store.size(), 3);

        // Should overwrite previously stored value at key 0.
        store.insert(0, MockExecutable(4));
        assert_eq!(store.size(), 4);

        // Insert to key 3.
        store.insert(3, MockExecutable(30));
        assert_eq!(store.size(), 34);

        // Re-insert key 2.
        store.insert(2, MockExecutable(200));
        assert_eq!(store.size(), 234);

        // Actually check that the proper executables are stored (by size).
        assert_some_eq!(store.get(&0), MockExecutable(4));
        assert_none!(store.get(&1));
        assert_some_eq!(store.get(&2), MockExecutable(200));
        assert_some_eq!(store.get(&3), MockExecutable(30));
    }

    fn test_concurrent_ops_size(ops: Vec<(usize, usize, usize)>) {
        let store = ExecutableStore::<usize, MockExecutable, ()>::default();

        let ops: Vec<(usize, usize, usize)> = ops
            .iter()
            .map(|(a, b, c)| (a % 3, b % 50, c % 1000))
            .collect();

        // Parameters according to which the ops are generated.
        let num_ops_per_thread = 2000;
        let num_threads = 4;

        let idx_gen = AtomicUsize::new(0);
        rayon::scope(|s| {
            for _ in 0..num_threads {
                s.spawn(|_| {
                    let thread_idx = idx_gen.fetch_add(1, Ordering::Relaxed);

                    for (op_kind, key, size) in ops
                        .iter()
                        .skip(thread_idx * num_ops_per_thread)
                        .take(num_ops_per_thread)
                    {
                        match op_kind {
                            0 => {
                                store.get(key);
                            },
                            1 => {
                                store.insert(*key, MockExecutable(*size));
                            },
                            2 => {
                                store.remove(key);
                            },
                            _ => unreachable!("Op alternatives"),
                        }
                    }
                });
            }
        });

        // Confirm that the size stored in a flag internally matches the data-structure
        // at the end of executing above concurrent operations.
        assert_eq!(store.size(), store.size_for_test());
    }

    proptest! {
        #[test]
        fn concurrent_ops_size_proptest(
            cache_ops in vec((any::<usize>(), any::<usize>(), any::<usize>()), 2000 * 4),
        ) {
            test_concurrent_ops_size(cache_ops);
        }
    }
}
