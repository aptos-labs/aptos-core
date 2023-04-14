// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_logger::error;
use aptos_types::executable::Executable;
use dashmap::DashMap;
use num_derive::FromPrimitive;
use std::{
    hash::Hash,
    sync::atomic::{AtomicU8, AtomicUsize, Ordering},
};

/// Represents the state of the ExecutableStore. The goal is to catch errors in intended
/// use, and reset the executable cache to an empty state if its state can't guarantee
/// matching the state after the parent block id.
///
/// During the execution, the cache might get updated, i.e. if a new executable at storage
/// version gets stored. After block execution, more updates may occur to align the cache
/// contents to the new boundary: published modules may invalidated previously stored
/// executables, and new corresponding executables may also be available. 'mark_updated'
/// method can be called afterwards to mark state accordingly. Missing Updated state
/// would indicate that the cache wasn't sychronized to the new block, and should not be
/// re-used for the subsequent blocks. The Void state in such cases allows graceful
/// handling by resetting / clearing the cache as needed.
///
/// Finally, ExecutableStore must be pruned, to control its size, and then the hash of the
/// block that was executed needs to be set, marking it full circle in the Ready state.
#[derive(Copy, Clone, FromPrimitive, PartialEq, Debug)]
enum ExecutableStoreState {
    Ready = 0,   // The cache is ready-to-use at a state after block id is recorded.
    Updated = 1, // The cache is updated / synchronized after the block execution.
    Pruned = 2,  // The cache is pruned (must record executed block id to become ready).
    Void = 3,    // Unexpected state update order (expected: Ready -> Updated -> Pruned).
}

pub struct ExecutableStore<K: Eq + Hash, X: Executable> {
    executables: DashMap<K, X>,
    total_size: AtomicUsize,
    state: AtomicU8,
    block_id: Mutex<Option<HashValue>>,
}

impl<K: Eq + Hash, X: Executable> Default for ExecutableStore<K, X> {
    fn default() -> Self {
        Self {
            executables: DashMap::new(),
            total_size: AtomicUsize::new(0),
            state: AtomicU8::new(ExecutableStoreState::Ready as u8),
            block_id: Mutex::new(None),
        }
    }
}

impl<K: Eq + Hash, X: Executable> ExecutableStore<K, X> {
    ///
    /// The following methods should be called in quiescence. These are intended to
    /// process the state and state of the Cache between block executions and be called
    /// single-threaded. As such, no extra atomicity of ops within methods is needed.
    ///

    // Flushes the cache and marks state as Ready. This happens for error handling
    // in cases when the empty cache is sufficient to proceed despite the error.
    fn reset(&self) {
        self.executables.clear();
        self.total_size.store(0, Ordering::Relaxed);
        self.block_id.lock().take();
    }

    /// Should be invoked after fully executing a new block w. block_id, and performing
    /// required steps (updating & pruning the cache to align with the new block). If
    /// the state was not updated in a proper order (Ready -> Updated -> Pruned),
    /// then the cache will be cleared instead of recording the block_id.
    pub fn mark_ready(&self, block_id: HashValue) {
        let prev_state = self
            .state
            .swap(ExecutableStoreState::Ready as u8, Ordering::Relaxed);

        if prev_state == ExecutableStoreState::Void as u8 {
            self.reset();
        } else {
            self.block_id.lock().replace(block_id);
        }
    }

    /// This method checks that the state is Ready, and either self.block_id is None
    /// (corresponding to an empty cache), or matching block_id must be provided by the
    /// caller for confirmation. This method panics if the state is not ready, as
    /// mark_ready (or new ExecutableStore) must be used to ensure a proper state.
    /// However, if the block_id does not match the provided parent block id, the cache
    /// is cleared & error is logged for out of order execution.
    pub fn check_ready(&self, parent_block_id: HashValue) {
        assert!(
            self.state.load(Ordering::Relaxed) == ExecutableStoreState::Ready as u8,
            "Executable cache not Ready for block execution"
        );

        let block_id = *self.block_id.lock();
        // Lock is released to avoid reset re-entry. Note: these calls are quiescent.

        if block_id.map_or(false, |id| id != parent_block_id) {
            self.reset();
            error!(
                "ExecutableStore block id {:?} != provided parent block id {:?}",
                block_id, parent_block_id
            );
        }
    }

    /// If the state is observed to be expected_state, set it to new_state.
    /// Otherwise, set state to Void.
    fn set_state(&self, expected_state: ExecutableStoreState, new_state: ExecutableStoreState) {
        let state = self.state.load(Ordering::Relaxed);

        // Load and Store do not need to be atomic as the calling methods are supposed
        // to only be used by a single thread in quiescence.
        self.state.store(
            if state == expected_state as u8 {
                new_state as u8
            } else {
                ExecutableStoreState::Void as u8
            },
            Ordering::Relaxed,
        );
    }

    /// Should be called when the cache is updated to be aligned with the state after
    /// a block execution. Will set state to Void if the previous state wasn't Ready.
    pub fn mark_updated(&self) {
        self.set_state(ExecutableStoreState::Ready, ExecutableStoreState::Updated);
    }

