// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    metrics::TIMER,
    state_store::{
        hot_state::HotStateLRU,
        state_delta::StateDelta,
        state_update_refs::{BatchedStateUpdateRefs, PerVersionStateUpdateRefs, StateUpdateRefs},
        state_view::{
            cached_state_view::{
                CachedStateView, PrimingPolicy, ShardedStateCache, StateCacheShard,
            },
            hot_state_view::HotStateView,
        },
        versioned_state_value::StateUpdateRef,
        HotStateShardUpdates, HotStateUpdates,
    },
    DbReader,
};
use anyhow::{bail, Result};
use aptos_config::config::HotStateConfig;
use aptos_experimental_layered_map::{LayeredMap, MapLayer};
use aptos_logger::warn;
use aptos_metrics_core::TimerHelper;
use aptos_types::{
    state_store::{
        hot_state::HotStateValue, state_key::StateKey, state_slot::StateSlot,
        state_storage_usage::StateStorageUsage, StateViewId, NUM_STATE_SHARDS,
    },
    transaction::Version,
};
use arr_macro::arr;
use derive_more::Deref;
use itertools::Itertools;
use rayon::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    num::NonZeroUsize,
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct HotStateMetadata {
    latest: Option<StateKey>,
    oldest: Option<StateKey>,
    num_items: usize,
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
    hot_state_config: HotStateConfig,
}

impl State {
    pub fn new_with_updates(
        version: Option<Version>,
        shards: Arc<[MapLayer<StateKey, StateSlot>; NUM_STATE_SHARDS]>,
        hot_state_metadata: [HotStateMetadata; NUM_STATE_SHARDS],
        usage: StateStorageUsage,
        hot_state_config: HotStateConfig,
    ) -> Self {
        Self {
            next_version: version.map_or(0, |v| v + 1),
            shards,
            hot_state_metadata,
            usage,
            hot_state_config,
        }
    }

    pub fn new_at_version(
        version: Option<Version>,
        usage: StateStorageUsage,
        hot_state_config: HotStateConfig,
    ) -> Self {
        Self::new_with_updates(
            version,
            Arc::new(arr![MapLayer::new_family("state"); 16]),
            arr![HotStateMetadata::new(); 16],
            usage,
            hot_state_config,
        )
    }

