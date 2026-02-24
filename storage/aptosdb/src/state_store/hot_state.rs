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
        mpsc::{Receiver, RecvTimeoutError, Sender, SyncSender},
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

/// A composite HotStateView: checks the delta first, falls back to the base DashMaps. The delta
/// covers changes from what's actually in the DashMaps (`merged_state`) to the current committed
/// state. This enables RCU semantics: the new committed state is published immediately via the
/// delta overlay, while DashMap mutations are deferred until all old readers are gone.
struct LayeredHotStateView {
    /// If `Some`, overlay these changes on top of base. If `None`, base is up-to-date.
    delta: Option<StateDelta>,
    base: Arc<HotStateBase>,
}

impl HotStateView for LayeredHotStateView {
    fn get_state_slot(&self, state_key: &StateKey) -> Option<StateSlot> {
        if let Some(delta) = &self.delta {
            if let Some(slot) = delta.get_state_slot(state_key) {
                // Delta says this key changed. If hot, return it. If cold/evicted, return None —
                // do NOT fall through to base, the key was explicitly evicted in committed state.
                return if slot.is_hot() { Some(slot) } else { None };
            }
        }
        // Key not in delta (unchanged) — read from base DashMap.
        let shard_id = state_key.get_shard_id();
        self.base
            .get_from_shard(shard_id, state_key)
            .map(|v| v.clone())
    }
}

enum CommitMsg {
    Commit(State),
    /// Sent by `hack_reset` to synchronously reset the Committer's `merged_state` and `old_views`.
    /// The caller blocks on `ack` until the Committer has finished processing the reset.
    HackReset {
        state: State,
        ack: Sender<()>,
    },
}

/// Bundles the committed `State` with a `HotStateView` that is consistent with it.
struct CommittedSnapshot {
    state: State,
    view: Arc<LayeredHotStateView>,
}

pub struct HotState {
    base: Arc<HotStateBase>,
    committed: Arc<Mutex<CommittedSnapshot>>,
    commit_tx: SyncSender<CommitMsg>,
    /// Updated by the Committer after each successful DashMap merge. Tests use this to wait for
    /// the merge to complete before inspecting DashMaps. Only read by test helpers.
    #[allow(dead_code)]
    merged_version: Arc<AtomicU64>,
}

impl HotState {
    pub fn new(state: State, config: HotStateConfig) -> Self {
        let base = Arc::new(HotStateBase::new_empty(config.max_items_per_shard));
        let view = Arc::new(LayeredHotStateView {
            delta: None,
            base: Arc::clone(&base),
        });
        let committed = Arc::new(Mutex::new(CommittedSnapshot {
            state: state.clone(),
            view,
        }));
        let merged_version = Arc::new(AtomicU64::new(state.next_version()));
        let commit_tx = Committer::spawn(
            Arc::clone(&base),
            Arc::clone(&committed),
            state,
            Arc::clone(&merged_version),
        );

        Self {
            base,
            committed,
            commit_tx,
            merged_version,
        }
    }

    pub(crate) fn hack_reset(&self, state: State) {
        {
            let mut committed = self.committed.lock();
            committed.state = state.clone();
            // Reset view to base-only (no delta). hack_reset is only called when no commits are in
            // flight, so DashMaps and committed state are in sync from the readers' perspective.
            committed.view = Arc::new(LayeredHotStateView {
                delta: None,
                base: Arc::clone(&self.base),
            });
        }
        // Synchronously reset the Committer's merged_state and old_views. Block until processed,
        // so the caller has a hard guarantee that no stale Committer state remains.
        let (ack_tx, ack_rx) = std::sync::mpsc::channel();
        self.commit_tx
            .send(CommitMsg::HackReset { state, ack: ack_tx })
            .expect("Failed to send reset to hot state committer.");
        ack_rx
            .recv()
            .expect("Failed to receive reset ack from hot state committer.");
    }

    pub fn get_committed(&self) -> (Arc<dyn HotStateView>, State) {
        let committed = self.committed.lock();
        (
            Arc::clone(&committed.view) as Arc<dyn HotStateView>,
            committed.state.clone(),
        )
    }

