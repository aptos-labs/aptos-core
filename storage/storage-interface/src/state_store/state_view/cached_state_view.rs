// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::TIMER,
    state_store::{
        state::State,
        state_delta::StateDelta,
        state_update_refs::{BatchedStateUpdateRefs, StateUpdateRefs},
        state_view::db_state_view::DbStateView,
        versioned_state_value::StateCacheEntry,
    },
    DbReader,
};
use anyhow::Result;
use aptos_metrics_core::TimerHelper;
use aptos_types::{
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
        StateViewId, StateViewResult, TStateView,
    },
    transaction::Version,
};
use core::fmt;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use rayon::prelude::*;
use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
    sync::Arc,
};

pub type StateCacheShard = DashMap<StateKey, StateCacheEntry>;

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
    pub shards: [StateCacheShard; 16],
}

impl ShardedStateCache {
    pub fn new_empty(version: Option<Version>) -> Self {
        Self {
            next_version: version.map_or(0, |v| v + 1),
            shards: Default::default(),
        }
    }

    fn shard(&self, shard_id: u8) -> &StateCacheShard {
        &self.shards[shard_id as usize]
    }

    pub fn get_cloned(&self, state_key: &StateKey) -> Option<StateCacheEntry> {
        self.shard(state_key.get_shard_id())
            .get(state_key)
            .map(|r| r.clone())
    }

    pub fn next_version(&self) -> Version {
        self.next_version
    }
}

/// `CachedStateView` is like a snapshot of the global state comprised of state view at two
/// levels, persistent storage and memory.
/// TODO(aldenhu): This is actually MemorizingStateUpdateView?
pub struct CachedStateView {
    /// For logging and debugging purpose, identifies what this view is for.
    id: StateViewId,

    /// The persisted state is readable from the persist storage, at the version of
    /// `self.speculative.base_version()`
    reader: Arc<dyn DbReader>,

    /// The in-memory state on top of known persisted state
    speculative: StateDelta,

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
        let persisted_state = reader.get_persisted_state()?;
        Ok(Self::new_impl(id, reader, persisted_state, state))
    }

    pub fn new_impl(
        id: StateViewId,
        reader: Arc<dyn DbReader>,
        persisted_state: State,
        state: State,
    ) -> Self {
        Self {
            id,
            reader,
            memorized: ShardedStateCache::new_empty(state.version()),
            speculative: state.into_delta(persisted_state),
        }
    }

    pub fn new_dummy(state: &State) -> Self {
        struct DummyDbReader;
        impl DbReader for DummyDbReader {}

        Self {
            id: StateViewId::Miscellaneous,
            reader: Arc::new(DummyDbReader),
            memorized: ShardedStateCache::new_empty(None),
            speculative: state.make_delta(state),
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
        let Self {
            id: _,
            reader: _,
            speculative: _,
            memorized,
        } = self;

        memorized
    }

    fn base_version(&self) -> Option<Version> {
        self.speculative.base_version()
    }

    fn get_uncached(&self, state_key: &StateKey) -> Result<StateCacheEntry> {
        let ret = if let Some(update) = self.speculative.get_state_update(state_key) {
            // found in speculative state, can be either a new value or a deletion
            update.to_state_value_with_version()
        } else if let Some(base_version) = self.base_version() {
            StateCacheEntry::from_tuple_opt(
                self.reader
                    .get_state_value_with_version_by_version(state_key, base_version)?,
            )
        } else {
            StateCacheEntry::NonExistent
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
        // First check if the cache has the state value.
        if let Some(value_with_version_opt) = self.memorized.get_cloned(state_key) {
            return Ok(value_with_version_opt.into_state_value_opt());
        }

        // TODO(aldenhu): reduce duplicated gets
        let value_with_version_opt = self.get_uncached(state_key)?;

        // Update the cache if still empty
        let new_value_with_version_opt = self
            .memorized
            .shard(state_key.get_shard_id())
            .entry(state_key.clone())
            .or_insert(value_with_version_opt);
        Ok(new_value_with_version_opt.to_state_value_opt())
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
