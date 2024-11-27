// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::TIMER,
    state_store::{
        state::State, state_delta::StateDelta, state_update::StateValueWithVersionOpt,
        state_view::db_state_view::DbStateView,
    },
    DbReader,
};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_scratchpad::{FrozenSparseMerkleTree, SparseMerkleTree};
use aptos_types::{
    proof::SparseMerkleProofExt,
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
        StateViewId, StateViewResult, TStateView,
    },
    transaction::Version,
    write_set::WriteSet,
};
use core::fmt;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
    sync::Arc,
};
/* FIXME(aldenhu): remove
static IO_POOL: Lazy<rayon::ThreadPool> = Lazy::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads(32)
        .thread_name(|index| format!("kv_reader_{}", index))
        .build()
        .unwrap()
});
 */

pub type StateCacheShard = DashMap<StateKey, StateValueWithVersionOpt>;

#[derive(Debug, Default)]
pub struct ShardedStateCache {
    pub shards: [StateCacheShard; 16],
}

impl ShardedStateCache {
    pub fn combine(&mut self, _rhs: Self) {
        // FIXME(aldenhu): remove?
        todo!()
    }

    pub fn shard(&self, shard_id: u8) -> &StateCacheShard {
        &self.shards[shard_id as usize]
    }

    pub fn get_cloned(&self, state_key: &StateKey) -> Option<StateValueWithVersionOpt> {
        self.shard(state_key.get_shard_id())
            .get(state_key)
            .map(|r| r.clone())
    }
}

/// `CachedStateView` is like a snapshot of the global state comprised of state view at two
/// levels, persistent storage and memory.
/// TODO(aldenhu): This is actually MemorizingStateUpdateView?
pub struct CachedStateView {
    /// For logging and debugging purpose, identifies what this view is for.
    id: StateViewId,

    /// The persisted state is readable from the persist storage, at the version of
    /// `self.speculative.parent_version()`
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
    pub fn new(
        _id: StateViewId,
        _reader: Arc<dyn DbReader>,
        _state: State,
    ) -> StateViewResult<Self> {
        // FIXME(aldnehu): get persisted state from db and call `new_impl()`
        todo!()
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
            speculative: StateDelta::new(persisted_state, state),
            memorized: ShardedStateCache::default(),
        }
    }

    pub fn new_dummy() -> Self {
        // FIXME(aldenhu)
        todo!()
    }

    pub fn prime_cache_by_write_set<'a, T: IntoIterator<Item = &'a WriteSet> + Send>(
        &self,
        _write_sets: T,
    ) -> StateViewResult<()> {
        /*
        IO_POOL.scope(|s| {
            write_sets
                .into_iter()
                .flat_map(|write_set| write_set.iter())
                .map(|(key, _)| key)
                .collect::<HashSet<_>>()
                .into_iter()
                .for_each(|key| {
                    s.spawn(move |_| {
                        self.get_state_value_bytes(key).expect("Must succeed.");
                    })
                });
        });
        Ok(())
         */
        // FIXME(aldenhu): remove
        todo!()
    }

    pub fn into_state_cache(self) -> StateCache {
        // FIXME(aldenhu): rename: seal
        todo!()
    }

    /// Consumes `Self` and returns the state and all the memorized state reads.
    pub fn finish(self) -> (State, ShardedStateCache) {
        todo!()
        /* FIXME(aldenhu)
        let Self {
            id: _,
            persisted: _,
            speculative,
            memorized,
        } = self;

        (speculative.current, memorized)
         */
    }

    fn parent_version(&self) -> Option<Version> {
        self.speculative.parent_version()
    }

    fn get_uncached(&self, state_key: &StateKey) -> Result<StateValueWithVersionOpt> {
        let ret = if let Some(update) = self.speculative.get_state_update(state_key) {
            // found in speculative state, can be either a new value or a deletion
            update.to_state_value_with_version()
        } else if let Some(parent_version) = self.parent_version() {
            StateValueWithVersionOpt::from_tuple_opt(
                self.reader
                    .get_state_value_with_version_by_version(state_key, parent_version)?,
            )
        } else {
            StateValueWithVersionOpt::NonExistent
        };

        Ok(ret)
    }

    pub fn next_version(&self) -> Version {
        self.speculative.next_version()
    }
}

// FIXME(aldenhu): remove unnecessary fields, probably remove entirely and use ShardedStateCache directly
#[derive(Debug)]
pub struct StateCache {
    pub frozen_base: FrozenSparseMerkleTree<StateValue>,
    pub sharded_state_cache: ShardedStateCache,
    pub proofs: HashMap<HashValue, SparseMerkleProofExt>,
}

impl StateCache {
    pub fn new_empty(smt: SparseMerkleTree<StateValue>) -> Self {
        let frozen_base = smt.freeze(&smt);
        Self {
            frozen_base,
            sharded_state_cache: ShardedStateCache::default(),
            proofs: HashMap::new(),
        }
    }

    pub fn new_dummy() -> Self {
        Self::new_empty(SparseMerkleTree::new_empty())
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
