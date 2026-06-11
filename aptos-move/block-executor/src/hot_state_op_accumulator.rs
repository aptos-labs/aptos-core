// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![allow(clippy::new_without_default)]

use crate::counters::HOT_STATE_OP_ACCUMULATOR_COUNTER as COUNTER;
use aptos_metrics_core::IntCounterVecHelper;
use std::{collections::BTreeSet, fmt::Debug, hash::Hash};

pub struct BlockHotStateOpAccumulator<Key> {
    /// Keys read but never written to across the entire block are to be made hot (or refreshed
    /// `hot_since_version` one is already hot but last refresh is far in the history) as the side
    /// effect of the block epilogue.
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
    /// TODO(HotState): make on-chain config. Also consider capping by total key size instead of
    /// number.
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
        for key in writes {
            if self.to_make_hot.remove(key) {
                COUNTER.inc_with(&["promotion_removed_by_write"]);
            }
            self.writes.get_or_insert_owned(key);
        }

        for key in reads {
            if self.writes.contains(key) {
                continue;
            }
            self.to_make_hot.insert(key.clone());
        }
    }

    pub fn get_keys_to_make_hot(&self) -> BTreeSet<Key> {
        let num_eligible = self.to_make_hot.len();
        if num_eligible > self.max_promotions_per_block {
            COUNTER.inc_with_by(
                &["promotions_dropped_over_cap"],
                (num_eligible - self.max_promotions_per_block) as u64,
            );
        }
        self.to_make_hot
            .iter()
            .take(self.max_promotions_per_block)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(accu: &mut BlockHotStateOpAccumulator<u64>, keys: &[u64]) {
        accu.add_transaction(std::iter::empty(), keys.iter());
    }

    fn write(accu: &mut BlockHotStateOpAccumulator<u64>, keys: &[u64]) {
        accu.add_transaction(keys.iter(), std::iter::empty());
    }

    fn set(keys: &[u64]) -> BTreeSet<u64> {
        keys.iter().copied().collect()
    }

    /// When more keys are eligible than the cap allows, the surviving subset must be the same
    /// regardless of the order reads are observed in (the bug being that HashSet-ordered reads
    /// made the dropped subset process-dependent).
    #[test]
    fn cap_selects_smallest_keys_independent_of_order() {
        let mut forward = BlockHotStateOpAccumulator::<u64>::new_with_config(3);
        for k in 0..10 {
            read(&mut forward, &[k]);
        }

        let mut reverse = BlockHotStateOpAccumulator::<u64>::new_with_config(3);
        for k in (0..10).rev() {
            read(&mut reverse, &[k]);
        }

        let expected = set(&[0, 1, 2]);
        assert_eq!(forward.get_keys_to_make_hot(), expected);
        assert_eq!(reverse.get_keys_to_make_hot(), expected);
    }

    #[test]
    fn written_keys_are_not_promoted() {
        let mut accu = BlockHotStateOpAccumulator::<u64>::new_with_config(100);
        // 2 is read and written in the same txn; the write makes it hot, so it must not also be
        // promoted by the epilogue.
        accu.add_transaction([2u64].iter(), [1u64, 2, 3].iter());
        // A read of an already-written key in a later txn is likewise ignored.
        read(&mut accu, &[2]);
        assert_eq!(accu.get_keys_to_make_hot(), set(&[1, 3]));
    }

    #[test]
    fn write_after_read_removes_promotion() {
        let mut accu = BlockHotStateOpAccumulator::<u64>::new_with_config(100);
        read(&mut accu, &[1, 2]);
        write(&mut accu, &[1]);
        assert_eq!(accu.get_keys_to_make_hot(), set(&[2]));
    }
}