    pub fn enqueue_commit(&self, to_commit: State) {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["hot_state_enqueue_commit"]);

        self.commit_tx
            .send(CommitMsg::Commit(to_commit))
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

/// Background thread that merges committed state into the base DashMaps.
///
/// ```text
///   merged_state ──[delta overlay]──> committed.state <── to_commit (incoming)
///        |                                  |
///   base DashMaps                     what readers see
///   (physical store)                  (via LayeredHotStateView)
/// ```
///
/// The Committer tracks three `State` references:
///
/// - **`merged_state`** — what the base DashMaps currently reflect.
/// - **`committed.state`** — latest state published to readers, possibly ahead of `merged_state`.
///   Readers see it through a `LayeredHotStateView` that overlays
///   `StateDelta(merged_state -> committed)` on the DashMaps.
/// - **incoming `to_commit`** — the next state received from the channel.
///
/// On each incoming state the Committer builds a new `LayeredHotStateView`, atomically swaps it
/// into `committed`, and tracks the old view via `Weak`. The Committer can advance `merged_state`
/// (apply deltas to DashMaps) only when **all** old views have been dropped: each old view's delta
/// assumes DashMaps = `merged_state`, and keys outside the delta fall through to the DashMaps
/// directly. Advancing DashMaps while any such view is live would corrupt those fall-through reads.
///
/// In steady state (no forks, fast readers), `old_views` drains immediately and merges are inline.
/// During a fork or under load, merges are deferred and the Committer retries via `recv_timeout`.
struct Committer {
    base: Arc<HotStateBase>,
    committed: Arc<Mutex<CommittedSnapshot>>,
    rx: Receiver<CommitMsg>,
    total_key_bytes: usize,
    total_value_bytes: usize,
    /// Points to the newest entry. `None` if empty.
    heads: [Option<StateKey>; NUM_STATE_SHARDS],
    /// Points to the oldest entry. `None` if empty.
    tails: [Option<StateKey>; NUM_STATE_SHARDS],

    /// What the base DashMaps currently reflect. May lag behind `committed.state` while a merge
    /// is deferred.
    merged_state: State,

    /// Weak refs to all previously published views. Merge is deferred until every one is dropped
    /// (strong_count == 0). See struct doc.
    old_views: Vec<Weak<LayeredHotStateView>>,

    /// Shared with HotState; updated after each successful DashMap merge.
    merged_version: Arc<AtomicU64>,
}

impl Committer {
    fn spawn(
        base: Arc<HotStateBase>,
        committed: Arc<Mutex<CommittedSnapshot>>,
        initial_state: State,
        merged_version: Arc<AtomicU64>,
    ) -> SyncSender<CommitMsg> {
        let (tx, rx) = std::sync::mpsc::sync_channel(MAX_HOT_STATE_COMMIT_BACKLOG);
        std::thread::Builder::new()
            .name("hotstate-commit".to_string())
            .spawn(move || Self::new(base, committed, rx, initial_state, merged_version).run())
            .expect("Failed to spawn hot state committer thread");

        tx
    }

    fn new(
        base: Arc<HotStateBase>,
        committed: Arc<Mutex<CommittedSnapshot>>,
        rx: Receiver<CommitMsg>,
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
            self.try_merge(); // merge any deferred delta to shrink the next one

            // Skip if DashMaps already reflect this state (unlikely).
            if self.merged_state.is_the_same(&to_commit) {
                warn!(
                    incoming_version = to_commit.next_version(),
                    "Incoming state already merged.",
                );
                continue;
            }

            // If merged_state is too old for to_commit (persisted snapshot advanced
            // while merge was deferred), wait for old views to drain so try_merge
            // can advance merged_state.
            while !self.merged_state.can_be_delta_base_of(&to_commit) {
                if !self.try_merge() {
                    std::thread::sleep(DEFERRED_MERGE_RETRY_INTERVAL);
                }
            }

            let committed_version = to_commit.next_version();

            // Build a layered view: delta(merged_state -> to_commit) over base DashMaps.
            let delta = to_commit.make_delta(&self.merged_state);
            let new_view = Arc::new(LayeredHotStateView {
                delta: Some(delta),
                base: Arc::clone(&self.base),
            });

            // Atomically publish new view + state; track the old view for deferred merge.
            {
                let mut committed = self.committed.lock();
                Self::swap_view(&mut self.old_views, &mut committed, new_view);
                committed.state = to_commit;
            }

            self.try_merge();
            self.update_deferred_merge_gauges(committed_version);
        }

