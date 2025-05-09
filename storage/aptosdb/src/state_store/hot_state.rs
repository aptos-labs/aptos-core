// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{COUNTER, GAUGE, OTHER_TIMERS_SECONDS};
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_metrics_core::{IntCounterHelper, IntGaugeHelper, TimerHelper};
use aptos_storage_interface::state_store::{
    state::State, state_slot::StateSlot, state_view::hot_state_view::HotStateView,
};
use aptos_types::{
    state_store::{state_key::StateKey, StateViewResult},
    transaction::Version,
};
use dashmap::DashMap;
use std::{
    collections::BTreeSet,
    sync::{
        mpsc::{Receiver, SyncSender, TryRecvError},
        Arc,
    },
};

const MAX_HOT_STATE_COMMIT_BACKLOG: usize = 10;

#[derive(Debug)]
pub struct HotStateBase {
    /// After committing a new batch to `inner`, items are evicted so that
    ///  1. total number of items doesn't exceed this number
    max_items: usize,
    ///  2. total number of bytes, incl. both keys and values doesn't exceed this number
    max_bytes: usize,
    /// No item is accepted to `inner` if the size of the value exceeds this number
    max_single_value_bytes: usize,

    inner: DashMap<StateKey, StateSlot>,
}

impl HotStateBase {
    fn new_empty(max_items: usize, max_bytes: usize, max_single_value_bytes: usize) -> Self {
        Self {
            max_items,
            max_bytes,
            max_single_value_bytes,
            inner: DashMap::with_capacity(max_items),
        }
    }

    fn get(&self, key: &StateKey) -> Option<StateSlot> {
        self.inner.get(key).map(|val| val.clone())
    }
}

impl HotStateView for HotStateBase {
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
    key_by_hot_since_version: BTreeSet<(Version, StateKey)>,
    total_key_bytes: usize,
    total_value_bytes: usize,
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
            key_by_hot_since_version: BTreeSet::new(),
            total_key_bytes: 0,
            total_value_bytes: 0,
        }
    }

    fn run(&mut self) {
        info!("HotState committer thread started.");

        while let Some(to_commit) = self.next_to_commit() {
            self.commit(&to_commit);
            self.evict();
            *self.committed.lock() = to_commit;

            assert_eq!(self.key_by_hot_since_version.len(), self.base.inner.len());

            GAUGE.set_with(
                &["hot_state_items"],
                self.key_by_hot_since_version.len() as i64,
            );
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
        let mut n_too_large = 0;
        let mut n_update = 0;
        let mut n_insert = 0;

        let delta = to_commit.make_delta(&self.committed.lock());
        for (key, slot) in delta.shards.iter().flat_map(|shard| shard.iter()) {
            let has_old_entry = if let Some(old_slot) = self.base.get(&key) {
                self.total_key_bytes -= key.size();
                self.total_value_bytes -= old_slot.size();

                self.key_by_hot_since_version
                    .remove(&(old_slot.expect_hot_since_version(), key.clone()));
                true
            } else {
                false
            };

            if slot.is_cold() {
                // deletion
                if has_old_entry {
                    n_delete += 1;
                }

                self.base.inner.remove(&key);
            } else if slot.size() > self.base.max_single_value_bytes {
                // item too large to hold in memory
                n_too_large += 1;

                self.base.inner.remove(&key);
            } else {
                if has_old_entry {
                    n_update += 1;
                } else {
                    n_insert += 1;
                };

                self.total_key_bytes += key.size();
                self.total_value_bytes += slot.size();

                self.key_by_hot_since_version
                    .insert((slot.expect_hot_since_version(), key.clone()));
                self.base.inner.insert(key, slot);
            }
        }

        COUNTER.inc_with_by(&["hot_state_delete"], n_delete);
        COUNTER.inc_with_by(&["hot_state_too_large"], n_too_large);
        COUNTER.inc_with_by(&["hot_state_update"], n_update);
        COUNTER.inc_with_by(&["hot_state_insert"], n_insert);
    }

    fn evict(&mut self) {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["hot_state_evict"]);

        let latest_version = match self.key_by_hot_since_version.last() {
            None => {
                // hot state is empty
                return;
            },
            Some((hot_since_version, _key)) => *hot_since_version,
        };
        let mut evicted_version = 0;
        let mut num_evicted = 0;

        while self.should_evict() {
            let (ver, key) = self
                .key_by_hot_since_version
                .pop_first()
                .expect("Known Non-empty.");
            evicted_version = ver;
            let (key, slot) = self.base.inner.remove(&key).expect("Known to exist.");

            self.total_key_bytes -= key.size();
            self.total_value_bytes -= slot.size();
            num_evicted += 1;
        }

        if num_evicted > 0 {
            GAUGE.set_with(
                &["hot_state_item_evict_age_versions"],
                (latest_version - evicted_version) as i64,
            );
            COUNTER.inc_with_by(&["hot_state_evict"], num_evicted as u64);
        }
    }

    fn should_evict(&self) -> bool {
        self.base.inner.len() > self.base.max_items
            || self.total_key_bytes + self.total_value_bytes > self.base.max_bytes
    }
}
