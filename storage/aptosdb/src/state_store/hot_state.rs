// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{COUNTER, GAUGE, OTHER_TIMERS_SECONDS};
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_metrics_core::{IntCounterHelper, IntGaugeHelper, TimerHelper};
use aptos_storage_interface::state_store::{
    state::State, state_view::hot_state_view::HotStateView,
};
use aptos_types::state_store::{
    hot_state::THotStateSlot, state_key::StateKey, state_slot::StateSlot, NUM_STATE_SHARDS,
};
use arr_macro::arr;
use dashmap::{
    mapref::one::{Ref, RefMut},
    DashMap,
};
use std::sync::{
    mpsc::{Receiver, SyncSender, TryRecvError},
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
    K: Eq + std::hash::Hash,
{
    fn new(max_items: usize) -> Self {
        Self {
            inner: DashMap::with_capacity(max_items),
        }
    }

    fn contains_key(&self, key: &K) -> bool {
        self.inner.contains_key(key)
    }

    fn get(&self, key: &K) -> Option<Ref<K, V>> {
        self.inner.get(key)
    }

    fn get_mut(&self, key: &K) -> Option<RefMut<K, V>> {
        self.inner.get_mut(key)
    }

    fn insert(&self, key: K, value: V) {
        self.inner.insert(key, value);
    }

    fn remove(&self, key: &K) -> Option<(K, V)> {
        self.inner.remove(key)
    }

    fn len(&self) -> usize {
        self.inner.len()
    }
}

#[derive(Debug)]
pub struct HotStateBase<K = StateKey, V = StateSlot>
where
    K: Eq + std::hash::Hash,
{
    /// After committing a new batch to `inner`, items are evicted so that
    ///  1. total number of items doesn't exceed this number
    max_items_per_shard: usize,
    ///  2. total number of bytes, incl. both keys and values doesn't exceed this number
    #[allow(dead_code)] // TODO(HotState): not enforced for now
    max_bytes_per_shard: usize,
    /// No item is accepted to `inner` if the size of the value exceeds this number
    #[allow(dead_code)] // TODO(HotState): not enforced for now
    max_single_value_bytes: usize,

    shards: [Shard<K, V>; NUM_STATE_SHARDS],
}

impl<K, V> HotStateBase<K, V>
where
    K: Eq + std::hash::Hash,
    V: Clone,
{
    fn new_empty(
        max_items_per_shard: usize,
        max_bytes_per_shard: usize,
        max_single_value_bytes: usize,
    ) -> Self {
        Self {
            max_items_per_shard,
            max_bytes_per_shard,
            max_single_value_bytes,
            shards: arr![Shard::new(max_items_per_shard); 16],
        }
    }

    fn get_from_shard(&self, shard_id: usize, key: &K) -> Option<Ref<K, V>> {
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
    commit_tx: SyncSender<State>,
}

impl HotState {
    pub fn new(
        state: State,
        max_items_per_shard: usize,
        max_bytes_per_shard: usize,
        max_single_value_bytes: usize,
    ) -> Self {
        let base = Arc::new(HotStateBase::new_empty(
            max_items_per_shard,
            max_bytes_per_shard,
            max_single_value_bytes,
        ));
        let committed = Arc::new(Mutex::new(state));
        let commit_tx = Committer::spawn(base.clone(), committed.clone());

        Self {
            base,
            committed,
            commit_tx,
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
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["hot_state_enqueue_commit"]);

        self.commit_tx
            .send(to_commit)
            .expect("Failed to queue for hot state commit.")
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
    pub fn spawn(base: Arc<HotStateBase>, committed: Arc<Mutex<State>>) -> SyncSender<State> {
        let (tx, rx) = std::sync::mpsc::sync_channel(MAX_HOT_STATE_COMMIT_BACKLOG);
        std::thread::spawn(move || Self::new(base, committed, rx).run());

        tx
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
            self.evict();
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

        let mut n_delete = 0;
        let n_too_large = 0; // TODO(HotState): enforce this later.
        let mut n_update = 0;
        let mut n_insert = 0;

        let delta = to_commit.make_delta(&self.committed.lock());
        for shard_id in 0..NUM_STATE_SHARDS {
            let mut updates: Vec<_> = delta.shards[shard_id].iter().collect();
            // We will update the LRU next. Here we put the deletions at the beginning, then the
            // older updates, and the newest updates are at the end.
            updates.sort_unstable_by_key(|(_key, slot)| {
                slot.hot_since_version_opt().map_or(-1, |v| v as i64)
            });

            let mut updater = LRUUpdater::new(
                &self.base.shards[shard_id],
                &mut self.heads[shard_id],
                &mut self.tails[shard_id],
                self.base.max_items_per_shard,
            );

            for (key, slot) in updates {
                let has_old_entry = if let Some(old_slot) = self.base.get_state_slot(&key) {
                    self.total_key_bytes -= key.size();
                    self.total_value_bytes -= old_slot.size();
                    true
                } else {
                    false
                };

                if slot.is_cold() {
                    // deletion
                    if has_old_entry {
                        n_delete += 1;
                        updater.delete(&key);
                    }
                } else {
                    if has_old_entry {
                        n_update += 1;
                    } else {
                        n_insert += 1;
                    };

                    self.total_key_bytes += key.size();
                    self.total_value_bytes += slot.size();

                    updater.insert(key, slot);
                }
            }
        }

        COUNTER.inc_with_by(&["hot_state_delete"], n_delete);
        COUNTER.inc_with_by(&["hot_state_too_large"], n_too_large);
        COUNTER.inc_with_by(&["hot_state_update"], n_update);
        COUNTER.inc_with_by(&["hot_state_insert"], n_insert);
    }

    fn evict(&mut self) {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["hot_state_evict"]);
        let mut num_evicted = 0;

        for shard_id in 0..NUM_STATE_SHARDS {
            let mut updater = LRUUpdater::new(
                &self.base.shards[shard_id],
                &mut self.heads[shard_id],
                &mut self.tails[shard_id],
                self.base.max_items_per_shard,
            );
            let evicted = updater.evict();
            num_evicted += evicted.len();
            for (key, slot) in &evicted {
                self.total_key_bytes -= key.size();
                self.total_value_bytes -= slot.size();
            }
        }
        COUNTER.inc_with_by(&["hot_state_evict"], num_evicted as u64);
    }
}

struct LRUUpdater<'a, K, V>
where
    K: Eq + std::hash::Hash,
{
    shard: &'a Shard<K, V>,
    head: &'a mut Option<K>,
    tail: &'a mut Option<K>,
    max_items: usize,
}

impl<'a, K, V> LRUUpdater<'a, K, V>
where
    K: Clone + std::fmt::Debug + Eq + std::hash::Hash,
    V: Clone + std::fmt::Debug + THotStateSlot<Key = K>,
{
    fn new(
        shard: &'a Shard<K, V>,
        head: &'a mut Option<K>,
        tail: &'a mut Option<K>,
        max_items: usize,
    ) -> Self {
        Self {
            shard,
            head,
            tail,
            max_items,
        }
    }

    fn insert(&mut self, key: K, value: V) {
        if self.shard.contains_key(&key) {
            self.delete(&key);
        }
        self.insert_to_front(key, value);
    }

    /// Deletes and returns the oldest entry.
    fn delete_lru(&mut self) -> Option<(K, V)> {
        let key = match &self.tail {
            Some(k) => k.clone(),
            None => return None,
        };
        let value = self.delete(&key).expect("Tail must exist.");
        Some((key, value))
    }

    fn delete(&mut self, key: &K) -> Option<V> {
        let old_entry = match self.shard.remove(key) {
            Some((_k, e)) => e,
            None => return None,
        };

        match old_entry.prev() {
            Some(prev_key) => {
                let mut prev_entry = self
                    .shard
                    .get_mut(prev_key)
                    .expect("The previous key must exist");
                prev_entry.set_next(old_entry.next().cloned());
            },
            None => {
                // There is no newer entry. The current key was the head.
                *self.head = old_entry.next().cloned();
            },
        }

        match old_entry.next() {
            Some(next_key) => {
                let mut next_entry = self
                    .shard
                    .get_mut(next_key)
                    .expect("The next key must exist.");
                next_entry.set_prev(old_entry.prev().cloned());
            },
            None => {
                // There is no older entry. The current key was the tail.
                *self.tail = old_entry.prev().cloned();
            },
        }

        Some(old_entry)
    }

    fn insert_to_front(&mut self, key: K, mut value: V) {
        assert_eq!(self.head.is_some(), self.tail.is_some());
        match self.head.take() {
            Some(head) => {
                {
                    // Release the reference to the old entry ASAP to avoid deadlock when inserting
                    // the new entry below.
                    let mut old_head_entry = self.shard.get_mut(&head).expect("Head must exist.");
                    old_head_entry.set_prev(Some(key.clone()));
                }
                value.init_lru(None, Some(head));
                self.shard.insert(key.clone(), value);
                *self.head = Some(key);
            },
            None => {
                value.init_lru(None, None);
                self.shard.insert(key.clone(), value);
                *self.head = Some(key.clone());
                *self.tail = Some(key);
            },
        }
    }

    fn evict(&mut self) -> Vec<(K, V)> {
        if !self.should_evict() {
            return Vec::new();
        }

        let mut items = Vec::with_capacity(self.shard.len() - self.max_items);
        while self.should_evict() {
            items.push(self.delete_lru().unwrap());
        }
        items
    }

    fn should_evict(&self) -> bool {
        self.shard.len() > self.max_items
    }

    #[cfg(test)]
    fn collect_all(&self) -> Vec<(K, V)> {
        assert_eq!(self.head.is_some(), self.tail.is_some());

        let mut keys = Vec::new();
        let mut values = Vec::new();

        let mut current_key = self.head.clone();
        while let Some(key) = current_key {
            let entry = self.shard.get(&key).unwrap();
            assert_eq!(entry.prev(), keys.last());
            keys.push(key);
            values.push(entry.clone());
            current_key = entry.next().cloned();
        }
        itertools::zip_eq(keys, values).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{LRUUpdater, Shard, THotStateSlot};
    use lru::LruCache;
    use proptest::{collection::vec, option, prelude::*};
    use std::num::NonZeroUsize;

    #[derive(Clone, Debug)]
    struct TestSlot {
        num: u64,
        prev: Option<u32>,
        next: Option<u32>,
    }

    impl TestSlot {
        fn new(num: u64) -> Self {
            Self {
                num,
                prev: None,
                next: None,
            }
        }
    }

    impl THotStateSlot for TestSlot {
        type Key = u32;

        fn init_lru(&mut self, prev: Option<Self::Key>, next: Option<Self::Key>) {
            self.prev = prev;
            self.next = next;
        }

        fn prev(&self) -> Option<&Self::Key> {
            self.prev.as_ref()
        }

        fn next(&self) -> Option<&Self::Key> {
            self.next.as_ref()
        }

        fn set_prev(&mut self, prev: Option<Self::Key>) {
            self.prev = prev;
        }

        fn set_next(&mut self, next: Option<Self::Key>) {
            self.next = next;
        }
    }

    proptest! {
        #[test]
        fn test_hot_state_lru(
            max_items in 1..10usize,
            updates in vec((0..20u32, option::weighted(0.8, 0..1000u64)), 1..50),
        ) {
            let shard = Shard::new(max_items);
            let mut head = None;
            let mut tail = None;

            let mut updater = LRUUpdater::new(&shard, &mut head, &mut tail, max_items);
            let mut cache = LruCache::new(NonZeroUsize::new(max_items).unwrap());

            for (key, value_opt) in updates {
                match value_opt {
                    Some(value) => {
                        updater.insert(key, TestSlot::new(value));
                        cache.put(key, value);
                    }
                    None => {
                        updater.delete(&key);
                        cache.pop(&key);
                    }
                }
                updater.evict();

                prop_assert_eq!(shard.len(), cache.len());
                let items = updater.collect_all();
                prop_assert_eq!(
                    items.into_iter().map(|(k, v)| (k, v.num)).collect::<Vec<_>>(),
                    cache.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(),
                );
            }
        }
    }
}
