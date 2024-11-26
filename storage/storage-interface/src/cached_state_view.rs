// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::TIMER,
    state_delta::{InMemState, StateDelta, StateUpdate},
    state_view::DbStateView,
    DbReader,
};
use aptos_types::{
    state_store::{
        errors::StateviewError, state_key::StateKey, state_storage_usage::StateStorageUsage,
        state_value::StateValue, StateViewId, TStateView,
    },
    transaction::Version,
    write_set::{TransactionWrite, WriteOp},
};
use core::fmt;
use dashmap::DashMap;
use parking_lot::RwLock;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator};
use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
    sync::Arc,
};

type Result<T, E = StateviewError> = std::result::Result<T, E>;
type StateCacheShard = DashMap<StateKey, CachedStateValue>;

#[derive(Clone, Debug)]
pub enum CachedStateValue {
    NonExistent,
    Update(StateUpdate),
}

impl CachedStateValue {
    pub fn from_write_op(version: Version, write_op: &WriteOp) -> Self {
        Self::new_update(version, write_op.as_state_value())
    }

    pub fn from_db_result(db_result: Option<(Version, StateValue)>) -> Self {
        match db_result {
            Some((version, value)) => Self::new_update(version, Some(value)),
            None => Self::NonExistent,
        }
    }

    pub fn new_update(version: Version, value: Option<StateValue>) -> Self {
        Self::Update(StateUpdate { version, value })
    }

    pub fn value_opt(&self) -> Option<StateValue> {
        match self {
            Self::NonExistent => None,
            Self::Update(StateUpdate { value, .. }) => value.clone(),
        }
    }
}

// Sharded by StateKey.get_shard_id(). The version in the value indicates there is an entry on that
// version for the given StateKey, and the version is the maximum one which <= the base version. It
// will be None if the value is None, or we found the value on the speculative tree (in that case
// we don't know the maximum version).
#[derive(Debug, Default)]
pub struct ShardedStateCache {
    shards: [StateCacheShard; 16],
}

impl ShardedStateCache {
    pub fn shard(&self, shard_id: u8) -> &StateCacheShard {
        &self.shards[shard_id as usize]
    }

    pub fn par_iter(&self) -> impl IndexedParallelIterator<Item = &StateCacheShard> {
        self.shards.par_iter()
    }
}

/// `CachedStateView` is like a snapshot of the global state comprised of state view at two
/// levels, persistent storage and memory.
pub struct CachedStateView {
    /// For logging and debugging purpose, identifies what this view is for.
    id: StateViewId,

    /// The in-memory state on top of the snapshot.
    speculative_state: StateDelta,

    /// The persisted state, readable from the persistent storage.
    reader: Arc<dyn DbReader>,

    /// State values (with update versions) read across the lifetime of the state view.
    sharded_state_cache: ShardedStateCache,
}

impl Debug for CachedStateView {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.id)
    }
}

impl CachedStateView {
    pub fn new(id: StateViewId, state: InMemState, reader: Arc<dyn DbReader>) -> Result<Self> {
        let persisted_state = reader.get_persisted_state_before(&state)?;
        let speculative_state = state.into_delta(persisted_state);

        Ok(Self {
            id,
            speculative_state,
            reader,
            sharded_state_cache: ShardedStateCache::default(),
        })
    }

    pub fn seal(self) -> StateCache {
        StateCache {
            speculative_state: self.speculative_state,
            sharded_state_cache: self.sharded_state_cache,
        }
    }

    fn get_uncached(&self, state_key: &StateKey) -> Result<CachedStateValue> {
        let ret = if let Some(state_update) = self.speculative_state.get(state_key) {
            CachedStateValue::Update(state_update)
        } else if let Some(base_version) = self.speculative_state.base_version() {
            CachedStateValue::from_db_result(
                self.reader
                    .get_state_value_with_version_by_version(state_key, base_version)?,
            )
        } else {
            CachedStateValue::NonExistent
        };
        Ok(ret)
    }

    pub fn next_version(&self) -> Version {
        self.speculative_state.next_version()
    }
}

/// FIXME(alden): remove, since parent_state is not useful?
#[derive(Debug)]
pub struct StateCache {
    /// The state being read from the `CachedStateView`.
    pub speculative_state: StateDelta,
    /// KVs got read from the DB at the base of `speculative_state` during the lifetime of the
    /// CachedStateView
    pub sharded_state_cache: ShardedStateCache,
}

impl StateCache {
    pub fn new_empty(_state: InMemState) -> Self {
        /* FIXME(aldenhu)
        let frozen_base = smt.freeze(&smt);
        Self {
            frozen_base,
            sharded_state_cache: ShardedStateCache::default(),
            proofs: HashMap::new(),
        }

         */
        todo!()
    }

    pub fn new_dummy() -> Self {
        Self::new_empty(InMemState::new_empty())
    }
}

impl TStateView for CachedStateView {
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        self.id
    }

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>> {
        let _timer = TIMER.with_label_values(&["get_state_value"]).start_timer();

        // First check if the cache has the state value.
        if let Some(state_update) = self
            .sharded_state_cache
            .shard(state_key.get_shard_id())
            .get(state_key)
        {
            return Ok(state_update.value_opt());
        }
        let state_update = self.get_uncached(state_key)?;
        // Update the cache if still empty
        let state_update = self
            .sharded_state_cache
            .shard(state_key.get_shard_id())
            .entry(state_key.clone())
            .or_insert(state_update);
        Ok(state_update.value_opt())
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        // FIXME(aldenhu): Add `.usage()` to speculative state
        todo!()
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

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>> {
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

    fn get_usage(&self) -> Result<StateStorageUsage> {
        self.db_state_view.get_usage()
    }
}
