// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{COUNTER, GAUGE, OTHER_TIMERS_SECONDS};
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_metrics_core::{IntCounterHelper, IntGaugeHelper, TimerHelper};
use aptos_storage_interface::state_store::{
    state::State, state_view::hot_state_view::HotStateView, versioned_state_value::DbStateUpdate,
};
use aptos_types::state_store::{state_key::StateKey, StateViewResult};
use dashmap::DashMap;
use std::{
    collections::BTreeSet,
    sync::{
        mpsc::{Receiver, SyncSender, TryRecvError},
        Arc,
    },
};

const MAX_HOT_STATE_COMMIT_BACKLOG: usize = 10;
const HOT_STATE_MAX_ITEMS: usize = 1_000_000;
const HOT_STATE_MAX_VALUE_BYTES: usize = 4096;

#[derive(Debug)]
pub struct HotStateBase {
    inner: DashMap<StateKey, DbStateUpdate>,
}

impl HotStateBase {
    fn new_empty() -> Self {
        Self {
            inner: DashMap::with_capacity(HOT_STATE_MAX_ITEMS),
        }
    }

    fn get(&self, key: &StateKey) -> Option<DbStateUpdate> {
        self.inner.get(key).map(|val| val.clone())
    }
}

impl HotStateView for HotStateBase {
    fn get_state_update(&self, state_key: &StateKey) -> StateViewResult<Option<DbStateUpdate>> {
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
    pub fn new(state: State) -> Self {
        let base = Arc::new(HotStateBase::new_empty());
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
    key_by_access_time: BTreeSet<(u32, StateKey)>,
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
            key_by_access_time: BTreeSet::new(),
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

            assert_eq!(self.key_by_access_time.len(), self.base.inner.len());

            GAUGE.set_with(&["hot_state_items"], self.key_by_access_time.len() as i64);
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
        for (key, update) in delta.shards.iter().flat_map(|shard| shard.iter()) {
            let has_old_value = if let Some(old_upd) = self.base.get(&key) {
                let old_val = old_upd.expect_non_delete();

                self.total_key_bytes -= key.size();
                self.total_value_bytes -= old_val.size();

                self.key_by_access_time
                    .remove(&(old_val.access_time_secs(), key.clone()));
                true
            } else {
                false
            };

            if update.value.is_none() {
                // deletion
                if has_old_value {
                    n_delete += 1;
                }

                self.base.inner.remove(&key);
            } else if update.expect_non_delete().size() > HOT_STATE_MAX_VALUE_BYTES {
                // item too large to hold in memory
                n_too_large += 1;

                self.base.inner.remove(&key);
            } else {
                if has_old_value {
                    n_update += 1;
                } else {
                    n_insert += 1;
                };
                let new_val = update.expect_non_delete();

                self.total_key_bytes += key.size();
                self.total_value_bytes += new_val.size();

                self.key_by_access_time
                    .insert((new_val.access_time_secs(), key.clone()));
                self.base.inner.insert(key, update);
            }
        }

        COUNTER.inc_with_by(&["hot_state_delete"], n_delete);
        COUNTER.inc_with_by(&["hot_state_too_large"], n_too_large);
        COUNTER.inc_with_by(&["hot_state_update"], n_update);
        COUNTER.inc_with_by(&["hot_state_insert"], n_insert);
    }

    fn evict(&mut self) {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["hot_state_evict"]);

        let total = self.base.inner.len();
        if total <= HOT_STATE_MAX_ITEMS {
            return;
        }

        let latest = self.key_by_access_time.last().expect("Known Non-empty.").0;
        let mut last_evicted = 0;

        let to_evict = total - HOT_STATE_MAX_ITEMS;
        for _ in 0..to_evict {
            let (ts, key) = self
                .key_by_access_time
                .pop_first()
                .expect("Known Non-empty.");
            last_evicted = ts;
            let (k, v) = self.base.inner.remove(&key).expect("Known to exist.");

            self.total_key_bytes -= k.size();
            self.total_value_bytes -= v.expect_non_delete().size();
        }

        GAUGE.set_with(
            &["hot_state_item_evict_age"],
            (latest - last_evicted) as i64,
        );
        COUNTER.inc_with_by(&["hot_state_evict"], to_evict as u64);
    }
}
