// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::counters::HOT_STATE_OP_ACCUMULATOR_COUNTER as COUNTER;
use aptos_logger::{error, info};
use aptos_metrics_core::IntCounterHelper;
use aptos_types::{
    state_store::{state_slot::StateSlot, TStateView, NUM_STATE_SHARDS},
    transaction::Version,
};
use std::{collections::BTreeMap, fmt::Debug, hash::Hash};

pub struct BlockHotStateOpAccumulator<'base_view, Key, BaseView> {
    first_version: Version,
    base_view: &'base_view BaseView,
    /// Keys read but never written to across the entire block are to be made hot (or refreshed
    /// `hot_since_version` one is already hot but last refresh is far in the history) as the side
    /// effect of the block epilogue (subject to per block limit)
    to_make_hot: BTreeMap<Key, StateSlot>,
    /// Keys to evict.
    /// NOTE: oldest keys (the ones that are evicted first) are in the front.
    ///
    /// FIXME: this is not populated to the block epilogue yet, so no eviction is happening at the
    /// moment.
    to_evict: [Vec<Key>; NUM_STATE_SHARDS],
    num_free_slots: [usize; NUM_STATE_SHARDS],
    /// Keep track of all the keys that are written to across the whole block, these keys are made
    /// hot (or have a refreshed `hot_since_version`) immediately at the version they got changed,
    /// so no need to issue separate HotStateOps to promote them to the hot state.
    writes: hashbrown::HashSet<Key>,
    /// To prevent the block epilogue from being too heavy.
    max_promotions_per_block: usize,
    /// Every now and then refresh `hot_since_version` for hot items to prevent them from being
    /// evicted.
    refresh_interval_versions: usize,
}

impl<'base_view, Key, BaseView> BlockHotStateOpAccumulator<'base_view, Key, BaseView>
where
    Key: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + Debug,
    BaseView: TStateView<Key = Key>,
{
    /// TODO(HotState): make on-chain config
    const MAX_PROMOTIONS_PER_BLOCK: usize = 1024 * 10;
    /// TODO(HotState): make on-chain config
    const REFRESH_INTERVAL_VERSIONS: usize = 1_000_000;

    pub fn new(base_view: &'base_view BaseView) -> Self {
        Self::new_with_config(
            base_view,
            Self::MAX_PROMOTIONS_PER_BLOCK,
            Self::REFRESH_INTERVAL_VERSIONS,
        )
    }

    pub fn new_with_config(
        base_view: &'base_view BaseView,
        max_promotions_per_block: usize,
        refresh_interval_versions: usize,
    ) -> Self {
        Self {
            first_version: base_view.next_version(),
            base_view,
            to_make_hot: BTreeMap::new(),
            to_evict: [(); NUM_STATE_SHARDS].map(|_| Vec::new()),
            num_free_slots: base_view.num_free_hot_slots(),
            writes: hashbrown::HashSet::new(),
            max_promotions_per_block,
            refresh_interval_versions,
        }
    }

    pub fn add_transaction<'a>(
        &mut self,
        writes: impl Iterator<Item = &'a Key>,
        reads: impl Iterator<Item = &'a Key>,
    ) where
        Key: 'a,
    {
        println!("BlockHotStateOpAccumulator::add_transaction start.");
        for key in writes {
            println!("write key: {:?}", key);
            if !self.writes.contains(key) && !self.base_view.contains_hot_state_value(key) {
                self.maybe_evict(key);
            }

            if self.to_make_hot.remove(key).is_some() {
                COUNTER.inc_with(&["promotion_removed_by_write"]);
            }
            self.writes.get_or_insert_owned(key);
        }

        for key in reads {
            println!("read key: {:?}", key);
            if self.to_make_hot.len() >= self.max_promotions_per_block {
                COUNTER.inc_with(&["max_promotions_per_block_hit"]);
                continue;
            }
            if self.to_make_hot.contains_key(key) {
                continue;
            }
            if self.writes.contains(key) {
                continue;
            }
            let slot = self
                .base_view
                .get_state_slot(key)
                .expect("base_view.get_slot() failed.");
            let make_hot = match slot {
                StateSlot::ColdVacant => {
                    COUNTER.inc_with(&["vacant_new"]);
                    self.maybe_evict(key);
                    true
                },
                StateSlot::HotVacant {
                    hot_since_version, ..
                } => {
                    if self.should_refresh(hot_since_version) {
                        COUNTER.inc_with(&["vacant_refresh"]);
                        true
                    } else {
                        COUNTER.inc_with(&["vacant_still_hot"]);
                        false
                    }
                },
                StateSlot::ColdOccupied { .. } => {
                    COUNTER.inc_with(&["occupied_new"]);
                    self.maybe_evict(key);
                    true
                },
                StateSlot::HotOccupied {
                    hot_since_version, ..
                } => {
                    if self.should_refresh(hot_since_version) {
                        COUNTER.inc_with(&["occupied_refresh"]);
                        true
                    } else {
                        COUNTER.inc_with(&["occupied_still_hot"]);
                        false
                    }
                },
            };
            if make_hot {
                self.to_make_hot.insert(key.clone(), slot);
            }
        }
        println!("BlockHotStateOpAccumulator::add_transaction end.");
        info!("Evicted keys: {:?}", self.to_evict);
    }

    fn maybe_evict(&mut self, key_added: &Key) {
        let shard_id = self.base_view.get_shard_id(key_added);
        info!(
            "shard {}: num_free_slots: {}",
            shard_id, self.num_free_slots[shard_id]
        );
        if self.num_free_slots[shard_id] > 0 {
            self.num_free_slots[shard_id] -= 1;
            return;
        }

        // FIXME: let's say it's empty at the beginning, and the first block is really large. Then
        // get_next_old_key would always return None, so nothing will be evicted. And the LRU would
        // end up being larger than capacity.
        // Next time when computing num_free_slots, it might overflow.
        //
        //
        // The above is okay. However.
        //
        // Additional FIXME: not sure if the eviction logic is correct here. We evict the next old
        // key. However, we do not take into consideration that this key might have been promoted
        // recently. So we need to look further!!!

        let last_evicted = self.to_evict[shard_id].last();
        if let Some(k) = self
            .base_view
            .get_next_old_key(shard_id, last_evicted)
            .unwrap()
        {
            info!("Decided to evict key {:?}", k);
            // Unless the entire LRU is evicted (in that case `last_evicted` is already the newest
            // key in the LRU and`get_next_old_key` would return `None`), evict the next key.
            self.to_evict[shard_id].push(k);
        }
    }

    pub fn get_slots_to_make_hot(&self) -> BTreeMap<Key, StateSlot> {
        self.to_make_hot.clone()
    }

    pub fn get_eviction(&self) -> [Vec<Key>; NUM_STATE_SHARDS] {
        self.to_evict.clone()
    }

    pub fn should_refresh(&self, hot_since_version: Version) -> bool {
        if hot_since_version >= self.first_version {
            error!(
                "Unexpected: hot_since_version {} >= block first version {}",
                hot_since_version, self.first_version
            );
        }
        hot_since_version + self.refresh_interval_versions as Version <= self.first_version
    }
}
