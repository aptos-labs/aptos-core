// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::metrics::{COUNTER, GAUGE, OTHER_TIMERS_SECONDS};
use anyhow::{ensure, Result};
use aptos_config::config::HotStateConfig;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_metrics_core::{IntCounterVecHelper, IntGaugeVecHelper, TimerHelper};
use aptos_storage_interface::state_store::{
    state::State, state_delta::StateDelta, state_view::hot_state_view::HotStateView,
};
use aptos_types::state_store::{
    hot_state::THotStateSlot, state_key::StateKey, state_slot::StateSlot, NUM_STATE_SHARDS,
};
#[cfg(test)]
use aptos_types::transaction::Version;
use arr_macro::arr;
use dashmap::{mapref::one::Ref, DashMap};
#[cfg(test)]
use std::collections::BTreeMap;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, RecvTimeoutError, SyncSender, TryRecvError},
        Arc, Weak,
    },
    time::Duration,
};

const MAX_HOT_STATE_COMMIT_BACKLOG: usize = 10;
const DEFERRED_MERGE_RETRY_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Debug)]
struct Shard<K, V>
where
    K: Eq + std::hash::Hash,
{
    inner: DashMap<K, V>,
}

impl<K, V> Shard<K, V>
where
    K: Clone + Eq + std::hash::Hash,
    V: Clone,
{
    fn new(max_items: usize) -> Self {
        Self {
            inner: DashMap::with_capacity(max_items),
        }
    }

    fn get(&self, key: &K) -> Option<Ref<'_, K, V>> {
        self.inner.get(key)
    }

    fn insert(&self, key: K, value: V) -> Option<V> {
        self.inner.insert(key, value)
    }

    fn remove(&self, key: &K) -> Option<(K, V)> {
        self.inner.remove(key)
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    #[cfg(test)]
    fn iter(&self) -> impl Iterator<Item = (K, V)> + use<'_, K, V> {
        self.inner
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
    }
}

#[derive(Debug)]
struct HotStateBase<K = StateKey, V = StateSlot>
where
    K: Eq + std::hash::Hash,
{
    shards: [Shard<K, V>; NUM_STATE_SHARDS],
}

impl<K, V> HotStateBase<K, V>
where
    K: Clone + Eq + std::hash::Hash,
    V: Clone,
{
    fn new_empty(max_items_per_shard: usize) -> Self {
        Self {
            shards: arr![Shard::new(max_items_per_shard); 16],
        }
    }

    fn get_from_shard(&self, shard_id: usize, key: &K) -> Option<Ref<'_, K, V>> {
        self.shards[shard_id].get(key)
    }

    fn len(&self) -> usize {
        self.shards.iter().map(|s| s.len()).sum()
    }
}

/// A composite HotStateView: checks the delta first, falls back to the base DashMaps.
/// The delta covers changes from what's actually in the DashMaps (`merged_state`) to
/// the current committed state. This enables RCU semantics: the new committed state is
/// published immediately via the delta overlay, while DashMap mutations are deferred
/// until all old readers are gone.
struct LayeredHotStateView {
    /// If `Some`, overlay these changes on top of base. If `None`, base is up-to-date.
    delta: Option<StateDelta>,
    base: Arc<HotStateBase>,
}

impl HotStateView for LayeredHotStateView {
    fn get_state_slot(&self, state_key: &StateKey) -> Option<StateSlot> {
        if let Some(delta) = &self.delta {
            if let Some(slot) = delta.get_state_slot(state_key) {
                // Delta says this key changed. If hot, return it.
                // If cold/evicted, return None (do NOT fall through to base,
                // because the key was explicitly evicted in the committed state).
                return if slot.is_hot() { Some(slot) } else { None };
            }
        }
        // Key not in delta (unchanged) -- read from base DashMap.
        let shard_id = state_key.get_shard_id();
        self.base
            .get_from_shard(shard_id, state_key)
            .map(|v| v.clone())
    }
}

/// Bundles the committed `State` with a `HotStateView` that is consistent with it.
/// Protected by a `Mutex` so `get_committed()` and the `Committer` can atomically
/// swap both together.
struct CommittedSnapshot {
    state: State,
    view: Arc<dyn HotStateView>,
}

