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
    /// Keys read but never written to across the entire block that are to be made hot (or refreshed
    /// `hot_since_version` if already hot but the last refresh is far in the history) as the side
    /// effect of the block epilogue.
    ///
    /// All eligible keys are accumulated here without applying `max_promotions_per_block`. The cap
    /// is applied deterministically at materialization time (see `get_slots_to_make_hot`) rather
    /// than during accumulation: reads are fed in from a `HashSet` whose iteration order varies
    /// across processes, so capping while accumulating would let different validators retain
    /// different subsets and diverge.
    to_make_hot: BTreeMap<Key, StateSlot>,
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
            writes: hashbrown::HashSet::new(),
            max_promotions_per_block,
            refresh_interval_versions,
        }
    }

    pub fn add_transaction<'a>(
        &mut self,
        writes: impl Iterator<Item = &'a Key>,
        read_only: impl Iterator<Item = &'a Key>,
    ) where
        Key: 'a,
    {
        for key in writes {
            if self.to_make_hot.remove(key).is_some() {
                COUNTER.inc_with(&["promotion_removed_by_write"]);
            }
            self.writes.get_or_insert_owned(key);
        }

        for key in read_only {
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
                    true
                },
                StateSlot::HotVacant { hot_since_version } => {
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
                self.to_make_hot.insert(key.clone(), slot);
            }
        }
    }

    pub fn get_slots_to_make_hot(&self) -> BTreeMap<Key, StateSlot> {
        // Apply the per-block cap here rather than while accumulating, so the promoted subset is a
        // deterministic function of the accumulated keys and not of the (non-deterministic) order
        // in which reads were observed. `to_make_hot` is ordered by key, so retain the smallest
        // `max_promotions_per_block` keys, which every process computes identically.
        if self.to_make_hot.len() > self.max_promotions_per_block {
            let num_dropped = self.to_make_hot.len() - self.max_promotions_per_block;
            COUNTER.inc_with_by(&["promotions_dropped_over_cap"], num_dropped as u64);
            self.to_make_hot
                .iter()
                .take(self.max_promotions_per_block)
                .map(|(key, slot)| (key.clone(), slot.clone()))
                .collect()
        } else {
            self.to_make_hot.clone()
        }
    }

    pub fn should_refresh(&self, hot_since_version: Version) -> bool {
        if hot_since_version >= self.first_version {
            error!("Unexpected: hot_since_version > block first version");
        }
        hot_since_version + self.refresh_interval_versions as Version >= self.first_version
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::state_store::{
        state_slot::StateSlot, state_storage_usage::StateStorageUsage, StateViewResult,
    };

    /// Every key is reported as cold & absent, so every read-only key is eligible for promotion.
    /// This lets the tests exercise the cap in isolation from the slot classification logic.
    struct MockBaseView;

    impl TStateView for MockBaseView {
        type Key = u64;

        fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
            Ok(StateStorageUsage::new_untracked())
        }

        fn next_version(&self) -> Version {
            0
        }

        fn get_state_slot(&self, _state_key: &Self::Key) -> StateViewResult<StateSlot> {
            Ok(StateSlot::ColdVacant)
        }
    }

    /// Feeding the same reads in two different orders must produce the exact same promoted subset,
    /// even when the number of eligible keys exceeds the per-block cap. This is the property that
    /// keeps validators from diverging when reads arrive in a non-deterministic (HashSet) order.
    #[test]
    fn cap_selection_is_independent_of_read_order() {
        const CAP: usize = 5;
        let base_view = MockBaseView;
        let ascending: Vec<u64> = (0..20).collect();
        let descending: Vec<u64> = (0..20).rev().collect();

        let mut accu_asc = BlockHotStateOpAccumulator::new_with_config(&base_view, CAP, 1_000_000);
        accu_asc.add_transaction(std::iter::empty(), ascending.iter());

        let mut accu_desc = BlockHotStateOpAccumulator::new_with_config(&base_view, CAP, 1_000_000);
        accu_desc.add_transaction(std::iter::empty(), descending.iter());

        let promoted_asc: Vec<u64> = accu_asc.get_slots_to_make_hot().into_keys().collect();
        let promoted_desc: Vec<u64> =
            accu_desc.get_slots_to_make_hot().into_keys().collect();

        assert_eq!(promoted_asc, promoted_desc);
        // The cap retains the smallest keys deterministically.
        assert_eq!(promoted_asc, vec![0, 1, 2, 3, 4]);
    }

    /// Below the cap, every eligible key is promoted.
    #[test]
    fn all_keys_promoted_when_under_cap() {
        let base_view = MockBaseView;
        let mut accu = BlockHotStateOpAccumulator::new_with_config(&base_view, 100, 1_000_000);
        let reads: Vec<u64> = (0..10).collect();
        accu.add_transaction(std::iter::empty(), reads.iter());

        let promoted: Vec<u64> = accu.get_slots_to_make_hot().into_keys().collect();
        assert_eq!(promoted, reads);
    }

    /// Keys written anywhere in the block are never promoted, regardless of whether the write is
    /// seen before or after the read.
    #[test]
    fn written_keys_are_excluded() {
        let base_view = MockBaseView;
        let mut accu = BlockHotStateOpAccumulator::new_with_config(&base_view, 100, 1_000_000);

        // Read 1 first, then a later transaction writes it: it must be removed from promotions.
        accu.add_transaction(std::iter::empty(), [1u64, 2, 3].iter());
        accu.add_transaction([1u64].iter(), std::iter::empty());
        // Write 5 first, then read it: it must never be added.
        accu.add_transaction([5u64].iter(), std::iter::empty());
        accu.add_transaction(std::iter::empty(), [5u64].iter());

        let promoted: Vec<u64> = accu.get_slots_to_make_hot().into_keys().collect();
        assert_eq!(promoted, vec![2, 3]);
    }
}
