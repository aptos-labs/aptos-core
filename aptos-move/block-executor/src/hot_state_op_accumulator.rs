// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![allow(clippy::new_without_default)]

use crate::counters::HOT_STATE_OP_ACCUMULATOR_COUNTER as COUNTER;
use aptos_metrics_core::IntCounterVecHelper;
use std::{collections::BTreeSet, fmt::Debug, hash::Hash};

pub struct BlockHotStateOpAccumulator<Key> {
    /// Keys read but never written to across the entire block are candidates to be made hot (or to
    /// have their `hot_since_version` refreshed if already hot but the last refresh is far in the
    /// history) as a side effect of the block epilogue.
    ///
    /// The per-block promotion cap is intentionally NOT applied here while reads stream in. The cap
    /// is applied over the final sorted candidate set in [`Self::get_keys_to_make_hot`]. Applying it
    /// during streaming made the selected set depend on read-observation order, which is not
    /// deterministic across validators (e.g. reads were fed from hash-ordered iterators); a
    /// divergent `to_make_hot` produces divergent block-epilogue bytes and a state-root mismatch.
    to_make_hot: BTreeSet<Key>,
    /// Keep track of all the keys that are written to across the whole block, these keys are made
    /// hot (or have a refreshed `hot_since_version`) immediately at the version they got changed,
    /// so no need to issue separate HotStateOps to promote them to the hot state.
    writes: hashbrown::HashSet<Key>,
    /// To prevent the block epilogue from being too heavy.
    max_promotions_per_block: usize,
}

impl<Key> BlockHotStateOpAccumulator<Key>
where
    Key: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + Debug,
{
    /// TODO(HotState): make on-chain config. NOTE: this bounds the serialized
    /// `BlockEpiloguePayload::V2::to_make_hot` set, so it is consensus-relevant: if it ever becomes
    /// on-chain config it must be read identically by all validators.
    const MAX_PROMOTIONS_PER_BLOCK: usize = 1024 * 10;

    pub fn new() -> Self {
        Self::new_with_config(Self::MAX_PROMOTIONS_PER_BLOCK)
    }

    pub fn new_with_config(max_promotions_per_block: usize) -> Self {
        Self {
            to_make_hot: BTreeSet::new(),
            writes: hashbrown::HashSet::new(),
            max_promotions_per_block,
        }
    }

    pub fn add_transaction<'a>(
        &mut self,
        writes: impl Iterator<Item = &'a Key>,
        reads: impl Iterator<Item = &'a Key>,
    ) where
        Key: 'a,
    {
        let mut num_writes = 0;
        for key in writes {
            num_writes += 1;
            if self.to_make_hot.remove(key) {
                COUNTER.inc_with(&["promotion_removed_by_write"]);
            }
            self.writes.get_or_insert_owned(key);
        }

        let mut num_reads = 0;
        for key in reads {
            num_reads += 1;
            if self.writes.contains(key) {
                COUNTER.inc_with(&["read_skipped_written_in_block"]);
                continue;
            }
            self.to_make_hot.insert(key.clone());
        }

        COUNTER
            .with_label_values(&["writes_observed"])
            .inc_by(num_writes);
        COUNTER
            .with_label_values(&["reads_observed"])
            .inc_by(num_reads);
    }

    /// Returns the keys to be promoted to hot state in the block epilogue.
    ///
    /// The per-block cap is applied here, deterministically: candidates are held in a `BTreeSet`, so
    /// taking the first `max_promotions_per_block` yields the N smallest keys regardless of the
    /// order in which reads were observed. This is what makes the result identical across validators.
    pub fn get_keys_to_make_hot(&self) -> BTreeSet<Key> {
        COUNTER
            .with_label_values(&["candidates_considered"])
            .inc_by(self.to_make_hot.len() as u64);

        let result: BTreeSet<Key> = if self.to_make_hot.len() > self.max_promotions_per_block {
            COUNTER.inc_with(&["max_promotions_per_block_hit"]);
            self.to_make_hot
                .iter()
                .take(self.max_promotions_per_block)
                .cloned()
                .collect()
        } else {
            self.to_make_hot.clone()
        };

        COUNTER
            .with_label_values(&["promoted"])
            .inc_by(result.len() as u64);
        result
    }
}
