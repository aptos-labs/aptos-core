// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::TIMER,
    state_store::{
        hot_state::HotStateLRU,
        state_delta::StateDelta,
        state_update_refs::{BatchedStateUpdateRefs, StateUpdateRefs},
        state_view::{
            cached_state_view::{CachedStateView, ShardedStateCache, StateCacheShard},
            hot_state_view::HotStateView,
        },
        versioned_state_value::StateUpdateRef,
    },
    DbReader,
};
use anyhow::Result;
use aptos_experimental_layered_map::{LayeredMap, MapLayer};
use aptos_logger::prelude::*;
use aptos_metrics_core::TimerHelper;
use aptos_types::{
    state_store::{
        state_key::StateKey, state_slot::StateSlot, state_storage_usage::StateStorageUsage,
        StateViewId, NUM_STATE_SHARDS,
    },
    transaction::Version,
    write_set::BaseStateOp,
};
use arr_macro::arr;
use derive_more::Deref;
use rayon::prelude::*;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Debug)]
pub struct HotStateMetadata {
    pub latest: Option<StateKey>,
    pub oldest: Option<StateKey>,
    pub num_items: usize,
}

impl HotStateMetadata {
    fn new() -> Self {
        Self {
            latest: None,
            oldest: None,
            num_items: 0,
        }
    }
}

/// Represents the blockchain state at a given version.
/// n.b. the state can be either persisted or speculative.
#[derive(Clone, Debug)]
pub struct State {
    /// The next version. If this is 0, the state is the "pre-genesis" empty state.
    next_version: Version,
    /// The updates made to the state at the current version.
    ///  N.b. this is not directly iterable, one needs to make a `StateDelta`
    ///       between this and a `base_version` to list the updates or create a
    ///       new `State` at a descendant version.
    shards: Arc<[MapLayer<StateKey, StateSlot>; NUM_STATE_SHARDS]>,
    hot_state_metadata: [HotStateMetadata; NUM_STATE_SHARDS],
    /// The total usage of the state at the current version.
    usage: StateStorageUsage,
}

impl State {
    pub fn new_with_updates(
        version: Option<Version>,
        shards: Arc<[MapLayer<StateKey, StateSlot>; NUM_STATE_SHARDS]>,
        hot_state_metadata: [HotStateMetadata; NUM_STATE_SHARDS],
        usage: StateStorageUsage,
    ) -> Self {
        Self {
            next_version: version.map_or(0, |v| v + 1),
            shards,
            hot_state_metadata,
            usage,
        }
    }

    pub fn new_at_version(version: Option<Version>, usage: StateStorageUsage) -> Self {
        Self::new_with_updates(
            version,
            Arc::new(arr![MapLayer::new_family("state"); 16]),
            arr![HotStateMetadata::new(); 16],
            usage,
        )
    }

    pub fn new_empty() -> Self {
        Self::new_at_version(None, StateStorageUsage::zero())
    }

    pub fn next_version(&self) -> Version {
        self.next_version
    }

    pub fn version(&self) -> Option<Version> {
        self.next_version.checked_sub(1)
    }

    pub fn usage(&self) -> StateStorageUsage {
        self.usage
    }

    pub fn shards(&self) -> &[MapLayer<StateKey, StateSlot>; NUM_STATE_SHARDS] {
        &self.shards
    }

    pub fn make_delta(&self, base: &State) -> StateDelta {
        let _timer = TIMER.timer_with(&["state__make_delta"]);
        self.clone().into_delta(base.clone())
    }

    pub(crate) fn into_delta(self, base: State) -> StateDelta {
        StateDelta::new(base, self)
    }

    pub fn is_the_same(&self, rhs: &Self) -> bool {
        Arc::ptr_eq(&self.shards, &rhs.shards)
    }

    pub(crate) fn is_descendant_of(&self, rhs: &State) -> bool {
        self.shards[0].is_descendant_of(&rhs.shards[0])
    }

    pub(crate) fn latest_hot_key(&self, shard_id: usize) -> Option<StateKey> {
        self.hot_state_metadata[shard_id].latest.clone()
    }

    pub(crate) fn oldest_hot_key(&self, shard_id: usize) -> Option<StateKey> {
        self.hot_state_metadata[shard_id].oldest.clone()
    }

    pub(crate) fn num_hot_items(&self, shard_id: usize) -> usize {
        self.hot_state_metadata[shard_id].num_items
    }