    pub fn new_empty(hot_state_config: HotStateConfig) -> Self {
        Self::new_at_version(None, StateStorageUsage::zero(), hot_state_config)
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

    pub fn into_delta(self, base: State) -> StateDelta {
        StateDelta::new(base, self)
    }

    pub fn is_the_same(&self, rhs: &Self) -> bool {
        Arc::ptr_eq(&self.shards, &rhs.shards)
    }

    pub fn is_descendant_of(&self, rhs: &State) -> bool {
        self.shards[0].is_descendant_of(&rhs.shards[0])
    }

    pub fn latest_hot_key(&self, shard_id: usize) -> Option<StateKey> {
        self.hot_state_metadata[shard_id].latest.clone()
    }

    pub fn oldest_hot_key(&self, shard_id: usize) -> Option<StateKey> {
        self.hot_state_metadata[shard_id].oldest.clone()
    }

    pub fn num_hot_items(&self, shard_id: usize) -> usize {
        self.hot_state_metadata[shard_id].num_items
    }

    fn update(
        &self,
        persisted_hot_state: Arc<dyn HotStateView>,
        persisted: &State,
        batched_updates: &BatchedStateUpdateRefs,
        per_version_updates: &PerVersionStateUpdateRefs,
        all_checkpoint_versions: &[Version],
        state_cache: &ShardedStateCache,
    ) -> Result<(Self, [HotStateShardUpdates; NUM_STATE_SHARDS])> {
        let _timer = TIMER.timer_with(&["state__update"]);

        // 1. The update batch must begin at self.next_version().
        assert_eq!(self.next_version(), batched_updates.first_version);
        assert_eq!(self.next_version(), per_version_updates.first_version);
        // 2. The cache must be at a version equal or newer than `persisted`, otherwise
        //    updates between the cached version and the persisted version are potentially
        //    missed during the usage calculation.
        if persisted.next_version() > state_cache.next_version() {
            let msg = format!(
                "Persisted version ({}) is ahead of cache version ({}), possibly due to a fork.",
                persisted.next_version(),
                state_cache.next_version(),
            );
            warn!("{}", msg);
            bail!("{}", msg);
        }
        // 3. `self` must be at a version equal or newer than the cache, because we assume
        //    it is overlaid on top of the cache.
        assert!(self.next_version() >= state_cache.next_version());

        let overlay = self.make_delta(persisted);
        let (((shards, new_metadata), usage_delta_per_shard), hot_state_updates): (
            ((Vec<_>, Vec<_>), Vec<_>),
            Vec<_>,
        ) = (
            state_cache.shards.as_slice(),
            overlay.shards.as_slice(),
            self.hot_state_metadata.as_slice(),
            batched_updates.shards.as_slice(),
            per_version_updates.shards.as_slice(),
        )
            .into_par_iter()
            .map(
                |(cache, overlay, hot_metadata, batched_updates, per_version)| {
                    let mut lru = HotStateLRU::new(
                        NonZeroUsize::new(self.hot_state_config.max_items_per_shard).unwrap(),
                        Arc::clone(&persisted_hot_state),
                        overlay,
                        hot_metadata.latest.clone(),
                        hot_metadata.oldest.clone(),
                        hot_metadata.num_items,
                    );
                    let mut all_updates = per_version.iter();
                    let mut insertions = HashMap::new();
                    let mut evictions = HashSet::new();
                    for ckpt_version in all_checkpoint_versions {
                        for (key, update) in
                            all_updates.take_while_ref(|(_k, u)| u.version <= *ckpt_version)
                        {
                            evictions.remove(*key);
                            if let Some(hot_state_value) = Self::apply_one_update(
                                &mut lru,
                                overlay,
                                cache,
                                key,
                                update,
                                self.hot_state_config.refresh_interval_versions,
                            ) {
                                insertions.insert((*key).clone(), hot_state_value);
                            }
                        }
                        // Only evict at the checkpoints.
                        evictions.extend(lru.maybe_evict().into_iter().map(|(key, slot)| {
                            insertions.remove(&key);
                            assert!(slot.is_hot());
                            key
                        }));
                    }
                    for (key, update) in all_updates {
                        evictions.remove(*key);
                        if let Some(hot_state_value) = Self::apply_one_update(
                            &mut lru,
                            overlay,
                            cache,
                            key,
                            update,
                            self.hot_state_config.refresh_interval_versions,
                        ) {
                            insertions.insert((*key).clone(), hot_state_value);
                        }
                    }

                    let (new_items, new_head, new_tail, new_num_items) = lru.into_updates();
                    let new_items = new_items.into_iter().collect_vec();

                    // TODO(aldenhu): change interface to take iter of ref
                    let new_layer = overlay.new_layer(&new_items);
                    let new_metadata = HotStateMetadata {
                        latest: new_head,
                        oldest: new_tail,
                        num_items: new_num_items,
                    };
                    let new_usage = Self::usage_delta_for_shard(cache, overlay, batched_updates);
                    (
                        ((new_layer, new_metadata), new_usage),
                        HotStateShardUpdates::new(insertions, evictions),
                    )
                },
            )
            .unzip();
        let shards = Arc::new(shards.try_into().expect("Known to be 16 shards."));
        let new_metadata = new_metadata.try_into().expect("Known to be 16 shards.");
        let usage = self.update_usage(usage_delta_per_shard);
        let hot_state_updates = hot_state_updates
            .try_into()
            .expect("Known to be 16 shards.");

        // TODO(HotState): extract and pass new hot state onchain config if needed.
        Ok((
            State::new_with_updates(
                batched_updates.last_version(),
                shards,
                new_metadata,
                usage,
                self.hot_state_config,
            ),
            hot_state_updates,
        ))
    }

    /// Applies the update the returns the `HotStateValue` that will later go into the hot state
    /// Merkle tree. `None` if the op is `MakeHot` and it's determined that refresh is not
    /// necessary.
    fn apply_one_update(
        lru: &mut HotStateLRU,
        overlay: &LayeredMap<StateKey, StateSlot>,
        read_cache: &StateCacheShard,
        key: &StateKey,
        update: &StateUpdateRef,
        refresh_interval: Version,
    ) -> Option<HotStateValue> {
        if let Some(state_value_opt) = update.state_op.as_state_value_opt() {
            lru.insert((*key).clone(), update.to_result_slot().unwrap());
            return Some(HotStateValue::new(state_value_opt.cloned(), update.version));
        }

        if let Some(mut slot) = lru.get_slot(key) {
            let mut refreshed = true;
            let slot_to_insert = if slot.is_hot() {
                if slot.expect_hot_since_version() + refresh_interval <= update.version {
                    slot.refresh(update.version);
                } else {
                    refreshed = false;
                }
                slot
            } else {
                slot.to_hot(update.version)
            };
            if refreshed {
                let ret = HotStateValue::clone_from_slot(&slot_to_insert);
                lru.insert((*key).clone(), slot_to_insert);
                Some(ret)
            } else {
                None
            }
        } else {
            let slot = Self::expect_old_slot(overlay, read_cache, key);
            assert!(slot.is_cold());
            let slot = slot.to_hot(update.version);
            let ret = HotStateValue::clone_from_slot(&slot);
            lru.insert((*key).clone(), slot);
            Some(ret)
        }
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
            let state_value_opt = match v.state_op.as_state_value_opt() {
                Some(value_opt) => value_opt,
                None => continue,
            };

            let key_size = k.size();
            if let Some(value) = state_value_opt {
                items_delta += 1;
                bytes_delta += (key_size + value.size()) as i64;
            }

            // n.b. all updated state items must be read and recorded in the state cache,
            // otherwise we can't calculate the correct usage.
            let old_slot = Self::expect_old_slot(overlay, cache, k);
            if old_slot.is_occupied() {
                items_delta -= 1;
                bytes_delta -= (key_size + old_slot.size()) as i64;
            }
        }
        (items_delta, bytes_delta)
    }

