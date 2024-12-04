// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::TIMER,
    state_store::{
        per_version_state_update_refs::PerVersionStateUpdateRefs,
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
use itertools::{izip, Itertools};
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

    pub fn into_delta(self, _base: State) -> StateDelta {
        // FIXME(aldenhu)
        todo!()
    }

    pub fn make_delta(&self, _base: &State) -> StateDelta {
        // FIXME(aldenhu)
        todo!()
    }

    pub fn is_the_same(&self, _rhs: &Self) -> bool {
        // FIXME(aldenhu)
        todo!()
    }

    pub fn is_family(&self, _rhs: &State) -> bool {
        // FIXME(aldenhu)
        todo!()
    }

    // FIXME(aldenhu): check call sites, are we doing duplicate checks?
    pub fn follows(&self, _rhs: &State) -> bool {
        // FIXME(aldenhu)
        todo!()
    }

    pub fn count_items_heavy(&self) -> usize {
        // FIXME(aldenhu)
        todo!()
    }

    pub fn update(
        &self,
        persisted: &State,
        updates: &BatchedStateUpdateRefs,
        state_cache: &ShardedStateCache,
    ) -> Self {
        let _timer = TIMER.timer_with(&["state_delta__update"]);

        // 1. The update batch must begin at self.next_version().
        assert_eq!(self.next_version(), updates.first_version);
        // 2. The cache must be at a version equal or newer than `persisted`, otherwise
        //    updates between the cached version and the persisted version are potentially
        //    missed during the usage calculation.
        assert!(persisted.next_version() <= state_cache.next_version());
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
        assert!(latest.follows(&latest));

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

    pub fn update(
        &self,
        persisted_snapshot: &State,
        per_version_updates: &PerVersionStateUpdateRefs,
        last_checkpoint_index: Option<usize>,
        state_cache: &ShardedStateCache,
    ) -> LedgerState {
        let _timer = TIMER.timer_with(&["ledger_state__update"]);

        let (updates_for_last_checkpoint, updates_for_latest) =
            Self::collect_updates(per_version_updates, last_checkpoint_index);

        let last_checkpoint = if let Some(updates) = &updates_for_last_checkpoint {
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
        let latest = base_of_latest.update(persisted_snapshot, &updates_for_latest, state_cache);

        LedgerState::new(latest, last_checkpoint)
    }

    fn collect_updates<'kv>(
        state_update_refs: &PerVersionStateUpdateRefs<'kv>,
        last_checkpoint_index: Option<usize>,
    ) -> (
        Option<BatchedStateUpdateRefs<'kv>>,
        BatchedStateUpdateRefs<'kv>,
    ) {
        let _timer = TIMER.timer_with(&["ledger_state__collect_updates"]);

        let mut shard_iters = state_update_refs
            .shards
            .iter()
            .map(|shard| shard.iter().cloned())
            .collect::<Vec<_>>();

        let mut first_version = state_update_refs.first_version;
        let mut num_versions = state_update_refs.num_versions;
        let updates_for_last_checkpoint = last_checkpoint_index.map(|idx| {
            let ret = Self::collect_some_updates(first_version, idx + 1, &mut shard_iters);
            first_version += idx as Version + 1;
            num_versions -= idx + 1;
            ret
        });
        let updates_for_latest =
            Self::collect_some_updates(first_version, num_versions, &mut shard_iters);

        // Assert that all updates are consumed.
        assert!(shard_iters.iter_mut().all(|iter| iter.next().is_none()));

        (updates_for_last_checkpoint, updates_for_latest)
    }

    fn collect_some_updates<'kv>(
        first_version: Version,
        num_versions: usize,
        shard_iters: &mut [impl Iterator<Item = (&'kv StateKey, StateUpdateRef<'kv>)> + Clone],
    ) -> BatchedStateUpdateRefs<'kv> {
        let mut ret = BatchedStateUpdateRefs::new_empty(first_version, num_versions);
        // exclusive
        let end_version = first_version + num_versions as Version;
        izip!(shard_iters, &mut ret.shards).for_each(|(shard_iter, dedupped)| {
            dedupped.extend(
                shard_iter
                    // n.b. take_while_ref so that in the next step we can process the rest of the entries from the iters.
                    .take_while_ref(|(_k, u)| u.version < end_version),
            )
        });
        ret
    }
}
