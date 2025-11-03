// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::new_without_default)]

use crate::counters::HOT_STATE_OP_ACCUMULATOR_COUNTER as COUNTER;
use aptos_metrics_core::IntCounterVecHelper;
use std::{collections::BTreeSet, fmt::Debug, hash::Hash};

pub struct BlockHotStateOpAccumulator<Key> {
    /// Keys read but never written to across the entire block are to be made hot (or refreshed
    /// `hot_since_version` one is already hot but last refresh is far in the history) as the side
    /// effect of the block epilogue (subject to per block limit)
    to_make_hot: BTreeSet<Key>,
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

impl<Key> BlockHotStateOpAccumulator<Key>
where
    Key: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + Debug,
{
    /// TODO(HotState): make on-chain config
    const MAX_PROMOTIONS_PER_BLOCK: usize = 1024 * 10;
    /// TODO(HotState): make on-chain config
    const REFRESH_INTERVAL_VERSIONS: usize = 1_000_000;

    pub fn new() -> Self {
        Self::new_with_config(
            Self::MAX_PROMOTIONS_PER_BLOCK,
            Self::REFRESH_INTERVAL_VERSIONS,
        )
    }

    pub fn new_with_config(
        max_promotions_per_block: usize,
        refresh_interval_versions: usize,
    ) -> Self {
        Self {
            to_make_hot: BTreeSet::new(),
            writes: hashbrown::HashSet::new(),
            max_promotions_per_block,
            _refresh_interval_versions: refresh_interval_versions,
        }
    }

    pub fn add_transaction<'a>(
        &mut self,
        writes: impl Iterator<Item = &'a Key>,
        reads: impl Iterator<Item = &'a Key>,
    ) where
        Key: 'a,
    {
        for key in writes {
            if self.to_make_hot.remove(key) {
                COUNTER.inc_with(&["promotion_removed_by_write"]);
            }
            self.writes.get_or_insert_owned(key);
        }

        for key in reads {
            if self.to_make_hot.len() >= self.max_promotions_per_block {
                COUNTER.inc_with(&["max_promotions_per_block_hit"]);
                continue;
            }
            if self.writes.contains(key) {
                continue;
            }
            self.to_make_hot.insert(key.clone());
        }
    }

    pub fn get_keys_to_make_hot(&self) -> BTreeSet<Key> {
        self.to_make_hot.clone()
    }
}
