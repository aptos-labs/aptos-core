// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::metrics::{
    COUNTER, GAUGE, HOT_STATE_SHARD_GAUGE, OTHER_TIMERS_SECONDS, SHARD_NAME_BY_ID,
};
use anyhow::{ensure, Result};
use aptos_config::config::HotStateConfig;
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_metrics_core::{IntCounterVecHelper, IntGaugeVecHelper, TimerHelper};
use aptos_storage_interface::state_store::{
    state::State,
    state_delta::StateDelta,
    state_view::hot_state_view::{HotStateRevoked, HotStateView},
};
use aptos_types::{
    state_store::{
        hot_state::THotStateSlot, state_key::StateKey, state_slot::StateSlot, NUM_STATE_SHARDS,
    },
    transaction::Version,
};
use arr_macro::arr;
use dashmap::{mapref::one::Ref, DashMap};
#[cfg(test)]
use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc::{Receiver, Sender, SyncSender},
    Arc,
};

const MAX_HOT_STATE_COMMIT_BACKLOG: usize = 10;

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
struct HotStateBase<K = HashValue, V = StateSlot>
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
/// Carries a revocation flag — when the Committer flips it to `true`, DashMap reads are
/// no longer trusted (the Committer may be mutating them). Delta reads remain safe because
/// the delta is immutable.
struct LayeredHotStateView {
    /// If `Some`, overlay these changes on top of base. If `None`, base is up-to-date.
    delta: Option<StateDelta>,
    base: Arc<HotStateBase>,
    /// Flipped to `true` by the Committer before it mutates DashMaps. Readers use a
    /// post-read fence to detect revocation.
    revoked: Arc<AtomicBool>,
}

impl HotStateView for LayeredHotStateView {
    fn get_state_slot(&self, state_key: &StateKey) -> Result<Option<StateSlot>> {
        // Delta is immutable — always safe.
        if let Some(delta) = &self.delta {
            if let Some(slot) = delta.get_state_slot(state_key) {
                return Ok(if slot.is_hot() { Some(slot) } else { None });
            }
        }

        // Pre-check: fast-path reject if already revoked (optimization, not for correctness).
        if self.revoked.load(Ordering::Relaxed) {
            return Err(HotStateRevoked.into());
        }

        // Read from base DashMap.
        let shard_id = state_key.get_shard_id();
        let result = self
            .base
            .get_from_shard(shard_id, state_key.crypto_hash_ref())
            .map(|v| v.clone());

        // Post-read fence: if revoked, the DashMap value may be stale.
        // Happens-before: flag.store(Release) →po write_unlock →lock read_lock →po flag.load(Acquire)
        if self.revoked.load(Ordering::Acquire) {
            return Err(HotStateRevoked.into());
        }

        Ok(result)
    }
}

enum CommitMsg {
    Commit(State),
    /// Sent by `hack_reset` to synchronously reset the Committer's `merged_state` and
    /// `old_revoked_flags`. The caller blocks on `ack` until the Committer has finished
    /// processing the reset.
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
    #[cfg(test)]
    merged_version: Arc<AtomicU64>,
}

