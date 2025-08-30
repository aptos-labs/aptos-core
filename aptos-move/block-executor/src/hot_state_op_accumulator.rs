// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::counters::HOT_STATE_OP_ACCUMULATOR_COUNTER as COUNTER;
use aptos_logger::{error, info};
use aptos_metrics_core::IntCounterHelper;
use aptos_types::{
    state_store::{
        hot_state::HOT_STATE_MAX_ITEMS_PER_SHARD, state_slot::StateSlot, TStateView,
        NUM_STATE_SHARDS,
    },
    transaction::Version,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
    hash::Hash,
};

pub struct BlockHotStateOpAccumulator<'base_view, Key, BaseView> {
    first_version: Version,
    base_view: &'base_view BaseView,
    /// Keep track of all the keys that are read across the whole block. These keys are candidates
    /// for hot state promotion (subject to rules such as per block limit).
    reads: hashbrown::HashSet<Key>,
    /// To prevent the block epilogue from being too heavy.
    max_promotions_per_block: usize,
    /// Every now and then refresh `hot_since_version` for hot items to prevent them from being
    /// evicted.
    _refresh_interval_versions: usize,
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
            reads: hashbrown::HashSet::new(),
            max_promotions_per_block,
            _refresh_interval_versions: refresh_interval_versions,
        }
    }

    pub fn add_transaction_reads(&mut self, reads: impl IntoIterator<Item = Key>) {
        self.reads.extend(reads);
    }

    pub fn get_promotions_and_evictions(
        &self,
        writes: hashbrown::HashSet<Key>,
    ) -> (BTreeMap<Key, StateSlot>, BTreeMap<Key, StateSlot>) {
        let read_only: BTreeSet<_> = self.reads.difference(&writes).collect();
        let to_make_hot: BTreeMap<_, _> = read_only
            .iter()
            .filter_map(|key| self.maybe_make_hot(*key).map(|slot| ((*key).clone(), slot)))
            .take(self.max_promotions_per_block)
            .collect();
        COUNTER.inc_with_by(&["total_make_hot"], to_make_hot.len() as u64);

        let mut to_evict = BTreeMap::new();
        let mut num_hot_items = self.base_view.num_hot_items();
        for key in writes.iter().chain(read_only.iter().map(|k| *k)) {
            if self.base_view.contains_hot_state_value(key) {
                continue;
            }
            let shard_id = self.base_view.get_shard_id(key);
            num_hot_items[shard_id] += 1;
        }

        for shard_id in 0..NUM_STATE_SHARDS {
            // The previous key considered for eviction. Starts with `None`, which means we try to
            // evict the oldest key first.
            let mut prev = None;
            info!(
                "shard {} size before eviction: {}",
                shard_id, num_hot_items[shard_id]
            );
            while num_hot_items[shard_id] > HOT_STATE_MAX_ITEMS_PER_SHARD {
                unreachable!("no evictions for now");
                while let Some(k) = self
                    .base_view
                    .get_next_old_key(shard_id, prev.as_ref())
                    .unwrap()
                {
                    prev = Some(k.clone());
                    if !writes.contains(&k) && !read_only.contains(&k) {
                        let slot = self
                            .base_view
                            .get_state_slot(&k)
                            .expect("base_view.get_slot() should not fail for keys to evict");
                        to_evict.insert(k, slot);
                        num_hot_items[shard_id] -= 1;
                        break;
                    }
                }
            }
        }

        (to_make_hot, to_evict)
    }

    fn maybe_make_hot(&self, key: &Key) -> Option<StateSlot> {
        let slot = self
            .base_view
            .get_state_slot(key)
            .expect("base_view.get_slot() should not fail for keys to make hot");

        match &slot {
            StateSlot::ColdVacant => {
                COUNTER.inc_with(&["vacant_new"]);
                Some(slot)
            },
            StateSlot::HotVacant {
                hot_since_version, ..
            } => {
                if self.should_refresh(*hot_since_version) {
                    COUNTER.inc_with(&["vacant_refresh"]);
                    Some(slot)
                } else {
                    COUNTER.inc_with(&["vacant_still_hot"]);
                    None
                }
            },
            StateSlot::ColdOccupied { .. } => {
                COUNTER.inc_with(&["occupied_new"]);
                Some(slot)
            },
            StateSlot::HotOccupied {
                hot_since_version, ..
            } => {
                if self.should_refresh(*hot_since_version) {
                    COUNTER.inc_with(&["occupied_refresh"]);
                    Some(slot)
                } else {
                    COUNTER.inc_with(&["occupied_still_hot"]);
                    None
                }
            },
        }
    }

    pub fn should_refresh(&self, hot_since_version: Version) -> bool {
        if hot_since_version >= self.first_version {
            error!(
                "Unexpected: hot_since_version {} >= block first version {}",
                hot_since_version, self.first_version
            );
        }
        // TODO(HotState): understand perf impact. For now, we always refresh.
        // hot_since_version + self.refresh_interval_versions as Version <= self.first_version
        true
    }
}
