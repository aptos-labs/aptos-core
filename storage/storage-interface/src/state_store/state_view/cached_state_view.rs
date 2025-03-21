// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::{COUNTER, TIMER},
    state_store::{
        state::State,
        state_delta::StateDelta,
        state_update_refs::{BatchedStateUpdateRefs, StateUpdateRefs},
        state_view::{
            db_state_view::DbStateView,
            hot_state_view::{EmptyHotState, HotStateView},
        },
        versioned_state_value::{DbStateUpdate, MemorizedStateRead},
        NUM_STATE_SHARDS,
    },
    DbReader,
};
use anyhow::Result;
use aptos_infallible::duration_since_epoch;
use aptos_metrics_core::{IntCounterHelper, TimerHelper};
use aptos_types::{
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
        StateViewId, StateViewResult, TStateView,
    },
    transaction::Version,
};
use arr_macro::arr;
use core::fmt;
use dashmap::{mapref::entry::Entry, DashMap};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use rayon::prelude::*;
use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
    sync::Arc,
};

pub type StateCacheShard = DashMap<StateKey, MemorizedStateRead>;
pub type HotStateShardRefreshes = DashMap<StateKey, DbStateUpdate>;

static IO_POOL: Lazy<rayon::ThreadPool> = Lazy::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads(32)
        .thread_name(|index| format!("kv_reader_{}", index))
        .build()
        .unwrap()
});

#[derive(Debug)]
pub struct ShardedStateCache {
    next_version: Version,
    pub shards: [StateCacheShard; NUM_STATE_SHARDS],
    pub hot_state_refreshes: [HotStateShardRefreshes; NUM_STATE_SHARDS],
    access_time_secs: u32,
}

impl ShardedStateCache {
    pub fn new_empty(version: Option<Version>) -> Self {
        Self {
            next_version: version.map_or(0, |v| v + 1),
            shards: Default::default(),
            hot_state_refreshes: arr![DashMap::with_capacity(32); 16],
            access_time_secs: duration_since_epoch().as_secs() as u32,
        }
    }

    fn shard(&self, shard_id: u8) -> &StateCacheShard {
        &self.shards[shard_id as usize]
    }

    pub fn get_cloned(&self, state_key: &StateKey) -> Option<MemorizedStateRead> {
        self.shard(state_key.get_shard_id())
            .get(state_key)
            .map(|r| r.clone())
    }

    pub fn next_version(&self) -> Version {
        self.next_version
    }

    pub fn try_insert(
        &self,
        state_key: &StateKey,
        value: MemorizedStateRead,
        access_time_refresh_interval_secs: u32,
    ) -> Option<StateValue> {
        let shard_id = state_key.get_shard_id();

        let try_get_hot_state_refresh = match self.shard(shard_id).entry(state_key.clone()) {
            Entry::Occupied(_) => false,
            Entry::Vacant(entry) => {
                entry.insert(value.clone());
                true
            },
        };
        if try_get_hot_state_refresh {
            if let Some(refresh) =
                value.to_hot_state_refresh(self.access_time_secs, access_time_refresh_interval_secs)
            {
                self.hot_state_refreshes[shard_id as usize].insert(state_key.clone(), refresh);
            }
        }

        value.to_state_value_opt()
    }
}

/// `CachedStateView` is like a snapshot of the global state comprised of state view at two
/// levels, persistent storage and memory.
/// TODO(aldenhu): This is actually MemorizingStateUpdateView?
pub struct CachedStateView {
    /// For logging and debugging purpose, identifies what this view is for.
    id: StateViewId,

    /// The in-memory state on top of known persisted state.
    speculative: StateDelta,

    /// Persisted hot state. To be fetched if a key isn't in `speculative`.
    hot: Arc<dyn HotStateView>,

    /// Persisted base state. To be fetched if a key isn't in either `speculative` or `hot_state`.
    /// `self.speculative.base_version()` is targeted in db fetches.
    cold: Arc<dyn DbReader>,

    /// State values (with update versions) read across the lifetime of the state view.
    memorized: ShardedStateCache,

    /// Hot state access time updates no more frequent than the set interval.
    access_time_refresh_interval_secs: u32,
}

impl Debug for CachedStateView {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.id)
    }
}

impl CachedStateView {
    const ACCESS_TIME_REFRESH_INTERVAL_SECS: u32 = 600;

    /// Constructs a [`CachedStateView`] with persistent state view in the DB and the in-memory
    /// speculative state represented by `speculative_state`. The persistent state view is the
    /// latest one preceding `next_version`
    pub fn new(id: StateViewId, reader: Arc<dyn DbReader>, state: State) -> StateViewResult<Self> {
        let (hot_state, persisted_state) = reader.get_persisted_state()?;
        Ok(Self::new_impl(
            id,
            reader,
            hot_state,
            persisted_state,
            state,
        ))
    }

    pub fn new_impl(
        id: StateViewId,
        reader: Arc<dyn DbReader>,
        hot_state: Arc<dyn HotStateView>,
        persisted_state: State,
        state: State,
    ) -> Self {
        Self::new_with_config(
            id,
            reader,
            hot_state,
            persisted_state,
            state,
            Self::ACCESS_TIME_REFRESH_INTERVAL_SECS,
        )
    }

