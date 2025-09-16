// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{GAUGE, OTHER_TIMERS_SECONDS};
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_metrics_core::{IntGaugeVecHelper, TimerHelper};
use aptos_storage_interface::state_store::{
    state::State, state_view::hot_state_view::HotStateView,
};
use aptos_types::state_store::{
    hot_state::{HotStateConfig, THotStateSlot},
    state_key::StateKey,
    state_slot::StateSlot,
    NUM_STATE_SHARDS,
};
use arr_macro::arr;
use dashmap::{mapref::one::Ref, DashMap};
#[cfg(test)]
use std::collections::BTreeMap;
use std::{
    sync::{
        mpsc::{Receiver, SyncSender, TryRecvError},
        Arc,
    },
    thread::JoinHandle,
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

    fn insert(&self, key: K, value: V) {
        self.inner.insert(key, value);
    }

    fn remove(&self, key: &K) {
        self.inner.remove(key);
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
pub struct HotStateBase<K = StateKey, V = StateSlot>
where
    K: Eq + std::hash::Hash,
{
    /// After committing a new batch to `inner`, items are evicted so that
    ///  1. total number of items doesn't exceed this number
    #[allow(dead_code)] // TODO(HotState): not used for now
    max_items_per_shard: usize,

    shards: [Shard<K, V>; NUM_STATE_SHARDS],
}

impl<K, V> HotStateBase<K, V>
where
    K: Clone + Eq + std::hash::Hash,
    V: Clone,
{
    fn new_empty(max_items_per_shard: usize) -> Self {
        Self {
            max_items_per_shard,
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

impl HotStateView for HotStateBase<StateKey, StateSlot> {
    fn get_state_slot(&self, state_key: &StateKey) -> Option<StateSlot> {
        let shard_id = state_key.get_shard_id();
        self.get_from_shard(shard_id, state_key).map(|v| v.clone())
    }
}

#[derive(Debug)]
pub struct HotState {
    base: Arc<HotStateBase>,
    committed: Arc<Mutex<State>>,
    commit_tx: Mutex<(Option<SyncSender<State>>, Option<JoinHandle<()>>)>,
}

impl HotState {
    pub fn new(state: State, config: HotStateConfig) -> Self {
        let base = Arc::new(HotStateBase::new_empty(config.max_items_per_shard));
        let committed = Arc::new(Mutex::new(state));
        let (commit_tx, committer_handle) = Committer::spawn(base.clone(), committed.clone());

        Self {
            base,
            committed,
            commit_tx: Mutex::new((Some(commit_tx), Some(committer_handle))),
        }
    }

    pub(crate) fn set_commited(&self, state: State) {
        *self.committed.lock() = state
    }

    pub fn get_committed(&self) -> (Arc<dyn HotStateView>, State) {
        let state = self.committed.lock().clone();
        let base = self.base.clone();

        (base, state)
    }

    pub fn enqueue_commit(&self, to_commit: State) {
        println!(
            "HotState::enqueue_commit. version: {}",
            to_commit.next_version()
        );
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["hot_state_enqueue_commit"]);

        self.commit_tx
            .lock()
            .0
            .as_ref()
            .unwrap()
            .send(to_commit)
            .expect("Failed to queue for hot state commit.")
    }

    #[cfg(test)]
    pub(crate) fn drain_pending_commits(&self) {
        println!("drain pending commits");
        let mut locked = self.commit_tx.lock();
        // Drop the sender.
        locked.0.take().unwrap();
        let handle = locked.1.take().unwrap();
        handle.join().unwrap();
        println!("all drained");
    }

    #[cfg(test)]
    pub(crate) fn get_all_entries(&self) -> [BTreeMap<StateKey, StateSlot>; NUM_STATE_SHARDS] {
        std::array::from_fn(|shard_id| self.base.shards[shard_id].iter().collect())
    }
}

pub struct Committer {
    base: Arc<HotStateBase>,
    committed: Arc<Mutex<State>>,
    rx: Receiver<State>,
    total_key_bytes: usize,
    total_value_bytes: usize,
    /// Points to the newest entry. `None` if empty.
    heads: [Option<StateKey>; NUM_STATE_SHARDS],
    /// Points to the oldest entry. `None` if empty.
    tails: [Option<StateKey>; NUM_STATE_SHARDS],
}

impl Committer {
    fn spawn(
        base: Arc<HotStateBase>,
        committed: Arc<Mutex<State>>,
    ) -> (SyncSender<State>, JoinHandle<()>) {
        let (tx, rx) = std::sync::mpsc::sync_channel(MAX_HOT_STATE_COMMIT_BACKLOG);
        let handle = std::thread::spawn(move || Self::new(base, committed, rx).run());

        (tx, handle)
    }

    fn new(base: Arc<HotStateBase>, committed: Arc<Mutex<State>>, rx: Receiver<State>) -> Self {
        Self {
            base,
            committed,
            rx,
            total_key_bytes: 0,
            total_value_bytes: 0,
            heads: arr![None; 16],
            tails: arr![None; 16],
        }
    }

    fn run(&mut self) {
        info!("HotState committer thread started.");

        while let Some(to_commit) = self.next_to_commit() {
            self.commit(&to_commit);
            *self.committed.lock() = to_commit;

            GAUGE.set_with(&["hot_state_items"], self.base.len() as i64);
            GAUGE.set_with(&["hot_state_key_bytes"], self.total_key_bytes as i64);
            GAUGE.set_with(&["hot_state_value_bytes"], self.total_value_bytes as i64);
        }

        info!("HotState committer quitting.");
    }

    fn next_to_commit(&self) -> Option<State> {
        // blocking receive the first item
        let mut ret = match self.rx.recv() {
            Ok(state) => state,
            Err(_) => {
                return None;
            },
        };

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

    fn commit(&mut self, to_commit: &State) {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["hot_state_commit"]);

        let delta = to_commit.make_delta(&self.committed.lock());

        for shard_id in 0..NUM_STATE_SHARDS {
            let updates: Vec<_> = delta.shards[shard_id].iter().collect();
            for (key, slot) in updates {
                let shard_id = key.get_shard_id();
                if slot.is_hot() {
                    self.base.shards[shard_id].insert(key, slot);
                } else {
                    self.base.shards[shard_id].remove(&key);
                }
            }
            self.heads[shard_id] = delta.latest_hot_key(shard_id);
            self.tails[shard_id] = delta.oldest_hot_key(shard_id);

            self.validate_shard_debug_only(shard_id);
        }
    }

    fn validate_shard_debug_only(&self, shard_id: usize) {
        let head = &self.heads[shard_id];
        let tail = &self.tails[shard_id];
        assert_eq!(head.is_some(), tail.is_some());
        let shard = &self.base.shards[shard_id];

        {
            let mut num_visited = 0;
            let mut current = head.clone();
            while let Some(key) = current {
                let entry = shard.get(&key).unwrap();
                num_visited += 1;
                assert!(num_visited <= shard.len());
                assert!(entry.is_hot());
                current = entry.next().cloned();
            }
            assert_eq!(num_visited, shard.len());
        }

        {
            let mut num_visited = 0;
            let mut current = tail.clone();
            while let Some(key) = current {
                let entry = shard.get(&key).unwrap();
                num_visited += 1;
                assert!(num_visited <= shard.len());
                assert!(entry.is_hot());
                current = entry.prev().cloned();
            }
            assert_eq!(num_visited, shard.len());
        }
    }
}
