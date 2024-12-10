// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::TIMER,
    state_store::{
        state_delta::StateDelta,
        state_update::{StateUpdate, StateUpdateRef},
        state_update_ref_map::BatchedStateUpdateRefs,
        state_view::cached_state_view::{ShardedStateCache, StateCacheShard},
        NUM_STATE_SHARDS,
    },
};
use aptos_experimental_layered_map::{LayeredMap, MapLayer};
use aptos_metrics_core::TimerHelper;
use aptos_types::{
    state_store::{state_key::StateKey, state_storage_usage::StateStorageUsage},
    transaction::Version,
};
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
    pub shards: Arc<[MapLayer<StateKey, StateUpdate>; NUM_STATE_SHARDS]>,
    /// The total usage of the state at the current version.
    usage: StateStorageUsage,
}

impl State {
    pub fn new_empty() -> Self {
        Self {
            next_version: 0,
            shards: Arc::new(arr_macro::arr![MapLayer::new_family("pre_genesis_state"); 16]),
            usage: StateStorageUsage::zero(),
        }
    }

    pub fn new_empty_at_version(version: Option<Version>, usage: StateStorageUsage) -> Self {
        Self {
            next_version: version.map_or(0, |v| v + 1),
            shards: Arc::new(arr_macro::arr![MapLayer::new_family("state"); 16]),
            usage,
        }
    }

    pub fn new(
        next_version: Version,
        shards: Arc<[MapLayer<StateKey, StateUpdate>; NUM_STATE_SHARDS]>,
        usage: StateStorageUsage,
    ) -> Self {
        Self {
            next_version,
            shards,
            usage,
        }
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

    pub fn shards(&self) -> &[MapLayer<StateKey, StateUpdate>; NUM_STATE_SHARDS] {
        &self.shards
    }

    pub fn make_delta(&self, base: &State) -> StateDelta {
        self.clone().into_delta(base.clone())
    }

    pub fn into_delta(self, base: State) -> StateDelta {
        StateDelta::new(base, self)
    }

    pub fn is_the_same(&self, rhs: &Self) -> bool {
        Arc::ptr_eq(&self.shards, &rhs.shards)
    }

    // FIXME(aldenhu): check call sites, are we doing duplicate checks?
    pub fn is_descendant_of(&self, rhs: &State) -> bool {
        self.shards[0].is_descendant_of(&rhs.shards[0])
    }

    pub fn update(
        &self,
        persisted: &State,
        updates: &BatchedStateUpdateRefs,
        state_cache: &ShardedStateCache,
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
        //    it is overlayed on top of the cache.
        assert!(self.next_version() >= state_cache.next_version());

        let speculative_state = self.make_delta(persisted);
        let (shards, usage_delta_per_shard): (Vec<_>, Vec<_>) = (
            state_cache.shards.as_slice(),
            speculative_state.shards.as_slice(),
            updates.shards.as_slice(),
        )
            .into_par_iter()
            .map(|(cache, overlay, updates)| {
                (
                    // FIXME(aldenhu): change interface to take iter of ref
                    overlay.new_layer(
                        &updates
                            .iter()
                            .map(|(k, u)| ((*k).clone(), (*u).cloned()))
                            .collect_vec(),
                    ),
                    Self::usage_delta_for_shard(cache, overlay, updates),
                )
            })
            .unzip();
        let shards = Arc::new(shards.try_into().expect("Known to be 16 shards."));
        let usage = self.update_usage(usage_delta_per_shard);

        State::new(updates.next_version(), shards, usage)
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
        overlay: &LayeredMap<StateKey, StateUpdate>,
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

            // n.b. all updated state items must be read and recorded in the state cache,
            // otherwise we can't calculate the correct usage.
            // TODO(aldenhu): avoid cloning state value
            let old_value = overlay
                .get(k)
                .map(|update| update.value)
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

    pub fn update<'kv>(
        &self,
        persisted_snapshot: &State,
        updates_for_last_checkpoint: Option<&BatchedStateUpdateRefs<'kv>>,
        updates_for_latest: Option<&BatchedStateUpdateRefs<'kv>>,
        state_cache: &ShardedStateCache,
    ) -> LedgerState {
        let _timer = TIMER.timer_with(&["ledger_state__update"]);

        let last_checkpoint = if let Some(updates) = updates_for_last_checkpoint {
            self.latest()
                .update(persisted_snapshot, updates, state_cache)
        } else {
            self.last_checkpoint.clone()
        };

        let base_of_latest = if updates_for_last_checkpoint.is_none() {
            self.latest()
        } else {
            &last_checkpoint
        };
        let latest = if let Some(updates) = updates_for_latest {
            base_of_latest.update(persisted_snapshot, updates, state_cache)
        } else {
            base_of_latest.clone()
        };

        LedgerState::new(latest, last_checkpoint)
    }
}