    /// Must be called after block execution is complete, and the cache has been
    /// updated accordingly (to contain base executables after the block execution).
    /// Pruning is required to be able to update the block hash and re-use the
    /// executable cache. If the cache isn't intended to be re-used, there is no
    /// need to prune and new ExecutableStore can be created instead. If the previous
    /// state isn't Updated, the Void state is set.
    ///
    /// Basic eviction policy: if total size > provided threshold, clear everything.
    /// Returns true iff the cache was cleared.
    /// TODO: more complex eviction policy.
    pub fn prune(&self, size_threshold: usize) -> bool {
        let ret = if self.size() > size_threshold {
            self.reset();
            true
        } else {
            false
        };

        self.set_state(ExecutableStoreState::Updated, ExecutableStoreState::Pruned);

        ret
    }

    fn state(&self) -> ExecutableStoreState {
        num_traits::FromPrimitive::from_u8(self.state.load(Ordering::Relaxed)).unwrap()
    }

    fn size(&self) -> usize {
        self.total_size.load(Ordering::Relaxed)
    }

    ///
    /// The following methods can be concurrent when invoked during the block execution.
    ///

    pub fn get(&self, key: &K) -> Option<X> {
        debug_assert!(
            self.state() == ExecutableStoreState::Ready,
            "Getting from an Executable Cache in a not Ready state"
        );

        self.executables.get(key).map(|x| x.clone())
    }

    pub fn insert(&self, key: K, executable: X) {
        debug_assert!(
            self.state() == ExecutableStoreState::Ready,
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
            self.state() == ExecutableStoreState::Ready,
            "Removing from an Executable Cache in a not Ready state"
        );

        if let Some((_, x)) = self.executables.remove(key) {
            // Since the size was added to total size
            self.total_size.fetch_sub(x.size_bytes(), Ordering::Relaxed);
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::{ExecutableStore, ExecutableStoreState};
    use aptos_crypto::HashValue;
    use aptos_types::executable::Executable;
    use claims::{assert_none, assert_some_eq};

    #[derive(Clone, Debug, PartialEq)]
    struct MockExecutable(usize);

    impl Executable for MockExecutable {
        fn size_bytes(&self) -> usize {
            self.0
        }
    }

    #[test]
    fn executable_store_state_loop() {
        let store = ExecutableStore::<usize, MockExecutable>::default();

        assert_eq!(store.state(), ExecutableStoreState::Ready);
        store.insert(0, MockExecutable(3));

        store.mark_updated();
        assert_eq!(store.state(), ExecutableStoreState::Updated);

        assert!(!store.prune(5));
        assert_eq!(store.state(), ExecutableStoreState::Pruned);

        // We will use the size of the cache as a proxy to know whether the flushing / reset
        // has occurred, e.g. for unexpected state or block id.

        let h = HashValue::random();
        store.mark_ready(h);
        store.check_ready(h);
        assert_eq!(store.size(), 3);

        // Checking readiness with wrong previous block ID must clear the cache.
        let h1 = HashValue::random();
        assert_ne!(h, h1);
        store.check_ready(h1);
        assert_eq!(store.size(), 0);
    }

    #[test]
    fn executable_store_prune() {
        let store = ExecutableStore::<usize, MockExecutable>::default();

        store.insert(0, MockExecutable(3));
        store.insert(1, MockExecutable(3));

        store.mark_updated();

        assert!(store.prune(5));
        assert_eq!(store.size(), 0);
    }

    #[test]
    fn executable_store_void() {
        let store = ExecutableStore::<usize, MockExecutable>::default();

        store.insert(0, MockExecutable(3));
        // Pruning before marking updated should lead to the Void state.
        assert!(!store.prune(5));

        assert_eq!(store.state(), ExecutableStoreState::Void);

        // Mark ready must reset the state to Ready & clear the cache.
        assert_eq!(store.size(), 3);
        store.mark_ready(HashValue::random());
        assert_eq!(store.size(), 0);

        assert_eq!(store.state(), ExecutableStoreState::Ready);

        store.insert(0, MockExecutable(3));
        store.mark_updated();
        assert!(!store.prune(5));

        // marking updated in non-ready state must also lead to the Void state.
        store.mark_updated();
        assert_eq!(store.state(), ExecutableStoreState::Void);

        assert_eq!(store.size(), 3);
        store.mark_ready(HashValue::random());
        assert_eq!(store.size(), 0);
    }

    #[test]
    #[should_panic]
    fn insert_not_ready() {
        let store = ExecutableStore::<usize, MockExecutable>::default();
        store.mark_updated();
        store.insert(0, MockExecutable(3));
    }

    #[test]
    #[should_panic]
    fn get_not_ready() {
        let store = ExecutableStore::<usize, MockExecutable>::default();
        store.mark_updated();
        store.get(&0);
    }

    #[test]
    #[should_panic]
    fn remove_not_ready() {
        let store = ExecutableStore::<usize, MockExecutable>::default();
        store.mark_updated();
        store.remove(&0);
    }

    #[test]
    #[should_panic]
    fn check_not_ready() {
        let store = ExecutableStore::<usize, MockExecutable>::default();
        store.mark_updated();
        store.check_ready(HashValue::random());
    }

    #[test]
    fn total_size_simple() {
        let store = ExecutableStore::<usize, MockExecutable>::default();

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
}
