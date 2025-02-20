// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{HOT_STATE_COMMIT_BACKLOG, OTHER_TIMERS_SECONDS};
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::state_store::{
    state::State, state_view::hot_state_view::HotStateView, versioned_state_value::DbStateUpdate,
};
use aptos_types::state_store::{errors::StateViewError, state_key::StateKey, StateViewResult};
use dashmap::DashMap;
use std::{
    collections::BTreeSet,
    sync::{
        mpsc::{Receiver, SyncSender, TryRecvError},
        Arc,
    },
};

const MAX_HOT_STATE_COMMIT_BACKLOG: usize = 10;
const HOT_STATE_MAX_ITEMS: usize = 10_000_000;
const HOT_STATE_MAX_VALUE_BYTES: usize = 40960;

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
    fn get_state_update(
        &self,
        state_key: &StateKey,
    ) -> StateViewResult<Option<DbStateUpdate>, StateViewError> {
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
        // get the committed state before the base since the writer can be committing.
        let state = self.committed.lock().clone();
        let base = self.base.clone();

        (base, state)
    }

    pub fn enqueue_commit(&self, to_commit: State) {
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
        }
    }

    fn run(&mut self) {
        info!("HotState committer thread started.");

        while let Some(to_commit) = self.drain_rx() {
            self.commit(&to_commit);
            self.evict();
            *self.committed.lock() = to_commit;
        }
    }

    fn drain_rx(&self) -> Option<State> {
        // blocking receive the first item
        let mut ret = match self.rx.recv() {
            Ok(state) => state,
            Err(_) => {
                info!("HotState committer quitting.");
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
                    info!("HotState committer quitting.");
                    return None;
                },
            }
        }

        HOT_STATE_COMMIT_BACKLOG.set(n_backlog);
        Some(ret)
    }

    fn commit(&mut self, to_commit: &State) {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["hot_state_commit"]);

        let delta = to_commit.make_delta(&self.committed.lock());

        for (key, update) in delta.shards.iter().flat_map(|shard| shard.iter()) {
            if let Some(old_upd) = self.base.get_state_update(&key).expect("no error") {
                let old_ts = old_upd.expect_non_delete().access_time_secs();
                self.key_by_access_time.remove(&(old_ts, key.clone()));
            }
            if update.value.is_none() {
                // deletion
                self.base.inner.remove(&key);
            } else if update.expect_non_delete().size() > HOT_STATE_MAX_VALUE_BYTES {
                // item too large to hold in memory
                self.base.inner.remove(&key);
            } else {
                let new_ts = update.expect_non_delete().access_time_secs();
                self.key_by_access_time.insert((new_ts, key.clone()));
                self.base.inner.insert(key, update);
            }
        }
    }

    fn evict(&mut self) {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["hot_state_evict"]);

        let total = self.base.inner.len();
        if total <= HOT_STATE_MAX_ITEMS {
            return;
        }

        let to_evict = total - HOT_STATE_MAX_ITEMS;
        for _ in 0..to_evict {
            let (_ts, key) = self
                .key_by_access_time
                .pop_first()
                .expect("Known not empty.");
            self.base.inner.remove(&key);
        }
    }
}
