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
        HotEvictionOp, HotInsertionOp, HotStateShardUpdates, HotStateUpdates,
    },
    DbReader,
};
use anyhow::{bail, Result};
use aptos_config::config::HotStateConfig;
use aptos_crypto::HashValue;
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
use std::{collections::HashMap, num::NonZeroUsize, sync::Arc};

#[derive(Clone, Debug, Default)]
pub struct HotStateMetadata {
    latest: Option<HashValue>,
    oldest: Option<HashValue>,
    num_items: usize,
    total_value_bytes: usize,
}

impl HotStateMetadata {
    pub fn new(
        latest: Option<HashValue>,
        oldest: Option<HashValue>,
        num_items: usize,
        total_value_bytes: usize,
    ) -> Self {
        Self {
            latest,
            oldest,
            num_items,
            total_value_bytes,
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
    shards: Arc<[MapLayer<HashValue, StateSlot>; NUM_STATE_SHARDS]>,
    hot_state_metadata: [HotStateMetadata; NUM_STATE_SHARDS],
    /// The total usage of the state at the current version.
    usage: StateStorageUsage,
    hot_state_config: HotStateConfig,
}

impl State {
    pub fn new_with_updates(
        version: Option<Version>,
        shards: Arc<[MapLayer<HashValue, StateSlot>; NUM_STATE_SHARDS]>,
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
        Self::new_at_version_with_hot_state_metadata(
            version,
            usage,
            hot_state_config,
            arr![HotStateMetadata::default(); 16],
        )
    }

    pub fn new_at_version_with_hot_state_metadata(
        version: Option<Version>,
        usage: StateStorageUsage,
        hot_state_config: HotStateConfig,
        hot_state_metadata: [HotStateMetadata; NUM_STATE_SHARDS],
    ) -> Self {
        Self::new_with_updates(
            version,
            Arc::new(arr![MapLayer::new_family("state"); 16]),
            hot_state_metadata,
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

    pub fn hot_state_config(&self) -> HotStateConfig {
        self.hot_state_config
    }

    pub fn shards(&self) -> &[MapLayer<HashValue, StateSlot>; NUM_STATE_SHARDS] {
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

    /// Returns true if `self` can serve as the base (older) side of a `StateDelta`
    /// with `current` as the newer side.
    pub fn can_be_delta_base_of(&self, current: &State) -> bool {
        self.shards
            .iter()
            .zip(current.shards.iter())
            .all(|(base_shard, top_shard)| top_shard.can_view_after(base_shard))
    }

    pub fn latest_hot_key(&self, shard_id: usize) -> Option<HashValue> {
        self.hot_state_metadata[shard_id].latest
    }

    pub fn oldest_hot_key(&self, shard_id: usize) -> Option<HashValue> {
        self.hot_state_metadata[shard_id].oldest
    }

    pub fn num_hot_items(&self, shard_id: usize) -> usize {
        self.hot_state_metadata[shard_id].num_items
    }

    pub fn hot_value_bytes(&self, shard_id: usize) -> usize {
        self.hot_state_metadata[shard_id].total_value_bytes
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
                        hot_metadata.latest,
                        hot_metadata.oldest,
                        hot_metadata.num_items,
                        hot_metadata.total_value_bytes,
                    );
                    let mut all_updates = per_version.iter();
                    let mut shard_updates = HotStateShardUpdates::default();
                    for ckpt_version in all_checkpoint_versions {
                        for (key, update) in
                            all_updates.take_while_ref(|(_k, u)| u.version <= *ckpt_version)
                        {
                            let key_hash = *key.crypto_hash_ref();
                            if let Some(op) = Self::apply_one_update(
                                &mut lru,
                                overlay,
                                cache,
                                key,
                                update,
                                self.hot_state_config.refresh_interval_versions,
                            ) {
                                shard_updates.insert(key_hash, op);
                            }
                        }
                        // Only evict at the checkpoints.
                        for (key_hash, slot) in lru.maybe_evict() {
                            assert!(slot.is_hot());
                            shard_updates
                                .evict(key_hash, HotEvictionOp {
                                    eviction_version: *ckpt_version,
                                    superseded_version: Some(slot.expect_hot_since_version()),
                                })
                                .expect("LRU eviction must succeed.");
                        }
                    }
                    for (key, update) in all_updates {
                        let key_hash = *key.crypto_hash_ref();
                        if let Some(op) = Self::apply_one_update(
                            &mut lru,
                            overlay,
                            cache,
                            key,
                            update,
                            self.hot_state_config.refresh_interval_versions,
                        ) {
                            shard_updates.insert(key_hash, op);
                        }
                    }

                    let (new_items, new_head, new_tail, new_num_items, new_total_value_bytes) =
                        lru.into_updates();
                    let new_items = new_items.into_iter().collect_vec();

                    // TODO(aldenhu): change interface to take iter of ref
                    let new_layer = overlay.new_layer(&new_items);
                    let new_metadata = HotStateMetadata {
                        latest: new_head,
                        oldest: new_tail,
                        num_items: new_num_items,
                        total_value_bytes: new_total_value_bytes,
                    };
                    let new_usage = Self::usage_delta_for_shard(cache, overlay, batched_updates);
                    (((new_layer, new_metadata), new_usage), shard_updates)
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

    /// Applies one update to the LRU. Returns `None` if the op is `MakeHot` and refresh is not
    /// necessary.
    fn apply_one_update(
        lru: &mut HotStateLRU,
        overlay: &LayeredMap<HashValue, StateSlot>,
        read_cache: &StateCacheShard,
        key: &StateKey,
        update: &StateUpdateRef,
        refresh_interval: Version,
    ) -> Option<HotInsertionOp> {
        let key_hash = *key.crypto_hash_ref();
        if let Some(state_value_opt) = update.state_op.as_state_value_opt() {
            let superseded_version =
                lru.insert(key, update.to_result_slot((*key).clone()).unwrap());
            return Some(HotInsertionOp {
                state_key: (*key).clone(),
                value: HotStateValue::new(state_value_opt.cloned(), update.version),
                value_version: state_value_opt.map(|_| update.version),
                superseded_version,
            });
        }

        if let Some(mut slot) = lru.get_slot(&key_hash) {
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
                let value_version = slot_to_insert.value_version_opt();
                let value = HotStateValue::clone_from_slot(&slot_to_insert);
                let superseded_version = lru.insert(key, slot_to_insert);
                Some(HotInsertionOp {
                    state_key: (*key).clone(),
                    value,
                    value_version,
                    superseded_version,
                })
            } else {
                None
            }
        } else {
            let slot = Self::expect_old_slot(overlay, read_cache, key);
            assert!(slot.is_cold());
            let value_version = slot.value_version_opt();
            let slot = slot.to_hot(update.version);
            let value = HotStateValue::clone_from_slot(&slot);
            let superseded_version = lru.insert(key, slot);
            Some(HotInsertionOp {
                state_key: (*key).clone(),
                value,
                value_version,
                superseded_version,
            })
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
        overlay: &LayeredMap<HashValue, StateSlot>,
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
        overlay: &LayeredMap<HashValue, StateSlot>,
        cache: &StateCacheShard,
        key: &StateKey,
    ) -> StateSlot {
        if let Some(slot) = overlay.get(key.crypto_hash_ref()) {
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
        assert!(latest.is_descendant_of(&last_checkpoint));

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_store::state_view::hot_state_view::EmptyHotState;
    use aptos_types::{
        state_store::{state_slot::StateSlotKind, state_value::StateValue},
        write_set::{BaseStateOp, WriteOp},
    };

    const TEST_CONFIG: HotStateConfig = HotStateConfig {
        max_items_per_shard: 100,
        refresh_interval_versions: 100,
        delete_on_restart: false,
        compute_root_hash: true,
    };

    /// Refresh interval used by the `apply_one_update` tests. Small so versions stay readable.
    const REFRESH: Version = 10;

    fn key(s: &str) -> StateKey {
        StateKey::raw(s.as_bytes())
    }

    fn val(s: &[u8]) -> StateValue {
        StateValue::new_legacy(s.to_vec().into())
    }

    fn khash(k: &StateKey) -> HashValue {
        *k.crypto_hash_ref()
    }

    fn upd<'a>(version: Version, state_op: &'a BaseStateOp) -> StateUpdateRef<'a> {
        StateUpdateRef { version, state_op }
    }

    /// Builds the (base, top) `MapLayer` pair for an overlay holding `entries`. The caller turns it
    /// into a `LayeredMap` with `top.view_layers_after(&base)`; splitting it this way keeps the
    /// layers owned by the test so the borrow checker is happy with the `LayeredMap` referencing
    /// them.
    fn layers(
        entries: &[(HashValue, StateSlot)],
    ) -> (
        MapLayer<HashValue, StateSlot>,
        MapLayer<HashValue, StateSlot>,
    ) {
        let base: MapLayer<HashValue, StateSlot> = MapLayer::new_family("test");
        let top = if entries.is_empty() {
            base.clone()
        } else {
            base.view_layers_after(&base).new_layer(entries)
        };
        (base, top)
    }

    fn empty_lru<'a>(overlay: &'a LayeredMap<HashValue, StateSlot>) -> HotStateLRU<'a> {
        HotStateLRU::new(
            NonZeroUsize::new(100).unwrap(),
            Arc::new(EmptyHotState),
            overlay,
            None,
            None,
            0,
            0,
        )
    }

    /// Makes `k` hot by applying a write at `version`. Returns nothing; just primes the LRU so a
    /// later `MakeHot` can exercise the refresh path.
    fn seed_hot(
        lru: &mut HotStateLRU<'_>,
        overlay: &LayeredMap<HashValue, StateSlot>,
        cache: &StateCacheShard,
        k: &StateKey,
        version: Version,
    ) {
        let op = BaseStateOp::Modification(val(b"v"));
        State::apply_one_update(lru, overlay, cache, k, &upd(version, &op), REFRESH);
    }

    /// Builds an empty `State` that is a child of `parent` at `version`, sharing the layer family
    /// rooted at `root` so delta/descendant queries remain valid.
    fn empty_descendant(root: &State, parent: &State, version: Version) -> State {
        let shards = std::array::from_fn(|i| {
            parent.shards()[i]
                .view_layers_after(&root.shards()[i])
                .new_layer(&[])
        });
        State::new_with_updates(
            Some(version),
            Arc::new(shards),
            Default::default(),
            StateStorageUsage::zero(),
            TEST_CONFIG,
        )
    }

    // ===== apply_one_update: writes =====

    #[test]
    fn test_write_creation_inserts_hot_occupied() {
        let (base, top) = layers(&[]);
        let overlay = top.view_layers_after(&base);
        let cache = StateCacheShard::new();
        let mut lru = empty_lru(&overlay);

        let k = key("a");
        let op = BaseStateOp::Creation(val(b"v0"));
        let result =
            State::apply_one_update(&mut lru, &overlay, &cache, &k, &upd(7, &op), REFRESH).unwrap();

        assert_eq!(result.state_key, k);
        assert_eq!(result.value.value_opt(), Some(&val(b"v0")));
        assert_eq!(result.value.hot_since_version(), 7);
        assert_eq!(result.value_version, Some(7));
        assert_eq!(result.superseded_version, None);

        let slot = lru.get_slot(&khash(&k)).unwrap();
        assert!(slot.is_hot() && slot.is_occupied());
        assert_eq!(slot.expect_hot_since_version(), 7);
        assert_eq!(slot.expect_value_version(), 7);
    }

    #[test]
    fn test_write_deletion_inserts_hot_vacant() {
        let (base, top) = layers(&[]);
        let overlay = top.view_layers_after(&base);
        let cache = StateCacheShard::new();
        let mut lru = empty_lru(&overlay);

        let k = key("a");
        let op = WriteOp::legacy_deletion().into_base_op();
        let result =
            State::apply_one_update(&mut lru, &overlay, &cache, &k, &upd(3, &op), REFRESH).unwrap();

        assert_eq!(result.value.value_opt(), None);
        assert_eq!(result.value.hot_since_version(), 3);
        assert_eq!(result.value_version, None);
        assert_eq!(result.superseded_version, None);

        let slot = lru.get_slot(&khash(&k)).unwrap();
        assert!(slot.is_hot() && !slot.is_occupied());
    }

    #[test]
    fn test_write_supersedes_existing_hot() {
        let (base, top) = layers(&[]);
        let overlay = top.view_layers_after(&base);
        let cache = StateCacheShard::new();
        let mut lru = empty_lru(&overlay);

        let k = key("a");
        let op0 = BaseStateOp::Modification(val(b"v0"));
        State::apply_one_update(&mut lru, &overlay, &cache, &k, &upd(1, &op0), REFRESH);
        let op1 = BaseStateOp::Modification(val(b"v1"));
        let result =
            State::apply_one_update(&mut lru, &overlay, &cache, &k, &upd(5, &op1), REFRESH)
                .unwrap();

        // The second write supersedes the first, reporting the first's hot_since_version.
        assert_eq!(result.superseded_version, Some(1));
        assert_eq!(result.value.value_opt(), Some(&val(b"v1")));
        assert_eq!(result.value.hot_since_version(), 5);

        let slot = lru.get_slot(&khash(&k)).unwrap();
        assert_eq!(slot.expect_hot_since_version(), 5);
        // A write advances value_version too; a refresh (tested below) would not.
        assert_eq!(slot.expect_value_version(), 5);
    }

    // ===== apply_one_update: MakeHot promotion =====

    #[test]
    fn test_make_hot_promotes_cold_occupied_from_cache() {
        let (base, top) = layers(&[]);
        let overlay = top.view_layers_after(&base);
        let cache = StateCacheShard::new();
        let mut lru = empty_lru(&overlay);

        // The key was read cold (occupied) during execution and recorded in the cache.
        let k = key("a");
        cache.insert(
            k.clone(),
            StateSlot::new(k.clone(), StateSlotKind::ColdOccupied {
                value_version: 2,
                value: val(b"v"),
            }),
        );

        let op = BaseStateOp::MakeHot;
        let result =
            State::apply_one_update(&mut lru, &overlay, &cache, &k, &upd(9, &op), REFRESH).unwrap();

        assert_eq!(result.superseded_version, None);
        // The value and its version carry over from the cold slot; only hot_since is new.
        assert_eq!(result.value_version, Some(2));
        assert_eq!(result.value.value_opt(), Some(&val(b"v")));
        assert_eq!(result.value.hot_since_version(), 9);

        let slot = lru.get_slot(&khash(&k)).unwrap();
        assert!(slot.is_hot() && slot.is_occupied());
        assert_eq!(slot.expect_hot_since_version(), 9);
        assert_eq!(slot.expect_value_version(), 2);
    }

    #[test]
    fn test_make_hot_promotes_cold_vacant_from_cache() {
        let (base, top) = layers(&[]);
        let overlay = top.view_layers_after(&base);
        let cache = StateCacheShard::new();
        let mut lru = empty_lru(&overlay);

        let k = key("a");
        cache.insert(
            k.clone(),
            StateSlot::new(k.clone(), StateSlotKind::ColdVacant),
        );

        let op = BaseStateOp::MakeHot;
        let result =
            State::apply_one_update(&mut lru, &overlay, &cache, &k, &upd(4, &op), REFRESH).unwrap();

        assert_eq!(result.superseded_version, None);
        assert_eq!(result.value_version, None);
        assert_eq!(result.value.value_opt(), None);
        assert_eq!(result.value.hot_since_version(), 4);

        let slot = lru.get_slot(&khash(&k)).unwrap();
        assert!(slot.is_hot() && !slot.is_occupied());
    }

    #[test]
    fn test_make_hot_promotes_cold_slot_in_overlay() {
        // A key evicted to cold earlier in the speculative chain sits in the overlay, not the
        // cache. MakeHot must find it there and re-promote it.
        let k = key("a");
        let kh = khash(&k);
        let cold = StateSlot::new(k.clone(), StateSlotKind::ColdOccupied {
            value_version: 1,
            value: val(b"v"),
        });
        let (base, top) = layers(&[(kh, cold)]);
        let overlay = top.view_layers_after(&base);
        let cache = StateCacheShard::new();
        let mut lru = empty_lru(&overlay);

        let op = BaseStateOp::MakeHot;
        let result =
            State::apply_one_update(&mut lru, &overlay, &cache, &k, &upd(6, &op), REFRESH).unwrap();

        assert_eq!(result.superseded_version, None);
        assert_eq!(result.value_version, Some(1));
        assert_eq!(result.value.hot_since_version(), 6);

        let slot = lru.get_slot(&kh).unwrap();
        assert!(slot.is_hot());
        assert_eq!(slot.expect_hot_since_version(), 6);
    }

    // ===== apply_one_update: refresh interval =====

    #[test]
    fn test_make_hot_refresh_below_interval_is_noop() {
        let (base, top) = layers(&[]);
        let overlay = top.view_layers_after(&base);
        let cache = StateCacheShard::new();
        let mut lru = empty_lru(&overlay);

        let k = key("a");
        seed_hot(&mut lru, &overlay, &cache, &k, 0); // hot since 0

        // 0 + 10 <= 9 is false: not enough versions elapsed, no refresh, no op emitted.
        let op = BaseStateOp::MakeHot;
        let result = State::apply_one_update(&mut lru, &overlay, &cache, &k, &upd(9, &op), REFRESH);

        assert!(result.is_none(), "below-interval make-hot must not refresh");
        let slot = lru.get_slot(&khash(&k)).unwrap();
        assert_eq!(
            slot.expect_hot_since_version(),
            0,
            "hot_since must be unchanged"
        );
    }

    #[test]
    fn test_make_hot_refresh_at_interval_boundary() {
        let (base, top) = layers(&[]);
        let overlay = top.view_layers_after(&base);
        let cache = StateCacheShard::new();
        let mut lru = empty_lru(&overlay);

        let k = key("a");
        seed_hot(&mut lru, &overlay, &cache, &k, 0); // hot since 0

        // 0 + 10 <= 10 is true: refresh fires exactly at the boundary.
        let op = BaseStateOp::MakeHot;
        let result =
            State::apply_one_update(&mut lru, &overlay, &cache, &k, &upd(10, &op), REFRESH)
                .expect("boundary make-hot must refresh");

        assert_eq!(result.superseded_version, Some(0));
        assert_eq!(result.value.hot_since_version(), 10);
        // The value is untouched by a refresh; only hot_since advances.
        assert_eq!(result.value_version, Some(0));
        assert_eq!(result.value.value_opt(), Some(&val(b"v")));

        let slot = lru.get_slot(&khash(&k)).unwrap();
        assert_eq!(slot.expect_hot_since_version(), 10);
    }

    #[test]
    fn test_make_hot_refresh_above_interval() {
        let (base, top) = layers(&[]);
        let overlay = top.view_layers_after(&base);
        let cache = StateCacheShard::new();
        let mut lru = empty_lru(&overlay);

        let k = key("a");
        seed_hot(&mut lru, &overlay, &cache, &k, 0);

        let op = BaseStateOp::MakeHot;
        let result =
            State::apply_one_update(&mut lru, &overlay, &cache, &k, &upd(11, &op), REFRESH)
                .expect("above-interval make-hot must refresh");

        assert_eq!(result.superseded_version, Some(0));
        assert_eq!(result.value.hot_since_version(), 11);
    }

    #[test]
    fn test_make_hot_zero_interval_refreshes() {
        let (base, top) = layers(&[]);
        let overlay = top.view_layers_after(&base);
        let cache = StateCacheShard::new();
        let mut lru = empty_lru(&overlay);

        let k = key("a");
        seed_hot(&mut lru, &overlay, &cache, &k, 0);

        // refresh_interval == 0: any later version refreshes (0 + 0 <= 1).
        let op = BaseStateOp::MakeHot;
        let result = State::apply_one_update(&mut lru, &overlay, &cache, &k, &upd(1, &op), 0)
            .expect("zero interval always refreshes");
        assert_eq!(result.value.hot_since_version(), 1);
    }

    // ===== usage accounting =====

    #[test]
    fn test_update_usage_sums_deltas() {
        let state = State::new_at_version(Some(0), StateStorageUsage::new(10, 1000), TEST_CONFIG);
        let mut deltas = vec![(0i64, 0i64); NUM_STATE_SHARDS];
        deltas[0] = (3, 100);
        deltas[5] = (-1, -40);
        let usage = state.update_usage(deltas);
        assert_eq!(usage.items(), 12);
        assert_eq!(usage.bytes(), 1060);
    }

    #[test]
    fn test_usage_delta_new_occupied_key() {
        let (base, top) = layers(&[]);
        let overlay = top.view_layers_after(&base);
        let cache = StateCacheShard::new();

        // A newly created key was read cold-absent, so the cache holds a ColdVacant slot.
        let k = key("a");
        cache.insert(
            k.clone(),
            StateSlot::new(k.clone(), StateSlotKind::ColdVacant),
        );
        let op = BaseStateOp::Creation(val(b"hello"));
        let mut updates = HashMap::new();
        updates.insert(&k, upd(1, &op));

        let (items, bytes) = State::usage_delta_for_shard(&cache, &overlay, &updates);
        assert_eq!(items, 1);
        assert_eq!(bytes as usize, k.size() + val(b"hello").size());
    }

    #[test]
    fn test_usage_delta_overwrite_changes_bytes_only() {
        let (base, top) = layers(&[]);
        let overlay = top.view_layers_after(&base);
        let cache = StateCacheShard::new();

        let k = key("a");
        cache.insert(
            k.clone(),
            StateSlot::new(k.clone(), StateSlotKind::ColdOccupied {
                value_version: 0,
                value: val(b"x"),
            }),
        );
        let op = BaseStateOp::Modification(val(b"yyyy"));
        let mut updates = HashMap::new();
        updates.insert(&k, upd(1, &op));

        let (items, bytes) = State::usage_delta_for_shard(&cache, &overlay, &updates);
        // Item count unchanged; the key size cancels, leaving only the value-size difference.
        assert_eq!(items, 0);
        assert_eq!(bytes, val(b"yyyy").size() as i64 - val(b"x").size() as i64);
    }

    #[test]
    fn test_usage_delta_deletion_removes_item() {
        let (base, top) = layers(&[]);
        let overlay = top.view_layers_after(&base);
        let cache = StateCacheShard::new();

        let k = key("a");
        cache.insert(
            k.clone(),
            StateSlot::new(k.clone(), StateSlotKind::ColdOccupied {
                value_version: 0,
                value: val(b"x"),
            }),
        );
        let op = WriteOp::legacy_deletion().into_base_op();
        let mut updates = HashMap::new();
        updates.insert(&k, upd(1, &op));

        let (items, bytes) = State::usage_delta_for_shard(&cache, &overlay, &updates);
        assert_eq!(items, -1);
        assert_eq!(bytes, -((k.size() + val(b"x").size()) as i64));
    }

    #[test]
    fn test_usage_delta_make_hot_ignored() {
        let (base, top) = layers(&[]);
        let overlay = top.view_layers_after(&base);
        let cache = StateCacheShard::new();

        // MakeHot carries no value change, so it never touches usage (and never reads the cache).
        let k = key("a");
        let op = BaseStateOp::MakeHot;
        let mut updates = HashMap::new();
        updates.insert(&k, upd(1, &op));

        let (items, bytes) = State::usage_delta_for_shard(&cache, &overlay, &updates);
        assert_eq!((items, bytes), (0, 0));
    }

    // ===== State / LedgerState accessors and invariants =====

    #[test]
    fn test_version_accessors() {
        let empty = State::new_empty(TEST_CONFIG);
        assert_eq!(empty.next_version(), 0);
        assert_eq!(empty.version(), None);

        let at5 = State::new_at_version(Some(5), StateStorageUsage::new(1, 2), TEST_CONFIG);
        assert_eq!(at5.next_version(), 6);
        assert_eq!(at5.version(), Some(5));
        assert_eq!(at5.usage().items(), 1);
        assert_eq!(at5.usage().bytes(), 2);
    }

    #[test]
    fn test_is_the_same() {
        let a = State::new_empty(TEST_CONFIG);
        assert!(a.is_the_same(&a.clone()));
        let b = State::new_empty(TEST_CONFIG);
        assert!(!a.is_the_same(&b));
    }

    #[test]
    fn test_descendant_and_delta_base() {
        let s0 = State::new_empty(TEST_CONFIG);
        let s1 = empty_descendant(&s0, &s0, 0);

        assert!(s1.is_descendant_of(&s0));
        assert!(s0.can_be_delta_base_of(&s1));
        assert!(s0.can_be_delta_base_of(&s0));

        // A state from an independent layer family cannot serve as a delta base.
        let other = State::new_empty(TEST_CONFIG);
        assert!(!other.can_be_delta_base_of(&s1));
    }

    #[test]
    fn test_hot_state_metadata_accessors() {
        let mut md: [HotStateMetadata; NUM_STATE_SHARDS] = Default::default();
        let head = HashValue::new([1u8; HashValue::LENGTH]);
        let tail = HashValue::new([2u8; HashValue::LENGTH]);
        md[3] = HotStateMetadata::new(Some(head), Some(tail), 5, 999);

        let state = State::new_at_version_with_hot_state_metadata(
            Some(0),
            StateStorageUsage::zero(),
            TEST_CONFIG,
            md,
        );

        assert_eq!(state.latest_hot_key(3), Some(head));
        assert_eq!(state.oldest_hot_key(3), Some(tail));
        assert_eq!(state.num_hot_items(3), 5);
        assert_eq!(state.hot_value_bytes(3), 999);
        // Untouched shards stay at their defaults.
        assert_eq!(state.latest_hot_key(0), None);
        assert_eq!(state.num_hot_items(0), 0);
    }

    #[test]
    fn test_ledger_state_empty_is_checkpoint() {
        let ls = LedgerState::new_empty(TEST_CONFIG);
        assert!(ls.is_checkpoint());
        assert_eq!(ls.latest().next_version(), 0);
        assert_eq!(ls.last_checkpoint().next_version(), 0);
    }

    #[test]
    fn test_ledger_state_non_checkpoint() {
        let s0 = State::new_empty(TEST_CONFIG);
        let s1 = empty_descendant(&s0, &s0, 0);
        let ls = LedgerState::new(s1.clone(), s0.clone());

        assert!(!ls.is_checkpoint());
        assert!(ls.latest().is_the_same(&s1));
        assert!(ls.last_checkpoint().is_the_same(&s0));
    }
}
