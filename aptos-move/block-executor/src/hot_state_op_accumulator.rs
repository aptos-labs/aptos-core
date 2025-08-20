// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::counters::HOT_STATE_OP_ACCUMULATOR_COUNTER as COUNTER;
use aptos_logger::error;
use aptos_metrics_core::IntCounterHelper;
use aptos_types::{
    state_store::{state_slot::StateSlot, TStateView},
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
    /// for hot state promotion (subject to rules such as refresh interval and per block limit).
    reads: hashbrown::HashSet<Key>,
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
            reads: hashbrown::HashSet::new(),
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
        self.writes.extend(writes);
        self.reads.extend(reads);
    }

    pub fn get_slots_to_make_hot(&self) -> BTreeMap<Key, StateSlot> {
        let read_only: BTreeSet<_> = self.reads.difference(&self.writes).collect();
        read_only
            .into_iter()
            .filter_map(|key| self.maybe_make_hot(key).map(|slot| (key.clone(), slot)))
            .take(self.max_promotions_per_block)
            .collect()
    }

    fn maybe_make_hot(&self, key: &Key) -> Option<StateSlot> {
        let slot = self
            .base_view
            .get_state_slot(key)
            .expect("base_view.get_slot() failed.");

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
