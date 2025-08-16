// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::counters::HOT_STATE_OP_ACCUMULATOR_COUNTER as COUNTER;
use aptos_logger::error;
use aptos_metrics_core::IntCounterHelper;
use aptos_types::{
    state_store::{state_slot::StateSlot, TStateView},
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
    /// Keep track of all the keys that are written to across the whole block, these keys are made
    /// hot (or have a refreshed `hot_since_version`) immediately at the version they got changed,
    /// so no need to issue separate HotStateOps to promote them to the hot state.
    writes: hashbrown::HashSet<Key>,
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
            to_make_hot: BTreeMap::new(),
            writes: hashbrown::HashSet::new(),
            max_promotions_per_block,
            _refresh_interval_versions: refresh_interval_versions,
        }
    }

    pub fn add_transaction(
        &mut self,
        writes: impl IntoIterator<Item = Key>,
        reads: impl IntoIterator<Item = Key>,
    ) {
        for key in writes {
            if self.to_make_hot.remove(&key).is_some() {
                COUNTER.inc_with(&["promotion_removed_by_write"]);
            }
            self.writes.get_or_insert_owned(&key);
        }

        for key in reads {
            if self.to_make_hot.len() >= self.max_promotions_per_block {
                COUNTER.inc_with(&["max_promotions_per_block_hit"]);
                continue;
            }
            if self.to_make_hot.contains_key(&key) {
                continue;
            }
            if self.writes.contains(&key) {
                continue;
            }
            let slot = self
                .base_view
                .get_state_slot(&key)
                .expect("base_view.get_slot() failed.");
            let make_hot = match slot {
                StateSlot::ColdVacant => {
                    COUNTER.inc_with(&["vacant_new"]);
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
                self.to_make_hot.insert(key, slot);
            }
        }
    }

    pub fn get_slots_to_make_hot(&self) -> BTreeMap<Key, StateSlot> {
        self.to_make_hot.clone()
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
