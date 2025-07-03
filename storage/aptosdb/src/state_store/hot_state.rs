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
use aptos_types::state_store::{state_key::StateKey, state_slot::StateSlot, StateViewResult};
use dashmap::DashMap;
use std::sync::{
    mpsc::{Receiver, SyncSender, TryRecvError},
    Arc,
};

const MAX_HOT_STATE_COMMIT_BACKLOG: usize = 10;

#[derive(Debug)]
struct Entry<K, V> {
    data: V,
    /// The key that is slightly newer than the current entry. `None` for the newest entry.
    prev: Option<K>,
    /// The key that is slightly older than the current entry. `None` for the oldest entry.
    next: Option<K>,
}

#[derive(Debug)]
pub struct HotStateBase<K = StateKey, V = StateSlot>
where
    K: Eq + std::hash::Hash,
{
    /// After committing a new batch to `inner`, items are evicted so that
    ///  1. total number of items doesn't exceed this number
    max_items: usize,
    ///  2. total number of bytes, incl. both keys and values doesn't exceed this number
    #[allow(dead_code)] // TODO(HotState): not enforced for now
    max_bytes: usize,
    /// No item is accepted to `inner` if the size of the value exceeds this number
    #[allow(dead_code)] // TODO(HotState): not enforced for now
    max_single_value_bytes: usize,

    inner: DashMap<K, Entry<K, V>>,
}

impl<K, V> HotStateBase<K, V>
where
    K: Eq + std::hash::Hash,
    V: Clone,
{
    fn new_empty(max_items: usize, max_bytes: usize, max_single_value_bytes: usize) -> Self {
        Self {
            max_items,
            max_bytes,
            max_single_value_bytes,
            inner: DashMap::with_capacity(max_items),
        }
    }

    fn get(&self, key: &K) -> Option<V> {
        self.inner.get(key).map(|val| val.data.clone())
    }
}