pub struct HotState {
    base: Arc<HotStateBase>,
    committed: Arc<Mutex<CommittedSnapshot>>,
    commit_tx: SyncSender<State>,
    /// Updated by the Committer after each successful DashMap merge.
    /// Tests use this to wait for the merge to complete before inspecting DashMaps.
    /// Always created and passed to the Committer; only read by test helpers.
    #[allow(dead_code)]
    merged_version: Arc<AtomicU64>,
}

impl HotState {
    pub fn new(state: State, config: HotStateConfig) -> Self {
        let base = Arc::new(HotStateBase::new_empty(config.max_items_per_shard));
        let view: Arc<dyn HotStateView> = Arc::new(LayeredHotStateView {
            delta: None,
            base: base.clone(),
        });
        let committed = Arc::new(Mutex::new(CommittedSnapshot {
            state: state.clone(),
            view,
        }));
        let merged_version = Arc::new(AtomicU64::new(state.next_version()));
        let commit_tx = Committer::spawn(
            base.clone(),
            committed.clone(),
            state,
            merged_version.clone(),
        );

        Self {
            base,
            committed,
            commit_tx,
            merged_version,
        }
    }

    pub(crate) fn set_committed(&self, state: State) {
        let mut committed = self.committed.lock();
        committed.state = state;
        // Reset view to base-only (no delta). hack_reset is only called when
        // no commits are in flight, so DashMaps and committed state are in sync
        // from the readers' perspective (the Committer will re-sync merged_state
        // when it detects the non-descendant case).
        committed.view = Arc::new(LayeredHotStateView {
            delta: None,
            base: self.base.clone(),
        });
    }

    pub fn get_committed(&self) -> (Arc<dyn HotStateView>, State) {
        let committed = self.committed.lock();
        (Arc::clone(&committed.view), committed.state.clone())
    }

    pub fn enqueue_commit(&self, to_commit: State) {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["hot_state_enqueue_commit"]);