    pub fn new_with_config(
        id: StateViewId,
        reader: Arc<dyn DbReader>,
        hot_state: Arc<dyn HotStateView>,
        persisted_state: State,
        state: State,
        access_time_refresh_interval_secs: u32,
    ) -> Self {
        let version = state.version();

        Self {
            id,
            speculative: state.into_delta(persisted_state),
            hot: hot_state,
            cold: reader,
            memorized: ShardedStateCache::new_empty(version),
            access_time_refresh_interval_secs,
        }
    }

    pub fn new_dummy(state: &State) -> Self {
        struct DummyDbReader;
        impl DbReader for DummyDbReader {}

        Self {
            id: StateViewId::Miscellaneous,
            speculative: state.make_delta(state),
            hot: Arc::new(EmptyHotState),
            cold: Arc::new(DummyDbReader),
            memorized: ShardedStateCache::new_empty(None),
            access_time_refresh_interval_secs: Self::ACCESS_TIME_REFRESH_INTERVAL_SECS,
        }
    }

    pub fn prime_cache(&self, updates: &StateUpdateRefs) -> Result<()> {
        let _timer = TIMER.timer_with(&["prime_state_cache"]);

        IO_POOL.install(|| {
            if let Some(updates) = &updates.for_last_checkpoint {
                self.prime_cache_for_batched_updates(updates)?;
            }
            if let Some(updates) = &updates.for_latest {
                self.prime_cache_for_batched_updates(updates)?;
            }
            Ok(())
        })
    }

    fn prime_cache_for_batched_updates(&self, updates: &BatchedStateUpdateRefs) -> Result<()> {
        updates
            .shards
            .par_iter()
            .try_for_each(|shard| self.prime_cache_for_keys(shard.keys().cloned()))
    }

    fn prime_cache_for_keys<'a, T: IntoIterator<Item = &'a StateKey> + Send>(
        &self,
        keys: T,
    ) -> Result<()> {
        rayon::scope(|s| {
            keys.into_iter().for_each(|key| {
                s.spawn(move |_| {
                    self.get_state_value(key).expect("Must succeed.");
                })
            });
        });
        Ok(())
    }

    /// Consumes `Self` and returns the state and all the memorized state reads.
    pub fn into_memorized_reads(self) -> ShardedStateCache {
        self.memorized
    }

    fn base_version(&self) -> Option<Version> {
        self.speculative.base_version()
    }

    fn get_unmemorized(&self, state_key: &StateKey) -> Result<MemorizedStateRead> {
        COUNTER.inc_with(&["sv_unmemorized"]);

        let ret = if let Some(update) = self.speculative.get_state_update(state_key) {
            COUNTER.inc_with(&["sv_hit_speculative"]);
            MemorizedStateRead::from_speculative_state(update)
        } else if let Some(update) = self.hot.get_state_update(state_key)? {
            COUNTER.inc_with(&["sv_hit_hot"]);
            MemorizedStateRead::from_hot_state_hit(update)
        } else if let Some(base_version) = self.base_version() {
            COUNTER.inc_with(&["sv_cold"]);
            MemorizedStateRead::from_db_get(
                self.cold
                    .get_state_value_with_version_by_version(state_key, base_version)?,
            )
        } else {
            MemorizedStateRead::NonExistent
        };

        Ok(ret)
    }

    pub fn next_version(&self) -> Version {
        self.speculative.next_version()
    }

    pub fn current_state(&self) -> &State {
        &self.speculative.current
    }

    pub fn persisted_state(&self) -> &State {
        &self.speculative.base
    }

    pub fn memorized_reads(&self) -> &ShardedStateCache {
        &self.memorized
    }
}

impl TStateView for CachedStateView {
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        self.id
    }

    fn get_state_value(&self, state_key: &StateKey) -> StateViewResult<Option<StateValue>> {
        let _timer = TIMER.with_label_values(&["get_state_value"]).start_timer();
        COUNTER.inc_with(&["sv_total_get"]);

        // First check if requested key is already memorized.
        if let Some(value_with_version_opt) = self.memorized.get_cloned(state_key) {
            COUNTER.inc_with(&["sv_memorized"]);
            return Ok(value_with_version_opt.into_state_value_opt());
        }

        // TODO(aldenhu): reduce duplicated gets
        let value_with_version_opt = self.get_unmemorized(state_key)?;
        Ok(self.memorized.try_insert(
            state_key,
            value_with_version_opt,
            self.access_time_refresh_interval_secs,
        ))
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        Ok(self.speculative.current.usage())
    }
}

pub struct CachedDbStateView {
    db_state_view: DbStateView,
    state_cache: RwLock<HashMap<StateKey, Option<StateValue>>>,
}

impl From<DbStateView> for CachedDbStateView {
    fn from(db_state_view: DbStateView) -> Self {
        Self {
            db_state_view,
            state_cache: RwLock::new(HashMap::new()),
        }
    }
}

impl TStateView for CachedDbStateView {
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        self.db_state_view.id()
    }

    fn get_state_value(&self, state_key: &StateKey) -> StateViewResult<Option<StateValue>> {
        // First check if the cache has the state value.
        if let Some(val_opt) = self.state_cache.read().get(state_key) {
            // This can return None, which means the value has been deleted from the DB.
            return Ok(val_opt.clone());
        }
        let state_value_option = self.db_state_view.get_state_value(state_key)?;
        // Update the cache if still empty
        let mut cache = self.state_cache.write();
        let new_value = cache
            .entry(state_key.clone())
            .or_insert_with(|| state_value_option);
        Ok(new_value.clone())
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        self.db_state_view.get_usage()
    }
}