impl HotStateView for HotStateBase<StateKey, StateSlot> {
    fn get_state_slot(&self, state_key: &StateKey) -> StateViewResult<Option<StateSlot>> {
        Ok(self.get(state_key))
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
        max_items: usize,
        max_bytes: usize,
        max_single_value_bytes: usize,
    ) -> Self {
        let base = Arc::new(HotStateBase::new_empty(
            max_items,
            max_bytes,
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
    head: Option<StateKey>,
    /// Points to the oldest entry. `None` if empty.
    tail: Option<StateKey>,
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
            head: None,
            tail: None,
        }
    }

    fn run(&mut self) {
        info!("HotState committer thread started.");

        while let Some(to_commit) = self.next_to_commit() {
            self.commit(&to_commit);
            *self.committed.lock() = to_commit;

            GAUGE.set_with(&["hot_state_items"], self.base.inner.len() as i64);
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
        let mut all_updates = delta
            .shards
            .iter()
            .flat_map(|shard| shard.iter())
            .collect::<Vec<_>>();
        // We will update the LRU next. Here we put the deletions at the
        // beginning, then the older updates, and the newest updates are at the
        // end.
        all_updates.sort_unstable_by_key(|(_key, slot)| {
            slot.hot_since_version_opt().map_or(-1, |v| v as i64)
        });

        let mut updater = LRUUpdater::new(Arc::clone(&self.base), &mut self.head, &mut self.tail);
        for (key, slot) in all_updates {
            let has_old_entry = if let Some(old_slot) = self.base.get(&key) {
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

        COUNTER.inc_with_by(&["hot_state_delete"], n_delete);
        COUNTER.inc_with_by(&["hot_state_too_large"], n_too_large);
        COUNTER.inc_with_by(&["hot_state_update"], n_update);
        COUNTER.inc_with_by(&["hot_state_insert"], n_insert);

        {
            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["hot_state_evict"]);

            let evicted = updater.evict();
            if evicted.is_empty() {
                return;
            }
            for (key, slot) in &evicted {
                self.total_key_bytes -= key.size();
                self.total_value_bytes -= slot.size();
            }

            let head = self
                .head
                .as_ref()
                .expect("LRU must not be empty when eviction has happened.");
            let latest_version = self
                .base
                .inner
                .get(head)
                .expect("Head must exist.")
                .data
                .expect_hot_since_version();
            let max_evicted_version = evicted.last().unwrap().1.expect_hot_since_version();
            GAUGE.set_with(
                &["hot_state_item_evict_age_versions"],
                (latest_version - max_evicted_version) as i64,
            );
            COUNTER.inc_with_by(&["hot_state_evict"], evicted.len() as u64);
        }
    }
}

struct LRUUpdater<'a, K, V>
where
    K: Eq + std::hash::Hash,
{
    base: Arc<HotStateBase<K, V>>,
    head: &'a mut Option<K>,
    tail: &'a mut Option<K>,
}

impl<'a, K, V> LRUUpdater<'a, K, V>
where
    K: Clone + std::fmt::Debug + Eq + std::hash::Hash,
    V: Clone + std::fmt::Debug,
{
    fn new(
        base: Arc<HotStateBase<K, V>>,
        head: &'a mut Option<K>,
        tail: &'a mut Option<K>,
    ) -> Self {
        Self { base, head, tail }
    }

    fn insert(&mut self, key: K, value: V) {
        if self.base.inner.contains_key(&key) {
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
        let old_entry = match self.base.inner.remove(key) {
            Some((_k, e)) => e,
            None => return None,
        };

        match &old_entry.prev {
            Some(prev_key) => {
                let mut prev_entry = self
                    .base
                    .inner
                    .get_mut(prev_key)
                    .expect("The previous key must exist");
                prev_entry.next = old_entry.next.clone();
            },
            None => {
                // There is no newer entry. The current key was the head.
                *self.head = old_entry.next.clone();
            },
        }

        match &old_entry.next {
            Some(next_key) => {
                let mut next_entry = self
                    .base
                    .inner
                    .get_mut(next_key)
                    .expect("The next key must exist.");
                next_entry.prev = old_entry.prev;
            },
            None => {
                // There is no older entry. The current key was the tail.
                *self.tail = old_entry.prev;
            },
        }

        Some(old_entry.data)
    }

    fn insert_to_front(&mut self, key: K, value: V) {
        assert_eq!(self.head.is_some(), self.tail.is_some());
        match self.head.take() {
            Some(head) => {
                {
                    // Release the reference to the old entry ASAP to avoid deadlock when inserting
                    // the new entry below.
                    let mut old_head_entry =
                        self.base.inner.get_mut(&head).expect("Head must exist.");
                    old_head_entry.prev = Some(key.clone());
                }
                let entry = Entry {
                    data: value,
                    prev: None,
                    next: Some(head),
                };
                self.base.inner.insert(key.clone(), entry);
                *self.head = Some(key);
            },
            None => {
                let entry = Entry {
                    data: value,
                    prev: None,
                    next: None,
                };
                self.base.inner.insert(key.clone(), entry);
                *self.head = Some(key.clone());
                *self.tail = Some(key);
            },
        }
    }

    fn evict(&mut self) -> Vec<(K, V)> {
        if !self.should_evict() {
            return Vec::new();
        }

        let mut items = Vec::with_capacity(self.base.inner.len() - self.base.max_items);
        while self.should_evict() {
            items.push(self.delete_lru().unwrap());
        }
        items
    }

    fn should_evict(&self) -> bool {
        self.base.inner.len() > self.base.max_items
    }

    #[cfg(test)]
    fn collect_all(&self) -> Vec<(K, V)> {
        assert_eq!(self.head.is_some(), self.tail.is_some());

        let mut keys = Vec::new();
        let mut values = Vec::new();

        let mut current_key = self.head.clone();
        while let Some(key) = current_key {
            let entry = self.base.inner.get(&key).unwrap();
            assert_eq!(entry.prev, keys.last().cloned());
            keys.push(key);
            values.push(entry.data.clone());
            current_key = entry.next.clone();
        }
        itertools::zip_eq(keys, values).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{HotStateBase, LRUUpdater};
    use lru::LruCache;
    use proptest::{collection::vec, option, prelude::*};
    use std::sync::Arc;

    const MAX_BYTES: usize = 10000;
    const MAX_SINGLE_VALUE_BYTES: usize = 100;

    proptest! {
        #[test]
        fn test_hot_state_lru(
            max_items in 1..10usize,
            updates in vec((0..20u64, option::weighted(0.8, 0..1000u64)), 1..50),
        ) {
            let base = Arc::new(HotStateBase::new_empty(
                max_items,
                MAX_BYTES,
                MAX_SINGLE_VALUE_BYTES,
            ));
            let mut head = None;
            let mut tail = None;

            let mut updater = LRUUpdater::new(base, &mut head, &mut tail);
            let mut cache = LruCache::new(max_items);

            for (key, value_opt) in updates {
                match value_opt {
                    Some(value) => {
                        updater.insert(key, value);
                        cache.put(key, value);
                    }
                    None => {
                        updater.delete(&key);
                        cache.pop(&key);
                    }
                }
                updater.evict();

                prop_assert_eq!(updater.base.inner.len(), cache.len());
                let items = updater.collect_all();
                prop_assert_eq!(items, cache.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>());
            }
        }
    }
}