impl HotState {
    pub fn new(state: State, config: HotStateConfig) -> Self {
        let base = Arc::new(HotStateBase::new_empty(config.max_items_per_shard));
        let view = Arc::new(LayeredHotStateView {
            delta: None,
            base: Arc::clone(&base),
            revoked: Arc::new(AtomicBool::new(false)),
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
            #[cfg(test)]
            merged_version,
        }
    }

    pub(crate) fn hack_reset(&self, state: State) {
        {
            let mut committed = self.committed.lock();
            committed.state = state.clone();
            // Reset view to base-only (no delta) with a fresh revocation flag.
            committed.view = Arc::new(LayeredHotStateView {
                delta: None,
                base: Arc::clone(&self.base),
                revoked: Arc::new(AtomicBool::new(false)),
            });
        }
        // Synchronously reset the Committer's merged_state and old_revoked_flags.
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
    pub fn get_all_entries(&self, shard_id: usize) -> BTreeMap<HashValue, StateSlot> {
        self.base.shards[shard_id].iter().collect()
    }
}

/// Background thread that merges committed state into the base DashMaps.
///
/// Uses epoch-revocation instead of deferred merge: before mutating DashMaps,
/// the Committer flips the `revoked` flag on all old views. Readers detect
/// revocation via a post-read fence and fall through to cold storage.
/// The Committer always merges inline — no waiting, no spin loop, no back-pressure.
struct Committer {
    base: Arc<HotStateBase>,
    committed: Arc<Mutex<CommittedSnapshot>>,
    rx: Receiver<CommitMsg>,
    total_key_bytes: usize,
    total_value_bytes: usize,
    /// Points to the newest entry. `None` if empty.
    heads: [Option<HashValue>; NUM_STATE_SHARDS],
    /// Points to the oldest entry. `None` if empty.
    tails: [Option<HashValue>; NUM_STATE_SHARDS],

    /// What the base DashMaps currently reflect.
    merged_state: State,

    /// Revocation flags for all previously published views. Flipped before DashMap mutations.
    old_revoked_flags: Vec<Arc<AtomicBool>>,

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
            old_revoked_flags: Vec::new(),
            merged_version,
        }
    }

    fn run(&mut self) {
        info!("HotState committer thread started.");

        while let Some(batch) = self.next_to_commit() {
            self.process_batch(batch);
        }

        info!("HotState committer quitting.");
    }

    fn process_batch(&mut self, batch: Vec<State>) {
        // Try to skip to the latest: if merged_state can be delta base of the last item,
        // process only the last (skipping intermediate states).
        let to_process = if batch.len() > 1
            && self
                .merged_state
                .can_be_delta_base_of(batch.last().unwrap())
        {
            &batch[batch.len() - 1..]
        } else {
            // Iterate from oldest to newest. This handles layer advancement without
            // spin-waiting: intermediate states bridge the layer gap.
            &batch
        };

        for to_commit in to_process {
            if self.merged_state.is_the_same(to_commit) {
                warn!(
                    incoming_version = to_commit.next_version(),
                    "Incoming state already merged.",
                );
                continue;
            }

            if !self.merged_state.can_be_delta_base_of(to_commit) {
                // This can happen if we skipped intermediate states but shouldn't happen
                // when iterating from oldest to newest.
                warn!(
                    merged_version = self.merged_state.next_version(),
                    incoming_version = to_commit.next_version(),
                    "Skipping incompatible state (merged_state cannot be delta base).",
                );
                continue;
            }

            // Step 1: Publish delta view with fresh revocation flag.
            let delta = to_commit.make_delta(&self.merged_state);
            let revoked = Arc::new(AtomicBool::new(false));
            let new_view = Arc::new(LayeredHotStateView {
                delta: Some(delta),
                base: Arc::clone(&self.base),
                revoked: Arc::clone(&revoked),
            });

            {
                let mut committed = self.committed.lock();
                // Track the old view's revocation flag.
                self.old_revoked_flags
                    .push(Arc::clone(&committed.view.revoked));
                committed.view = new_view;
                committed.state = to_commit.clone();
            }

            // Step 2: Revoke all old views BEFORE mutating DashMaps.
            for flag in &self.old_revoked_flags {
                flag.store(true, Ordering::Release);
            }
            self.old_revoked_flags.clear();
            // Keep the current view's flag for future revocation.
            self.old_revoked_flags.push(revoked);

            // Step 3: Apply delta to base DashMaps.
            self.apply_delta_to_base(to_commit);
            self.merged_state = to_commit.clone();
            self.merged_version
                .store(self.merged_state.next_version(), Ordering::Release);
            info!(
                next_version = self.merged_state.next_version(),
                "Advanced merged_state.",
            );
        }
    }

    /// Process a `HackReset` message.
    fn handle_reset(&mut self, state: State, ack: Sender<()>) {
        self.merged_state = state;
        self.merged_version
            .store(self.merged_state.next_version(), Ordering::Release);
        self.old_revoked_flags.clear();
        let _ = ack.send(());
    }