        self.commit_tx
            .send(to_commit)
            .expect("Failed to queue for hot state commit.")
    }

    /// Wait until DashMaps have been merged up to at least the given version.
    #[cfg(test)]
    pub fn wait_for_merge(&self, next_version: Version) {
        while self.merged_version.load(Ordering::Acquire) < next_version {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    #[cfg(test)]
    pub fn get_all_entries(&self, shard_id: usize) -> BTreeMap<StateKey, StateSlot> {
        self.base.shards[shard_id].iter().collect()
    }
}

pub struct Committer {
    base: Arc<HotStateBase>,
    committed: Arc<Mutex<CommittedSnapshot>>,
    rx: Receiver<State>,
    total_key_bytes: usize,
    total_value_bytes: usize,
    /// Points to the newest entry. `None` if empty.
    heads: [Option<StateKey>; NUM_STATE_SHARDS],
    /// Points to the oldest entry. `None` if empty.
    tails: [Option<StateKey>; NUM_STATE_SHARDS],

    /// The state that the base DashMaps currently reflect. May lag behind
    /// `committed.state` while a merge is deferred.
    merged_state: State,

    /// Weak refs to ALL previously published views that were created while
    /// DashMaps reflected `merged_state`.
    ///
    /// Why we must track ALL of them (not just the latest):
    /// Each view contains a delta covering `(merged_state, X]` for some
    /// committed state X. For keys NOT in that delta, the view falls through
    /// to the DashMaps, which must still reflect `merged_state` for the
    /// read to be correct. If we advance DashMaps (merge) to state Y while
    /// ANY view still assumes DashMaps = `merged_state`, a key that changed
    /// between `merged_state` and Y but is NOT in that view's delta would
    /// return Y's value instead of `merged_state`'s value -- a wrong read.
    ///
    /// Therefore, merging is only safe when EVERY old view has been dropped
    /// by all readers (strong_count == 0 for all Weaks here).
    ///
    /// In steady state (no fork, readers finish quickly) this Vec typically
    /// has 0-1 entries and drains immediately.
    old_views: Vec<Weak<dyn HotStateView>>,

    /// Shared with HotState; updated after each successful DashMap merge.
    merged_version: Arc<AtomicU64>,
}

impl Committer {
    fn spawn(
        base: Arc<HotStateBase>,
        committed: Arc<Mutex<CommittedSnapshot>>,
        initial_state: State,
        merged_version: Arc<AtomicU64>,
    ) -> SyncSender<State> {
        let (tx, rx) = std::sync::mpsc::sync_channel(MAX_HOT_STATE_COMMIT_BACKLOG);
        std::thread::spawn(move || {
            Self::new(base, committed, rx, initial_state, merged_version).run()
        });

        tx
    }

    fn new(
        base: Arc<HotStateBase>,
        committed: Arc<Mutex<CommittedSnapshot>>,
        rx: Receiver<State>,
        initial_state: State,
        merged_version: Arc<AtomicU64>,
    ) -> Self {
        Self {
            base,
            committed,
            rx,
            total_key_bytes: 0,
            total_value_bytes: 0,
            heads: arr![None; 16],
            tails: arr![None; 16],
            merged_state: initial_state,
            old_views: Vec::new(),
            merged_version,
        }
    }

    fn run(&mut self) {
        info!("HotState committer thread started.");

        while let Some(to_commit) = self.next_to_commit() {
            // 0. Handle hack_reset: if to_commit is not a descendant of merged_state,
            //    a reset happened. Sync merged_state from committed.state.
            if !to_commit.is_descendant_of(&self.merged_state) {
                self.merged_state = self.committed.lock().state.clone();
                self.old_views.clear();
                // If still not a descendant, something is very wrong.
                assert!(to_commit.is_descendant_of(&self.merged_state));
            }

            // 1. Try to merge any previously deferred delta first.
            //    This reduces the size of the new delta we're about to create.
            self.try_merge();

            // 2. Skip if DashMaps already reflect this state (e.g., try_merge just
            //    merged to this exact state).
            if self.merged_state.is_the_same(&to_commit) {
                // Still need to update committed.state in case it's behind.
                let mut committed = self.committed.lock();
                committed.state = to_commit;
                continue;
            }

            // 3. Compute delta from what DashMaps actually reflect to the new state.
            let delta = to_commit.make_delta(&self.merged_state);

            // 4. Create new view with the delta overlay.
            let new_view: Arc<dyn HotStateView> = Arc::new(LayeredHotStateView {
                delta: Some(delta),
                base: self.base.clone(),
            });

            // 5. Atomically publish new view + state. Capture old view.
            let old_view = {
                let mut committed = self.committed.lock();
                let old = std::mem::replace(&mut committed.view, Arc::clone(&new_view));
                committed.state = to_commit;
                old
            };

            // 6. Track old view via Weak for deferred merge.
            //    We must track ALL outstanding views, not just the most recent,
            //    because any of them might still be held by readers who expect
            //    DashMaps to reflect merged_state.
            self.old_views.push(Arc::downgrade(&old_view));
            drop(old_view); // release our strong ref

            // 7. Try to merge immediately if possible.
            self.try_merge();

            GAUGE.set_with(&["hot_state_items"], self.base.len() as i64);
            GAUGE.set_with(&["hot_state_key_bytes"], self.total_key_bytes as i64);
            GAUGE.set_with(&["hot_state_value_bytes"], self.total_value_bytes as i64);
        }

        // Final merge attempt before quitting.
        self.try_merge();

        info!("HotState committer quitting.");
    }

    fn next_to_commit(&mut self) -> Option<State> {
        // Block-wait for the first item. When there are deferred merges
        // (old views not yet dropped), use a timeout so we periodically
        // wake up to retry try_merge(). Without this, the Committer would
        // sleep forever waiting for a new commit while wait_for_merge()
        // callers are blocked waiting for the merge that can only happen
        // here.
        let first = loop {
            if self.old_views.is_empty() {
                // No pending merges -- block indefinitely.
                match self.rx.recv() {
                    Ok(state) => break state,
                    Err(_) => return None,
                }
            } else {
                // Pending merges -- use timeout to periodically retry.
                match self.rx.recv_timeout(DEFERRED_MERGE_RETRY_INTERVAL) {
                    Ok(state) => break state,
                    Err(RecvTimeoutError::Timeout) => {
                        self.try_merge();
                        continue;
                    },
                    Err(RecvTimeoutError::Disconnected) => return None,
                }
            }
        };

        let mut ret = first;
        let mut n_backlog = 0;
        // try to drain all backlog
        loop {
            match self.rx.try_recv() {
                Ok(state) => {
                    n_backlog += 1;
                    ret = state;
                },
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    return None;
                },
            }
        }

        GAUGE.set_with(&["hot_state_commit_backlog"], n_backlog);
        Some(ret)
    }

    /// Attempt to merge deferred deltas into the base DashMaps.
    ///
    /// Merging is ONLY safe when all old views have been dropped by all readers.
    /// Each outstanding view has a delta covering `(merged_state, X]` for some state X,
    /// and assumes DashMaps reflect `merged_state`. A key K not in that delta falls
    /// through to DashMaps. If we advance DashMaps to state Y (by merging), K might have
    /// a different value at Y than at `merged_state`, breaking the view's correctness.
    ///
    /// Post-merge correctness for the "current" view: After merge, the current
    /// `committed.view` is replaced with a clean (delta=None) view. Any reader that
    /// already holds a clone of the old delta-bearing current view is fine: the delta
    /// shadows the DashMaps for changed keys, and for unchanged keys the DashMaps and
    /// the delta's target state agree. So the view remains correct even though DashMaps
    /// have been updated to match it.
    fn try_merge(&mut self) {
        // Prune dead views.
        self.old_views.retain(|w| w.strong_count() > 0);

        if !self.old_views.is_empty() {
            return; // Some readers still hold old views.
        }

        // All old views are gone. Merge to the current committed.state.
        let target = self.committed.lock().state.clone();
        if self.merged_state.is_the_same(&target) {
            return; // Already up-to-date.
        }

        self.apply_delta_to_base(&target);
        self.merged_state = target;

        // Update merged_version so tests can wait for merge completion.
        self.merged_version
            .store(self.merged_state.next_version(), Ordering::Release);

        // Publish clean view (no delta overhead for future readers).
        let clean_view: Arc<dyn HotStateView> = Arc::new(LayeredHotStateView {
            delta: None,
            base: self.base.clone(),
        });
        self.committed.lock().view = clean_view;
    }

    /// Apply the delta between `merged_state` and `target` to the base DashMaps.
    /// This is the same logic as the original `commit()` method.
    fn apply_delta_to_base(&mut self, target: &State) {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["hot_state_commit"]);

        let mut n_insert = 0;
        let mut n_update = 0;
        let mut n_evict = 0;

        let delta = target.make_delta(&self.merged_state);
        for shard_id in 0..NUM_STATE_SHARDS {
            for (key, slot) in delta.shards[shard_id].iter() {
                if slot.is_hot() {
                    let key_size = key.size();
                    self.total_key_bytes += key_size;
                    self.total_value_bytes += slot.size();
                    if let Some(old_slot) = self.base.shards[shard_id].insert(key, slot) {
                        self.total_key_bytes -= key_size;
                        self.total_value_bytes -= old_slot.size();
                        n_update += 1;
                    } else {
                        n_insert += 1;
                    }
                } else if let Some((key, old_slot)) = self.base.shards[shard_id].remove(&key) {
                    self.total_key_bytes -= key.size();
                    self.total_value_bytes -= old_slot.size();
                    n_evict += 1;
                }
            }
            self.heads[shard_id] = target.latest_hot_key(shard_id);
            self.tails[shard_id] = target.oldest_hot_key(shard_id);
            assert_eq!(
                self.base.shards[shard_id].len(),
                target.num_hot_items(shard_id)
            );

            debug_assert!(self.validate_lru(shard_id).is_ok());
        }

        COUNTER.inc_with_by(&["hot_state_insert"], n_insert);
        COUNTER.inc_with_by(&["hot_state_update"], n_update);
        COUNTER.inc_with_by(&["hot_state_evict"], n_evict);
    }

    /// Traverses the entire map and checks if all the pointers are correctly linked.
    fn validate_lru(&self, shard_id: usize) -> Result<()> {
        let head = &self.heads[shard_id];
        let tail = &self.tails[shard_id];
        ensure!(head.is_some() == tail.is_some());
        let shard = &self.base.shards[shard_id];

        {
            let mut num_visited = 0;
            let mut current = head.clone();
            while let Some(key) = current {
                let entry = shard.get(&key).expect("Must exist.");
                num_visited += 1;
                ensure!(num_visited <= shard.len());
                ensure!(entry.is_hot());
                current = entry.next().cloned();
            }
            ensure!(num_visited == shard.len());
        }

        {
            let mut num_visited = 0;
            let mut current = tail.clone();
            while let Some(key) = current {
                let entry = shard.get(&key).expect("Must exist.");
                num_visited += 1;
                ensure!(num_visited <= shard.len());
                ensure!(entry.is_hot());
                current = entry.prev().cloned();
            }
            ensure!(num_visited == shard.len());
        }

        Ok(())
    }
}