    fn update<'kv>(
        &self,
        persisted_hot_state: Arc<dyn HotStateView>,
        persisted: &State,
        batched_updates: &BatchedStateUpdateRefs,
        per_version_updates: Vec<&[(&'kv StateKey, StateUpdateRef<'kv>)]>,
        state_cache: &ShardedStateCache,
    ) -> Self {
        assert_eq!(per_version_updates.len(), NUM_STATE_SHARDS);

        let _timer = TIMER.timer_with(&["state__update"]);

        // 1. The update batch must begin at self.next_version().
        assert_eq!(self.next_version(), batched_updates.first_version);
        // 2. The cache must be at a version equal or newer than `persisted`, otherwise
        //    updates between the cached version and the persisted version are potentially
        //    missed during the usage calculation.
        assert!(
            persisted.next_version() <= state_cache.next_version(),
            "persisted: {}, cache: {}",
            persisted.next_version(),
            state_cache.next_version(),
        );
        // 3. `self` must be at a version equal or newer than the cache, because we assume
        //    it is overlaid on top of the cache.
        assert!(self.next_version() >= state_cache.next_version());

        let overlay = self.make_delta(persisted);
        let ((shards, new_metadata), usage_delta_per_shard): ((Vec<_>, Vec<_>), Vec<_>) = (
            state_cache.shards.as_slice(),
            overlay.shards.as_slice(),
            self.hot_state_metadata.as_slice(),
            batched_updates.shards.as_slice(),
            per_version_updates.as_slice(),
        )
            .into_par_iter()
            .map(|(cache, overlay, hot_metadata, batched_updates, updates)| {
                let head = hot_metadata.latest.clone();
                let tail = hot_metadata.oldest.clone();
                println!("head: {head:?}, tail: {tail:?}");

                let mut lru =
                    HotStateLRU::new(Arc::clone(&persisted_hot_state), overlay, head, tail);
                for (key, update) in *updates {
                    // We need to decide whether to put this update inside the LRU. It should go in
                    // unless it's an eviction.
                    match update.state_op {
                        BaseStateOp::Creation(_)
                        | BaseStateOp::Modification(_)
                        | BaseStateOp::Deletion(_)
                        | BaseStateOp::MakeHot { .. } => {
                            // Construct the writes such that the key goes to the front of the LRU.
                            lru.insert((*key).clone(), update.to_result_slot());
                            // println!(
                            //     "after insertion: head: {:?}, tail: {:?}, cache: {:?}",
                            //     lru.head, lru.tail, lru.pending
                            // );
                        },
                        BaseStateOp::Eviction { .. } => {
                            // NOTE: once we actually populate the evictions here (currently it's
                            // not populated because `to_evict` in `BlockHotStateOpAccumulator` is
                            // never processed later), we want to double check inside the LRU that
                            // these keys are indeed the oldest, i.e. we are not deleting some keys
                            // in the middle.
                            // This probably can be done by comparing the newest version of the
                            // deleted key, with the tail of the LRU, or something similar.
                            //
                            // Construct the writes such that the key is removed from the LRU.
                            // Maybe assert that this key is always around the tail.
                            lru.delete(key);
                            // println!(
                            //     "after eviction: head: {:?}, tail: {:?}, cache: {:?}",
                            //     lru.head, lru.tail, lru.pending
                            // );
                        },
                    }
                }

                let new_items: Vec<_> = lru.pending.into_iter().collect();
                println!(
                    "new head: {:?}. new tail: {:?}. new_items: {:?}",
                    lru.head, lru.tail, new_items
                );

                (
                    // TODO(aldenhu): change interface to take iter of ref
                    (overlay.new_layer(&new_items), HotStateMetadata {
                        latest: lru.head,
                        oldest: lru.tail,
                        num_items: (hot_metadata.num_items as isize + lru.num_entries_changed)
                            as usize,
                    }),
                    Self::usage_delta_for_shard(cache, overlay, batched_updates),
                )
            })
            .unzip();
        let shards = Arc::new(shards.try_into().expect("Known to be 16 shards."));
        let new_metadata = new_metadata.try_into().expect("Known to be 16 shards.");
        let usage = self.update_usage(usage_delta_per_shard);

        State::new_with_updates(batched_updates.last_version(), shards, new_metadata, usage)
    }

    fn update_usage(&self, usage_delta_per_shard: Vec<(i64, i64)>) -> StateStorageUsage {
        assert_eq!(usage_delta_per_shard.len(), NUM_STATE_SHARDS);

        let (items_delta, bytes_delta) = usage_delta_per_shard
            .into_iter()
            .fold((0, 0), |(i1, b1), (i2, b2)| (i1 + i2, b1 + b2));
        StateStorageUsage::new(
            (self.usage().items() as i64 + items_delta) as usize,
            (self.usage().bytes() as i64 + bytes_delta) as usize,
        )
    }

    fn usage_delta_for_shard<'kv>(
        cache: &StateCacheShard,
        overlay: &LayeredMap<StateKey, StateSlot>,
        updates: &HashMap<&'kv StateKey, StateUpdateRef<'kv>>,
    ) -> (i64, i64) {
        let mut items_delta: i64 = 0;
        let mut bytes_delta: i64 = 0;
        for (k, v) in updates {
            let key_size = k.size();
            if let Some(value) = v.state_op.as_state_value_opt() {
                items_delta += 1;
                bytes_delta += (key_size + value.size()) as i64;
            }

            // TODO(aldenhu): avoid cloning the state value (by not using DashMap)
            // n.b. all updated state items must be read and recorded in the state cache,
            // otherwise we can't calculate the correct usage.
            let old_slot = overlay
                .get(k)
                .or_else(|| cache.get(*k).map(|entry| entry.value().clone()))
                .expect("Must cache read");
            if old_slot.is_occupied() {
                items_delta -= 1;
                bytes_delta -= (key_size + old_slot.size()) as i64;
            }
        }
        (items_delta, bytes_delta)
    }
}

