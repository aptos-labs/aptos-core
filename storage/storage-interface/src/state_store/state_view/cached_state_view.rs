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
    },
    DbReader,
};
use anyhow::Result;
use aptos_metrics_core::{IntCounterVecHelper, TimerHelper};
use aptos_types::{
    state_store::{
        hot_state::THotStateSlot, state_key::StateKey, state_slot::StateSlot,
        state_storage_usage::StateStorageUsage, state_value::StateValue, StateViewId,
        StateViewResult, TStateView, NUM_STATE_SHARDS,
    },
    transaction::Version,
};
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

pub type StateCacheShard = DashMap<StateKey, StateSlot>;

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
}

impl ShardedStateCache {
    pub fn new_empty(version: Option<Version>) -> Self {
        Self {
            next_version: version.map_or(0, |v| v + 1),
            shards: Default::default(),
        }
    }

    fn shard(&self, shard_id: usize) -> &StateCacheShard {
        &self.shards[shard_id]
    }

    pub fn get_cloned(&self, state_key: &StateKey) -> Option<StateSlot> {
        self.shard(state_key.get_shard_id())
            .get(state_key)
            .map(|r| r.clone())
    }

    pub fn next_version(&self) -> Version {
        self.next_version
    }

    pub fn try_insert(&self, state_key: &StateKey, slot: &StateSlot) {
        let shard_id = state_key.get_shard_id();

        match self.shard(shard_id).entry(state_key.clone()) {
            Entry::Occupied(_) => {},
            Entry::Vacant(entry) => {
                entry.insert(slot.clone());
            },
        };
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
    pub hot: Arc<dyn HotStateView>,

    /// Persisted base state. To be fetched if a key isn't in either `speculative` or `hot_state`.
    /// `self.speculative.base_version()` is targeted in db fetches.
    cold: Arc<dyn DbReader>,

    /// State values (with update versions) read across the lifetime of the state view.
    memorized: ShardedStateCache,
}

impl Debug for CachedStateView {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.id)
    }
}

impl CachedStateView {
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
        Self::new_with_config(id, reader, hot_state, persisted_state, state)
    }

    pub fn new_with_config(
        id: StateViewId,
        reader: Arc<dyn DbReader>,
        hot_state: Arc<dyn HotStateView>,
        persisted_state: State,
        state: State,
    ) -> Self {
        let version = state.version();

        Self {
            id,
            speculative: state.into_delta(persisted_state),
            hot: hot_state,
            cold: reader,
            memorized: ShardedStateCache::new_empty(version),
        }
    }

    pub fn new_dummy(state: &State) -> Self {
        struct DummyDbReader;
        impl DbReader for DummyDbReader {}

        Self::new_impl(
            StateViewId::Miscellaneous,
            Arc::new(DummyDbReader),
            Arc::new(EmptyHotState),
            state.clone(),
            state.clone(),
        )
    }

    pub fn prime_cache(&self, updates: &StateUpdateRefs) -> Result<()> {
        let _timer = TIMER.timer_with(&["prime_state_cache"]);

        IO_POOL.install(|| {
            if let Some(updates) = updates.for_last_checkpoint_batched() {
                self.prime_cache_for_batched_updates(updates)?;
            }
            if let Some(updates) = updates.for_latest_batched() {
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

    fn get_unmemorized(&self, state_key: &StateKey) -> Result<StateSlot> {
        COUNTER.inc_with(&["sv_unmemorized"]);

        let ret = if let Some(slot) = self.speculative.get_state_slot(state_key) {
            COUNTER.inc_with(&["sv_hit_speculative"]);
            slot
        } else if let Some(slot) = self.hot.get_state_slot(state_key) {
            COUNTER.inc_with(&["sv_hit_hot"]);
            slot
        } else if let Some(base_version) = self.base_version() {
            COUNTER.inc_with(&["sv_cold"]);
            StateSlot::from_db_get(
                self.cold
                    .get_state_value_with_version_by_version(state_key, base_version)?,
            )
        } else {
            StateSlot::ColdVacant
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

    fn get_state_slot(&self, state_key: &StateKey) -> StateViewResult<StateSlot> {
        let _timer = TIMER.timer_with(&["get_state_value"]);
        COUNTER.inc_with(&["sv_total_get"]);

        // First check if requested key is already memorized.
        if let Some(slot) = self.memorized.get_cloned(state_key) {
            COUNTER.inc_with(&["sv_memorized"]);
            return Ok(slot);
        }

        // TODO(aldenhu): reduce duplicated gets
        let slot = self.get_unmemorized(state_key)?;
        self.memorized.try_insert(state_key, &slot);
        Ok(slot)
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        Ok(self.speculative.current.usage())
    }

    fn next_version(&self) -> Version {
        self.speculative.next_version()
    }

    fn num_hot_items(&self) -> [usize; NUM_STATE_SHARDS] {
        self.speculative.num_hot_items()
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

    fn next_version(&self) -> Version {
        self.db_state_view.next_version()
    }
}