    fn expect_old_slot(
        overlay: &LayeredMap<StateKey, StateSlot>,
        cache: &StateCacheShard,
        key: &StateKey,
    ) -> StateSlot {
        if let Some(slot) = overlay.get(key) {
            return slot;
        }

        // TODO(aldenhu): avoid cloning the state value (by not using DashMap)
        cache
            .get(key)
            .unwrap_or_else(|| panic!("Key {:?} must exist in the cache.", key))
            .value()
            .clone()
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

    pub fn new_empty(hot_state_config: HotStateConfig) -> Self {
        let state = State::new_empty(hot_state_config);
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
    ) -> Result<(LedgerState, HotStateUpdates)> {
        let _timer = TIMER.timer_with(&["ledger_state__update"]);

        let mut all_hot_state_updates = HotStateUpdates::new_empty();
        let last_checkpoint = if let Some(batched) = updates.for_last_checkpoint_batched() {
            let per_version = updates
                .for_last_checkpoint_per_version()
                .expect("Both per-version and batched updates should exist.");
            let (new_ckpt, hot_state_updates) = self.latest().update(
                Arc::clone(&persisted_hot_view),
                persisted_snapshot,
                batched,
                per_version,
                updates.all_checkpoint_versions(),
                reads,
            )?;
            all_hot_state_updates.for_last_checkpoint = Some(hot_state_updates);
            new_ckpt
        } else {
            self.last_checkpoint.clone()
        };

        let base_of_latest = if updates.for_last_checkpoint_batched().is_none() {
            self.latest()
        } else {
            &last_checkpoint
        };
        let latest = if let Some(batched) = updates.for_latest_batched() {
            let per_version = updates
                .for_latest_per_version()
                .expect("Both per-version and batched updates should exist.");
            let (new_latest, hot_state_updates) = base_of_latest.update(
                persisted_hot_view,
                persisted_snapshot,
                batched,
                per_version,
                &[],
                reads,
            )?;
            all_hot_state_updates.for_latest = Some(hot_state_updates);
            new_latest
        } else {
            base_of_latest.clone()
        };

        Ok((
            LedgerState::new(latest, last_checkpoint),
            all_hot_state_updates,
        ))
    }

    /// Old values of the updated keys are read from the DbReader at the version of the
    /// `persisted_snapshot`.
    pub fn update_with_db_reader(
        &self,
        persisted_snapshot: &State,
        hot_state: Arc<dyn HotStateView>,
        updates: &StateUpdateRefs,
        reader: Arc<dyn DbReader>,
    ) -> Result<(LedgerState, ShardedStateCache, HotStateUpdates)> {
        let state_view = CachedStateView::new_impl(
            StateViewId::Miscellaneous,
            reader,
            Arc::clone(&hot_state),
            persisted_snapshot.clone(),
            self.latest().clone(),
        );
        state_view.prime_cache(updates, PrimingPolicy::All)?;

        let (updated, hot_state_updates) = self.update_with_memorized_reads(
            hot_state,
            persisted_snapshot,
            updates,
            state_view.memorized_reads(),
        )?;
        let state_reads = state_view.into_memorized_reads();
        Ok((updated, state_reads, hot_state_updates))
    }

    pub fn is_the_same(&self, other: &Self) -> bool {
        self.latest.is_the_same(&other.latest)
            && self.last_checkpoint.is_the_same(&other.last_checkpoint)
    }
}