/// At a given version, the state and the last checkpoint state at or before the version.
#[derive(Clone, Debug, Deref)]
pub struct LedgerState {
    last_checkpoint: State,
    #[deref]
    latest: State,
}

impl LedgerState {
    pub fn new(latest: State, last_checkpoint: State) -> Self {
        assert!(latest.is_descendant_of(&latest));

        Self {
            latest,
            last_checkpoint,
        }
    }

    pub fn new_empty() -> Self {
        let state = State::new_empty();
        Self::new(state.clone(), state)
    }

    pub fn latest(&self) -> &State {
        &self.latest
    }

    pub fn last_checkpoint(&self) -> &State {
        &self.last_checkpoint
    }

    pub fn is_checkpoint(&self) -> bool {
        self.latest.is_the_same(&self.last_checkpoint)
    }

    /// In the execution pipeline, at the time of state update, the reads during execution
    /// have already been recorded.
    pub fn update_with_memorized_reads(
        &self,
        persisted_hot_view: Arc<dyn HotStateView>,
        persisted_snapshot: &State,
        updates: &StateUpdateRefs,
        reads: &ShardedStateCache,
    ) -> LedgerState {
        let _timer = TIMER.timer_with(&["ledger_state__update"]);

        let last_checkpoint = if let Some(u) = &updates.for_last_checkpoint {
            let mut per_version = Vec::new();
            for i in 0..NUM_STATE_SHARDS {
                let s = &updates.per_version.shards[i];
                let p = s.partition_point(|x| x.1.version < u.next_version());
                println!("shard id: {i}. s.len(): {}. p: {p}", s.len());
                per_version.push(&s[..p]);
            }
            self.latest().update(
                Arc::clone(&persisted_hot_view),
                persisted_snapshot,
                u,
                per_version,
                reads,
            )
        } else {
            self.last_checkpoint.clone()
        };

        let base_of_latest = if updates.for_last_checkpoint.is_none() {
            self.latest()
        } else {
            &last_checkpoint
        };
        let latest = if let Some(u) = &updates.for_latest {
            let mut per_version = Vec::new();
            for i in 0..NUM_STATE_SHARDS {
                let s = &updates.per_version.shards[i];
                let p = s.partition_point(|x| x.1.version < u.first_version());
                per_version.push(&s[p..]);
            }
            base_of_latest.update(
                persisted_hot_view,
                persisted_snapshot,
                u,
                per_version,
                reads,
            )
        } else {
            base_of_latest.clone()
        };

        LedgerState::new(latest, last_checkpoint)
    }

    /// Old values of the updated keys are read from the DbReader at the version of the
    /// `persisted_snapshot`.
    pub fn update_with_db_reader(
        &self,
        persisted_snapshot: &State,
        hot_state: Arc<dyn HotStateView>,
        updates: &StateUpdateRefs,
        reader: Arc<dyn DbReader>,
    ) -> Result<(LedgerState, ShardedStateCache)> {
        let state_view = CachedStateView::new_impl(
            StateViewId::Miscellaneous,
            reader,
            Arc::clone(&hot_state),
            persisted_snapshot.clone(),
            self.latest().clone(),
        );
        state_view.prime_cache(updates)?;

        let updated = self.update_with_memorized_reads(
            hot_state,
            persisted_snapshot,
            updates,
            state_view.memorized_reads(),
        );
        let state_reads = state_view.into_memorized_reads();
        Ok((updated, state_reads))
    }

    pub fn is_the_same(&self, other: &Self) -> bool {
        self.latest.is_the_same(&other.latest)
            && self.last_checkpoint.is_the_same(&other.last_checkpoint)
    }
}