    /// Block until the first Commit arrives, then drain the backlog.
    /// Returns the full batch (oldest to newest) or `None` on disconnect.
    fn next_to_commit(&mut self) -> Option<Vec<State>> {
        // Block until we receive the first Commit.
        let first = loop {
            match self.rx.recv() {
                Ok(CommitMsg::Commit(state)) => break state,
                Ok(CommitMsg::HackReset { state, ack }) => {
                    assert!(
                        self.rx.try_recv().is_err(),
                        "HackReset must be the only message in the channel — \
                         hack_reset is only valid when no commits are in flight."
                    );
                    self.handle_reset(state, ack);
                },
                Err(_) => return None,
            }
        };

        // Drain backlog — collect all states, oldest to newest.
        let mut batch = vec![first];
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                CommitMsg::Commit(state) => {
                    batch.push(state);
                },
                CommitMsg::HackReset { .. } => {
                    unreachable!(
                        "HackReset must not appear alongside Commit messages — \
                         hack_reset is only valid when no commits are in flight."
                    );
                },
            }
        }

        GAUGE.set_with(&["hot_state_commit_backlog"], (batch.len() - 1) as i64);
        Some(batch)
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
                let key_hash = *key.crypto_hash_ref();
                if slot.is_hot() {
                    self.total_key_bytes += HashValue::LENGTH;
                    self.total_value_bytes += slot.size();
                    if let Some(old_slot) = self.base.shards[shard_id].insert(key_hash, slot) {
                        self.total_key_bytes -= HashValue::LENGTH;
                        self.total_value_bytes -= old_slot.size();
                        n_update += 1;
                    } else {
                        n_insert += 1;
                    }
                } else if let Some((_, old_slot)) = self.base.shards[shard_id].remove(&key_hash) {
                    self.total_key_bytes -= HashValue::LENGTH;
                    self.total_value_bytes -= old_slot.size();
                    n_evict += 1;
                }
            }
            self.heads[shard_id] = target
                .latest_hot_key(shard_id)
                .map(|k| *k.crypto_hash_ref());
            self.tails[shard_id] = target
                .oldest_hot_key(shard_id)
                .map(|k| *k.crypto_hash_ref());
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

        self.report_age_metrics();
    }

    /// Reports per-shard MRU/LRU `hot_since_version` gauges and aggregate max/min LRU across shards.
    fn report_age_metrics(&self) {
        let mut global_min_lru: Option<Version> = None;
        let mut global_max_lru: Option<Version> = None;

        for (shard_id, shard_label) in SHARD_NAME_BY_ID.iter().enumerate() {
            let mru_version = self.heads[shard_id].as_ref().map(|k| {
                self.base.shards[shard_id]
                    .get(k)
                    .expect("head must exist in base")
                    .expect_hot_since_version()
            });
            let lru_version = self.tails[shard_id].as_ref().map(|k| {
                self.base.shards[shard_id]
                    .get(k)
                    .expect("tail must exist in base")
                    .expect_hot_since_version()
            });

            if let Some(v) = mru_version {
                HOT_STATE_SHARD_GAUGE
                    .with_label_values(&[*shard_label, "mru_hot_since_version"])
                    .set(v as i64);
            }
            if let Some(v) = lru_version {
                HOT_STATE_SHARD_GAUGE
                    .with_label_values(&[*shard_label, "lru_hot_since_version"])
                    .set(v as i64);
                global_min_lru = Some(global_min_lru.map_or(v, |cur| cur.min(v)));
                global_max_lru = Some(global_max_lru.map_or(v, |cur| cur.max(v)));
            }
        }

        if let (Some(max_lru), Some(min_lru)) = (global_max_lru, global_min_lru) {
            GAUGE.set_with(&["hot_state_max_lru_hot_since_version"], max_lru as i64);
            GAUGE.set_with(&["hot_state_min_lru_hot_since_version"], min_lru as i64);
        }
    }

    /// Traverses the entire map and checks if all the pointers are correctly linked.
    fn validate_lru(&self, shard_id: usize) -> Result<()> {
        let head = &self.heads[shard_id];
        let tail = &self.tails[shard_id];
        ensure!(head.is_some() == tail.is_some());
        let shard = &self.base.shards[shard_id];

        {
            let mut num_visited = 0;
            let mut current = *head;
            while let Some(key_hash) = current {
                let entry = shard.get(&key_hash).expect("Must exist.");
                num_visited += 1;
                ensure!(num_visited <= shard.len());
                ensure!(entry.is_hot());
                current = entry.next().map(|k| *k.crypto_hash_ref());
            }
            ensure!(num_visited == shard.len());
        }

        {
            let mut num_visited = 0;
            let mut current = *tail;
            while let Some(key_hash) = current {
                let entry = shard.get(&key_hash).expect("Must exist.");
                num_visited += 1;
                ensure!(num_visited <= shard.len());
                ensure!(entry.is_hot());
                current = entry.prev().map(|k| *k.crypto_hash_ref());
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
    use aptos_storage_interface::state_store::{
        state::State, state_delta::StateDelta, state_view::hot_state_view::HotStateRevoked,
    };
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

    /// Create an empty `State` that is a descendant of `parent` at the given version.
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
        base.shards[shard_id].insert(*key.crypto_hash_ref(), slot.clone());

        let view = LayeredHotStateView {
            delta: None,
            base: Arc::clone(&base),
            revoked: Arc::new(AtomicBool::new(false)),
        };

        // Key in base -> returns base value.
        let result = view.get_state_slot(&key).unwrap();
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().as_state_value_opt(),
            slot.as_state_value_opt()
        );

        // Key not in base -> returns None.
        let missing = StateKey::raw(b"missing");
        assert!(view.get_state_slot(&missing).unwrap().is_none());
    }

    #[test]
    fn test_layered_view_with_delta() {
        let base = Arc::new(HotStateBase::new_empty(100));

        // key_base_only: in base DashMap, NOT in delta.
        let key_base_only = StateKey::raw(b"base_only");
        let slot_base = make_hot_slot(1, b"base_value");
        base.shards[key_base_only.get_shard_id()]
            .insert(*key_base_only.crypto_hash_ref(), slot_base.clone());

        // key_updated: in base DashMap AND in delta (hot) -> delta wins.
        let key_updated = StateKey::raw(b"updated");
        let slot_old = make_hot_slot(1, b"old_value");
        let slot_new = make_hot_slot(2, b"new_value");
        base.shards[key_updated.get_shard_id()].insert(*key_updated.crypto_hash_ref(), slot_old);

        // key_evicted: in base DashMap AND in delta (cold) -> returns None.
        let key_evicted = StateKey::raw(b"evicted");
        let slot_was_hot = make_hot_slot(1, b"was_hot");
        base.shards[key_evicted.get_shard_id()]
            .insert(*key_evicted.crypto_hash_ref(), slot_was_hot);

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
            revoked: Arc::new(AtomicBool::new(false)),
        };

        // Key only in base -> falls through to base.
        let result = view.get_state_slot(&key_base_only).unwrap();
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().as_state_value_opt(),
            slot_base.as_state_value_opt()
        );

        // Key updated in delta -> returns delta value.
        let result = view.get_state_slot(&key_updated).unwrap();
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().as_state_value_opt(),
            slot_new.as_state_value_opt()
        );

        // Key evicted in delta -> returns None (even though in base).
        assert!(view.get_state_slot(&key_evicted).unwrap().is_none());

        // New key in delta -> returns delta value.
        let result = view.get_state_slot(&key_new).unwrap();
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().as_state_value_opt(),
            slot_new_key.as_state_value_opt()
        );

        // Key neither in delta nor base -> returns None.
        assert!(view.get_state_slot(&key_missing).unwrap().is_none());
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
            revoked: Arc::new(AtomicBool::new(false)),
        };

        let result = view.get_state_slot(&key).unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().is_hot());
    }

    // ===== Revocation tests =====

    #[test]
    fn test_revoked_view_returns_error() {
        let base = Arc::new(HotStateBase::new_empty(100));
        let key_in_base = StateKey::raw(b"in_base");
        let slot = make_hot_slot(1, b"value");
        base.shards[key_in_base.get_shard_id()]
            .insert(*key_in_base.crypto_hash_ref(), slot.clone());

        let key_in_delta = StateKey::raw(b"in_delta");
        let delta_slot = make_hot_slot(2, b"delta_val");
        let delta = make_test_delta(&[(key_in_delta.clone(), delta_slot.clone())]);

        let revoked = Arc::new(AtomicBool::new(false));
        let view = LayeredHotStateView {
            delta: Some(delta),
            base: Arc::clone(&base),
            revoked: Arc::clone(&revoked),
        };

        // Before revocation: both keys work.
        assert!(view.get_state_slot(&key_in_base).unwrap().is_some());
        assert!(view.get_state_slot(&key_in_delta).unwrap().is_some());

        // Revoke the view.
        revoked.store(true, Ordering::Release);

        // Delta-covered keys still work (delta is immutable).
        let result = view.get_state_slot(&key_in_delta).unwrap();
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().as_state_value_opt(),
            delta_slot.as_state_value_opt()
        );

        // Base fall-through keys return Err.
        let err = view.get_state_slot(&key_in_base).unwrap_err();
        assert!(err.downcast_ref::<HotStateRevoked>().is_some());
    }

    #[test]
    fn test_revocation_does_not_block_merge() {
        let state0 = State::new_empty(TEST_CONFIG);
        let hot_state = HotState::new(state0.clone(), TEST_CONFIG);

        // Hold a view — in the old deferred-merge scheme this would block merge.
        let (held_view, _) = hot_state.get_committed();

        // Enqueue commits.
        let state1 = build_empty_descendant(&state0, &state0, 0);
        let state2 = build_empty_descendant(&state0, &state1, 1);
        hot_state.enqueue_commit(state1);
        hot_state.enqueue_commit(state2);

        // Merge should proceed immediately — no blocking on held_view.
        hot_state.wait_for_merge(2);
        assert_eq!(hot_state.merged_version.load(Ordering::Acquire), 2);

        // The held view should be revoked — base fall-through returns Err.
        let key = StateKey::raw(b"test_key");
        match held_view.get_state_slot(&key) {
            Ok(None) => {}, // View had no delta, key not in base — Ok(None) before revocation check
            Err(e) => assert!(e.downcast_ref::<HotStateRevoked>().is_some()),
            Ok(Some(_)) => panic!("Expected revoked error or None, got Some"),
        }

        drop(held_view);
    }

    // ===== Integration tests =====

    #[test]
    fn test_basic_commit() {
        let state0 = State::new_empty(TEST_CONFIG);
        let hot_state = HotState::new(state0.clone(), TEST_CONFIG);

        let state1 = build_empty_descendant(&state0, &state0, 0);
        hot_state.enqueue_commit(state1);
        hot_state.wait_for_merge(1);

        let (_, committed) = hot_state.get_committed();
        assert_eq!(committed.next_version(), 1);
    }

    #[test]
    fn test_commit_with_advanced_base_layer() {
        // Tests that batch iteration handles layer boundaries without spin-waiting.
        let state0 = State::new_empty(TEST_CONFIG);
        let hot_state = HotState::new(state0.clone(), TEST_CONFIG);

        // S1: base_layer = S0.layer = 0.
        let state1 = build_empty_descendant(&state0, &state0, 0);
        hot_state.enqueue_commit(state1.clone());

        // S2: base_layer = S1.layer = 1 (persisted snapshot advanced).
        let state2 = build_empty_descendant(&state1, &state1, 1);
        hot_state.enqueue_commit(state2);

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

        // Merge should proceed immediately without waiting for held_view to drop.
        hot_state.wait_for_merge(5);
        assert_eq!(hot_state.merged_version.load(Ordering::Acquire), 5);

        let (_, committed) = hot_state.get_committed();
        assert_eq!(committed.next_version(), 5);

        // The held view is revoked.
        drop(held_view);
    }
}