        self.try_merge(); // flush any remaining deferred merge before exit
        info!("HotState committer quitting.");
    }

    /// Process a `HackReset` message: synchronize `merged_state` / `old_views` with the caller and
    /// ack.
    ///
    /// `HackReset` is a hack used by `hack_reset` and is only sent when no commits are in flight,
    /// so it must be the sole message in the channel. `next_to_commit` asserts this before
    /// calling.
    fn handle_reset(&mut self, state: State, ack: Sender<()>) {
        self.merged_state = state;
        self.merged_version
            .store(self.merged_state.next_version(), Ordering::Release);
        self.old_views.clear();
        let _ = ack.send(());
    }

    fn next_to_commit(&mut self) -> Option<State> {
        // Block until we receive the first Commit, retrying merges on timeout.
        // HackReset messages are processed inline — they are only sent when no commits are in
        // flight, so we assert the channel is empty after processing one.
        let first = loop {
            match self.rx.recv_timeout(DEFERRED_MERGE_RETRY_INTERVAL) {
                Ok(CommitMsg::Commit(state)) => break state,
                Ok(CommitMsg::HackReset { state, ack }) => {
                    assert!(
                        self.rx.try_recv().is_err(),
                        "HackReset must be the only message in the channel — \
                         hack_reset is only valid when no commits are in flight."
                    );
                    self.handle_reset(state, ack);
                },
                Err(RecvTimeoutError::Timeout) => {
                    self.try_merge();
                },
                Err(RecvTimeoutError::Disconnected) => return None,
            }
        };

        // Drain backlog — only the latest Commit matters. HackReset must not appear here.
        let mut ret = first;
        let mut n_backlog = 0;
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                CommitMsg::Commit(state) => {
                    n_backlog += 1;
                    ret = state;
                },
                CommitMsg::HackReset { .. } => {
                    unreachable!(
                        "HackReset must not appear alongside Commit messages — \
                         hack_reset is only valid when no commits are in flight."
                    );
                },
            }
        }

        GAUGE.set_with(&["hot_state_commit_backlog"], n_backlog);
        Some(ret)
    }

    /// Replace `committed.view` with `new_view`, tracking the old view via a `Weak` ref so that
    /// `try_merge` defers DashMap mutations while any reader still holds it.
    fn swap_view(
        old_views: &mut Vec<Weak<LayeredHotStateView>>,
        committed: &mut CommittedSnapshot,
        new_view: Arc<LayeredHotStateView>,
    ) {
        let old_view = std::mem::replace(&mut committed.view, new_view);
        old_views.push(Arc::downgrade(&old_view));
    }

    /// Advance `merged_state` toward `committed.state` by applying deltas to the base DashMaps —
    /// but only when all old views have been dropped. After merge, replaces `committed.view` with
    /// a clean (no-delta) view. Readers who already cloned the old delta-bearing view are
    /// unaffected: the delta shadows changed keys, and unchanged keys agree between the delta's
    /// target and the updated DashMaps.
    /// Returns `false` if blocked by lingering old views, `true` otherwise.
    fn try_merge(&mut self) -> bool {
        self.old_views.retain(|v| v.strong_count() > 0);
        if !self.old_views.is_empty() {
            self.update_deferred_merge_gauges(self.committed.lock().state.next_version());
            return false;
        }

        let target = self.committed.lock().state.clone();
        if self.merged_state.is_the_same(&target) {
            return true;
        }

        self.apply_delta_to_base(&target);
        self.merged_state = target;
        self.merged_version
            .store(self.merged_state.next_version(), Ordering::Release);
        info!(
            next_version = self.merged_state.next_version(),
            "Advanced merged_state.",
        );

        // Publish a clean (delta-free) view so future readers hit the DashMaps directly without
        // the overhead of a delta lookup, now that the DashMaps are up to date.
        let clean_view = Arc::new(LayeredHotStateView {
            delta: None,
            base: Arc::clone(&self.base),
        });
        Self::swap_view(&mut self.old_views, &mut self.committed.lock(), clean_view);
        self.update_deferred_merge_gauges(self.merged_state.next_version());

        true
    }

    fn update_deferred_merge_gauges(&self, committed_version: u64) {
        GAUGE.set_with(
            &["hot_state_deferred_merge_old_views"],
            self.old_views.len() as i64,
        );
        GAUGE.set_with(
            &["hot_state_deferred_merge_version_lag"],
            (committed_version - self.merged_state.next_version()) as i64,
        );
    }

    /// Apply the delta between `merged_state` and `target` to the base DashMaps.
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
        GAUGE.set_with(&["hot_state_items"], self.base.len() as i64);
        GAUGE.set_with(&["hot_state_key_bytes"], self.total_key_bytes as i64);
        GAUGE.set_with(&["hot_state_value_bytes"], self.total_value_bytes as i64);
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

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_config::config::HotStateConfig;
    use aptos_experimental_layered_map::MapLayer;
    use aptos_storage_interface::state_store::{state::State, state_delta::StateDelta};
    use aptos_types::state_store::{
        hot_state::LRUEntry, state_key::StateKey, state_slot::StateSlot,
        state_storage_usage::StateStorageUsage, state_value::StateValue,
    };

    const TEST_CONFIG: HotStateConfig = HotStateConfig {
        max_items_per_shard: 100,
        refresh_interval_versions: 100,
        delete_on_restart: false,
        compute_root_hash: true,
    };

    fn make_hot_slot(version: Version, value: &[u8]) -> StateSlot {
        StateSlot::HotOccupied {
            value_version: version,
            value: StateValue::new_legacy(value.to_vec().into()),
            hot_since_version: version,
            lru_info: LRUEntry::uninitialized(),
        }
    }

    fn make_hot_vacant(version: Version) -> StateSlot {
        StateSlot::HotVacant {
            hot_since_version: version,
            lru_info: LRUEntry::uninitialized(),
        }
    }

    /// Create a `StateDelta` for testing `LayeredHotStateView`.
    /// The delta's shards contain exactly the given entries.
    fn make_test_delta(entries: &[(StateKey, StateSlot)]) -> StateDelta {
        let mut shard_entries: [Vec<(StateKey, StateSlot)>; NUM_STATE_SHARDS] =
            std::array::from_fn(|_| Vec::new());
        for (key, slot) in entries {
            shard_entries[key.get_shard_id()].push((key.clone(), slot.clone()));
        }

        let empty = State::new_empty(TEST_CONFIG);
        let shards = std::array::from_fn(|shard_id| {
            let base = &empty.shards()[shard_id];
            let child = base
                .view_layers_after(base)
                .new_layer(&shard_entries[shard_id]);
            child.view_layers_after(base)
        });

        StateDelta {
            base: empty.clone(),
            current: empty,
            shards: Arc::new(shards),
        }
    }

    /// Create an empty `State` that is a descendant of `parent` at the given version. `root` must
    /// be the original ancestor — it's used as the base layer when spawning children so that
    /// `make_delta(root)` remains valid for all descendants.
    fn build_empty_descendant(root: &State, parent: &State, version: Version) -> State {
        let shards: [MapLayer<StateKey, StateSlot>; NUM_STATE_SHARDS] =
            std::array::from_fn(|shard_id| {
                parent.shards()[shard_id]
                    .view_layers_after(&root.shards()[shard_id])
                    .new_layer(&[])
            });
        State::new_with_updates(
            Some(version),
            Arc::new(shards),
            Default::default(),
            StateStorageUsage::zero(),
            TEST_CONFIG,
        )
    }

    // ===== LayeredHotStateView tests =====

    #[test]
    fn test_layered_view_no_delta() {
        let base = Arc::new(HotStateBase::new_empty(100));
        let key = StateKey::raw(b"key_a");
        let shard_id = key.get_shard_id();
        let slot = make_hot_slot(1, b"value_a");
        base.shards[shard_id].insert(key.clone(), slot.clone());

        let view = LayeredHotStateView {
            delta: None,
            base: Arc::clone(&base),
        };

        // Key in base -> returns base value.
        let result = view.get_state_slot(&key);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().as_state_value_opt(),
            slot.as_state_value_opt()
        );

        // Key not in base -> returns None.
        let missing = StateKey::raw(b"missing");
        assert!(view.get_state_slot(&missing).is_none());
    }

    #[test]
    fn test_layered_view_with_delta() {
        let base = Arc::new(HotStateBase::new_empty(100));

        // key_base_only: in base DashMap, NOT in delta.
        let key_base_only = StateKey::raw(b"base_only");
        let slot_base = make_hot_slot(1, b"base_value");
        base.shards[key_base_only.get_shard_id()].insert(key_base_only.clone(), slot_base.clone());

        // key_updated: in base DashMap AND in delta (hot) -> delta wins.
        let key_updated = StateKey::raw(b"updated");
        let slot_old = make_hot_slot(1, b"old_value");
        let slot_new = make_hot_slot(2, b"new_value");
        base.shards[key_updated.get_shard_id()].insert(key_updated.clone(), slot_old);

        // key_evicted: in base DashMap AND in delta (cold) -> returns None.
        let key_evicted = StateKey::raw(b"evicted");
        let slot_was_hot = make_hot_slot(1, b"was_hot");
        base.shards[key_evicted.get_shard_id()].insert(key_evicted.clone(), slot_was_hot);

        // key_new: NOT in base, in delta (hot) -> returns delta value.
        let key_new = StateKey::raw(b"new_key");
        let slot_new_key = make_hot_slot(2, b"brand_new");

        // key_missing: NOT in base, NOT in delta -> returns None.
        let key_missing = StateKey::raw(b"missing");

        let delta = make_test_delta(&[
            (key_updated.clone(), slot_new.clone()),
            (key_evicted.clone(), StateSlot::ColdVacant),
            (key_new.clone(), slot_new_key.clone()),
        ]);

        let view = LayeredHotStateView {
            delta: Some(delta),
            base,
        };

        // Key only in base -> falls through to base.
        let result = view.get_state_slot(&key_base_only);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().as_state_value_opt(),
            slot_base.as_state_value_opt()
        );

        // Key updated in delta -> returns delta value.
        let result = view.get_state_slot(&key_updated);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().as_state_value_opt(),
            slot_new.as_state_value_opt()
        );

        // Key evicted in delta -> returns None (even though in base).
        assert!(view.get_state_slot(&key_evicted).is_none());

        // New key in delta -> returns delta value.
        let result = view.get_state_slot(&key_new);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().as_state_value_opt(),
            slot_new_key.as_state_value_opt()
        );

        // Key neither in delta nor base -> returns None.
        assert!(view.get_state_slot(&key_missing).is_none());
    }

    #[test]
    fn test_layered_view_hot_vacant_in_delta() {
        // HotVacant in delta -> is_hot() is true -> returns Some(HotVacant).
        let base = Arc::new(HotStateBase::new_empty(100));
        let key = StateKey::raw(b"hot_vacant");
        let slot = make_hot_vacant(5);

        let delta = make_test_delta(&[(key.clone(), slot)]);
        let view = LayeredHotStateView {
            delta: Some(delta),
            base,
        };

        let result = view.get_state_slot(&key);
        assert!(result.is_some());
        assert!(result.unwrap().is_hot());
    }

    // ===== Deferred merge tests =====

    /// Helper: wait for `get_committed()` to return a state at the given version.
    fn wait_for_committed_version(hot_state: &HotState, target_next_version: Version) {
        loop {
            let (_, committed) = hot_state.get_committed();
            if committed.next_version() >= target_next_version {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn test_deferred_merge_basic() {
        let state0 = State::new_empty(TEST_CONFIG);
        let hot_state = HotState::new(state0.clone(), TEST_CONFIG);

        let state1 = build_empty_descendant(&state0, &state0, 0);
        hot_state.enqueue_commit(state1);
        hot_state.wait_for_merge(1);

        let (_, committed) = hot_state.get_committed();
        assert_eq!(committed.next_version(), 1);
    }

    #[test]
    fn test_deferred_merge_with_lingering_reader() {
        let state0 = State::new_empty(TEST_CONFIG);
        let hot_state = HotState::new(state0.clone(), TEST_CONFIG);

        // Grab a view — this reader holds a strong ref to the initial view.
        let (held_view, _) = hot_state.get_committed();

        // Enqueue two commits.
        let state1 = build_empty_descendant(&state0, &state0, 0);
        let state2 = build_empty_descendant(&state0, &state1, 1);
        hot_state.enqueue_commit(state1);
        hot_state.enqueue_commit(state2);

        // Wait for the Committer to process the commits (view swap).
        wait_for_committed_version(&hot_state, 2);

        // Give the Committer time to attempt try_merge (should fail because we hold the old view).
        std::thread::sleep(Duration::from_millis(100));

        // Merge should NOT have happened yet.
        assert!(hot_state.merged_version.load(Ordering::Acquire) < 2);

        // Drop the old view.
        drop(held_view);

        // Now merge should proceed.
        hot_state.wait_for_merge(2);
        assert_eq!(hot_state.merged_version.load(Ordering::Acquire), 2);
    }

    /// Regression test: `try_merge()` must track the delta view it replaces with a clean view.
    ///
    /// Without tracking, a reader holding that delta view becomes invisible to the Committer.
    /// A subsequent merge advances the DashMaps past the view's target state, corrupting
    /// fall-through reads for keys not covered by the delta.
    ///
    /// Timeline (states are empty so deltas are no-ops, but tracking still matters):
    ///
    /// ```text
    ///   r0 holds V0_clean ──► blocks merge of S1
    ///   S1 committed        ──► V1_delta published
    ///   r1 grabs V1_delta
    ///   drop(r0)            ──► merge to S1 proceeds, try_merge replaces V1_delta with V1_clean
    ///   S2 committed        ──► V2_delta published
    ///   assert: merge to S2 blocked by r1 (V1_delta must be tracked)
    /// ```
    #[test]
    fn test_try_merge_tracks_replaced_view() {
        let state0 = State::new_empty(TEST_CONFIG);
        let hot_state = HotState::new(state0.clone(), TEST_CONFIG);

        // Hold the initial view (V0_clean). This blocks merge while S1 is committed,
        // keeping V1_delta in committed.view long enough for us to grab it.
        let (r0_view, _) = hot_state.get_committed();

        // Commit S1. The Committer publishes V1_delta and tracks Weak(V0_clean).
        // Merge is deferred because r0_view holds V0_clean.
        let state1 = build_empty_descendant(&state0, &state0, 0);
        hot_state.enqueue_commit(state1.clone());
        wait_for_committed_version(&hot_state, 1);

        // Grab V1_delta — the delta view currently in committed.view.
        let (r1_view, r1_state) = hot_state.get_committed();
        assert_eq!(r1_state.next_version(), 1);

        // Drop r0_view: unblocks merge to S1. The Committer's try_merge() will:
        //   1. Apply delta S0→S1 to DashMaps (no-op for empty states)
        //   2. Replace V1_delta with a clean V1_clean via swap_view
        //   3. swap_view pushes Weak(V1_delta) into old_views
        drop(r0_view);
        hot_state.wait_for_merge(1);

        // Commit S2. The Committer publishes V2_delta and tracks Weak(V1_clean).
        let state2 = build_empty_descendant(&state0, &state1, 1);
        hot_state.enqueue_commit(state2);
        wait_for_committed_version(&hot_state, 2);

        // Give the Committer time to attempt try_merge for S2.
        // V1_delta is tracked in old_views (r1_view holds it), so merge must be deferred.
        std::thread::sleep(Duration::from_millis(100));
        assert!(
            hot_state.merged_version.load(Ordering::Acquire) < 2,
            "DashMaps must not advance past S1 while a reader holds V1_delta"
        );

        // Drop r1_view. Now merge to S2 should proceed.
        drop(r1_view);
        hot_state.wait_for_merge(2);
        assert_eq!(hot_state.merged_version.load(Ordering::Acquire), 2);
    }

    /// Regression test: the committer must not crash when `merged_state` lags behind
    /// the `base_layer` of an incoming `to_commit`.
    ///
    /// In production, `State::update()` spawns new `MapLayer` shards with
    /// `base_layer = persisted_snapshot.layer()`. When old readers prevent
    /// `try_merge()` from advancing `merged_state`, and `persisted_snapshot`
    /// has advanced, the committer's `merged_state` can be at a lower layer than
    /// `to_commit.base_layer`. The fix makes the committer wait for merge before
    /// building the delta.
    ///
    /// Timeline:
    /// ```text
    ///   r0 holds V0_clean ──► blocks all merges (merged_state stuck at S0)
    ///   S1 committed (base_layer=0) ──► delta(S0→S1) OK
    ///   S2 committed (base_layer=1) ──► delta(S0→S2) would crash without fix
    ///   drop(r0) ──► merge to S1 proceeds, then delta(S1→S2) OK
    /// ```
    #[test]
    fn test_commit_with_advanced_base_layer() {
        let state0 = State::new_empty(TEST_CONFIG);
        let hot_state = HotState::new(state0.clone(), TEST_CONFIG);

        // Hold a view to block all merges (merged_state stays at S0).
        let (held_view, _) = hot_state.get_committed();

        // S1: spawned with root=S0, so base_layer = S0.layer = 0.
        // Compatible with merged_state at S0.
        let state1 = build_empty_descendant(&state0, &state0, 0);
        hot_state.enqueue_commit(state1.clone());
        wait_for_committed_version(&hot_state, 1);

        // S2: spawned with root=S1, simulating persisted_snapshot advancing to S1.
        // This gives base_layer = S1.layer = 1, incompatible with merged_state
        // still at S0 (layer 0). Without the fix this panics.
        let state2 = build_empty_descendant(&state1, &state1, 1);
        hot_state.enqueue_commit(state2);

        // Give the committer time to pick up S2 and hit the incompatible
        // merged_state. Without the fix, this would panic.
        std::thread::sleep(Duration::from_millis(100));

        // Drop the held view so the committer can merge S0→S1, then process S2.
        drop(held_view);

        hot_state.wait_for_merge(2);
        let (_, committed) = hot_state.get_committed();
        assert_eq!(committed.next_version(), 2);
    }

    #[test]
    fn test_rapid_commits_with_lingering_reader() {
        let state0 = State::new_empty(TEST_CONFIG);
        let hot_state = HotState::new(state0.clone(), TEST_CONFIG);

        // Grab a view.
        let (held_view, _) = hot_state.get_committed();

        // Rapidly enqueue many commits.
        let mut parent = state0.clone();
        for v in 0..5 {
            let child = build_empty_descendant(&state0, &parent, v);
            hot_state.enqueue_commit(child.clone());
            parent = child;
        }

        // Wait for committed version to advance to the last enqueued state.
        wait_for_committed_version(&hot_state, 5);

        // get_committed() returns the latest state even while merge is deferred.
        let (_, committed) = hot_state.get_committed();
        assert_eq!(committed.next_version(), 5);

        // Merge deferred because old view is held.
        std::thread::sleep(Duration::from_millis(100));
        assert!(hot_state.merged_version.load(Ordering::Acquire) < 5);

        // Drop old view -> merge should proceed.
        drop(held_view);
        hot_state.wait_for_merge(5);
        assert_eq!(hot_state.merged_version.load(Ordering::Acquire), 5);
    }
}
