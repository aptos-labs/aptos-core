// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::TIMER,
    state_store::{
        state_delta::StateDelta,
        state_update_refs::{BatchedStateUpdateRefs, StateUpdateRefs},
        state_view::{
            cached_state_view::{
                CachedStateView, HotStateShardRefreshes, ShardedStateCache, StateCacheShard,
            },
            hot_state_view::HotStateView,
        },
        versioned_state_value::{DbStateUpdate, StateUpdateRef},
        NUM_STATE_SHARDS,
    },
    DbReader,
};
use anyhow::Result;
use aptos_experimental_layered_map::{LayeredMap, MapLayer};
use aptos_infallible::duration_since_epoch;
use aptos_metrics_core::TimerHelper;
use aptos_types::{
    state_store::{state_key::StateKey, state_storage_usage::StateStorageUsage, StateViewId},
    transaction::Version,
};
use arr_macro::arr;
use derive_more::Deref;
use itertools::Itertools;
use rayon::prelude::*;
use std::{collections::HashMap, sync::Arc};

/// Represents the blockchain state at a given version.
/// n.b. the state can be either persisted or speculative.
#[derive(Clone, Debug)]
pub struct State {
    /// The next version. If this is 0, the state is the "pre-genesis" empty state.
    next_version: Version,
    /// The updates made to the state at the current version.
    ///  N.b. this is not directly iteratable, one needs to make a `StateDelta`
    ///       between this and a `base_version` to list the updates or create a
    ///       new `State` at a descendant version.
    shards: Arc<[MapLayer<StateKey, DbStateUpdate>; NUM_STATE_SHARDS]>,
    /// The total usage of the state at the current version.
    usage: StateStorageUsage,
}

impl State {
    pub fn new_with_updates(
        version: Option<Version>,
        shards: Arc<[MapLayer<StateKey, DbStateUpdate>; NUM_STATE_SHARDS]>,
        usage: StateStorageUsage,
    ) -> Self {
        Self {
            next_version: version.map_or(0, |v| v + 1),
            shards,
            usage,
        }
    }

    pub fn new_at_version(version: Option<Version>, usage: StateStorageUsage) -> Self {
        Self::new_with_updates(
            version,
            Arc::new(arr_macro::arr![MapLayer::new_family("state"); 16]),
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

    pub fn shards(&self) -> &[MapLayer<StateKey, DbStateUpdate>; NUM_STATE_SHARDS] {
        &self.shards
    }

    pub fn make_delta(&self, base: &State) -> StateDelta {
        let _timer = TIMER.timer_with(&["state__make_delta"]);
        self.clone().into_delta(base.clone())
    }

    pub fn into_delta(self, base: State) -> StateDelta {
        StateDelta::new(base, self)
    }

    pub fn is_the_same(&self, rhs: &Self) -> bool {
        Arc::ptr_eq(&self.shards, &rhs.shards)
    }

    pub fn is_descendant_of(&self, rhs: &State) -> bool {
        self.shards[0].is_descendant_of(&rhs.shards[0])
    }

    pub fn update(
        &self,
        persisted: &State,
        updates: &BatchedStateUpdateRefs,
        state_cache: &ShardedStateCache,
        hot_state_refreshes: &mut [Option<&HotStateShardRefreshes>; NUM_STATE_SHARDS],
    ) -> Self {
        let _timer = TIMER.timer_with(&["state__update"]);

        // 1. The update batch must begin at self.next_version().
        assert_eq!(self.next_version(), updates.first_version);
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
        let access_time_secs = duration_since_epoch().as_secs() as u32;
        let (shards, usage_delta_per_shard): (Vec<_>, Vec<_>) = (
            state_cache.shards.as_slice(),
            hot_state_refreshes.as_slice(),
            overlay.shards.as_slice(),
            updates.shards.as_slice(),
        )
            .into_par_iter()
            .map(|(cache, hot_state_refreshes, overlay, updates)| {
                let mut new_items = hot_state_refreshes
                    .as_ref()
                    .map(|refreshes| {
                        refreshes
                            .iter()
                            .map(|kv| (kv.key().clone(), kv.value().clone()))
                            .collect_vec()
                    })
                    .unwrap_or(vec![]);
                new_items.extend(
                    updates
                        .iter()
                        .map(|(k, u)| ((*k).clone(), u.to_db_state_update(access_time_secs))),
                );

                (
                    // TODO(aldenhu): change interface to take iter of ref
                    overlay.new_layer(&new_items),
                    Self::usage_delta_for_shard(cache, overlay, updates),
                )
            })
            .unzip();
        let shards = Arc::new(shards.try_into().expect("Known to be 16 shards."));
        let usage = self.update_usage(usage_delta_per_shard);

        // Consume hot_state_refreshes, so for each chunk it's taken into account only once.
        *hot_state_refreshes = arr![None; 16];

        State::new_with_updates(updates.last_version(), shards, usage)
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
        overlay: &LayeredMap<StateKey, DbStateUpdate>,
        updates: &HashMap<&'kv StateKey, StateUpdateRef<'kv>>,
    ) -> (i64, i64) {
        let mut items_delta: i64 = 0;
        let mut bytes_delta: i64 = 0;
        for (k, v) in updates {
            let key_size = k.size();
            if let Some(value) = v.value {
                items_delta += 1;
                bytes_delta += (key_size + value.size()) as i64;
            }

            // TODO(aldenhu): avoid cloning the state value (by not using DashMap)
            // n.b. all updated state items must be read and recorded in the state cache,
            // otherwise we can't calculate the correct usage.
            let old_value = overlay
                .get(k)
                .map(|update| {
                    update
                        .value
                        .and_then(|db_val| db_val.into_state_value_opt())
                })
                .or_else(|| cache.get(k).map(|entry| entry.value().to_state_value_opt()))
                .expect("Must cache read");
            if let Some(old_v) = old_value {
                items_delta -= 1;
                bytes_delta -= (key_size + old_v.size()) as i64;
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
        persisted_snapshot: &State,
        updates: &StateUpdateRefs,
        reads: &ShardedStateCache,
    ) -> LedgerState {
        let _timer = TIMER.timer_with(&["ledger_state__update"]);

        let mut iter = reads.hot_state_refreshes.iter();
        let mut cache_refreshes = arr![Some(iter.next().expect("Known to be 16 shards")); 16];

        let last_checkpoint = if let Some(updates) = &updates.for_last_checkpoint {
            self.latest()
                .update(persisted_snapshot, updates, reads, &mut cache_refreshes)
        } else {
            self.last_checkpoint.clone()
        };

        let base_of_latest = if updates.for_last_checkpoint.is_none() {
            self.latest()
        } else {
            &last_checkpoint
        };
        let latest = if let Some(updates) = &updates.for_latest {
            base_of_latest.update(persisted_snapshot, updates, reads, &mut cache_refreshes)
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
            hot_state,
            persisted_snapshot.clone(),
            self.latest().clone(),
        );
        state_view.prime_cache(updates)?;

        let updated = self.update_with_memorized_reads(
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
