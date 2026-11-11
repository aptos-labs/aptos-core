// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::counters::HOT_STATE_OP_ACCUMULATOR_COUNTER as COUNTER;
use aptos_metrics_core::IntCounterVecHelper;
use aptos_types::{error::PanicError, on_chain_config::OnChainHotStateConfig};
use std::{collections::BTreeSet, fmt::Debug, hash::Hash};

pub struct BlockHotStateOpAccumulator<Key> {
    /// Keys read but never written to across the entire block are to be made hot (or refreshed
    /// `hot_since_version` one is already hot but last refresh is far in the history) as the side
    /// effect of the block epilogue.
    ///
    /// Unordered: the in-memory `Key` has no deterministic order, and its `Hash`/`Eq` are cheaper
    /// than materializing the storage key on every read. The deterministic order the epilogue
    /// needs is imposed once, when keys are converted to storage keys in `get_keys_to_make_hot`.
    to_make_hot: hashbrown::HashSet<Key>,
    /// Keep track of all the keys that are written to across the whole block, these keys are made
    /// hot (or have a refreshed `hot_since_version`) immediately at the version they got changed,
    /// so no need to issue separate HotStateOps to promote them to the hot state.
    writes: hashbrown::HashSet<Key>,
    /// Caps the total size of the promoted keys, to prevent the block epilogue from being too
    /// heavy.
    max_promotion_bytes_per_block: usize,
}

impl<Key> BlockHotStateOpAccumulator<Key>
where
    Key: Send + Sync + Clone + Hash + Eq + Debug,
{
    /// Used when the on-chain config doesn't exist.
    const DEFAULT_MAX_PROMOTION_BYTES_PER_BLOCK: usize = 512 * 1024;

    pub fn new(on_chain_config: Option<&OnChainHotStateConfig>) -> Self {
        let max_promotion_bytes_per_block = match on_chain_config {
            Some(OnChainHotStateConfig::V1(config)) => {
                config.max_promotion_bytes_per_block as usize
            },
            None => Self::DEFAULT_MAX_PROMOTION_BYTES_PER_BLOCK,
        };
        Self::new_with_config(max_promotion_bytes_per_block)
    }

    pub fn new_with_config(max_promotion_bytes_per_block: usize) -> Self {
        Self {
            to_make_hot: hashbrown::HashSet::new(),
            writes: hashbrown::HashSet::new(),
            max_promotion_bytes_per_block,
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
            self.writes.get_or_insert_with(key, Key::clone);
        }

        for key in reads {
            if self.writes.contains(key) {
                continue;
            }
            // Most reads repeat keys seen in earlier transactions (e.g. framework modules),
            // so clone only when the key is new to the set.
            self.to_make_hot.get_or_insert_with(key, Key::clone);
        }
    }

    /// Converts each eligible in-memory key to its storage key and returns the promotion set
    /// ordered by storage key. Ordering on the storage key (rather than the non-deterministic
    /// in-memory key) makes the cap select the same subset on every node.
    pub fn get_keys_to_make_hot<O, F, S>(
        &self,
        convert: F,
        key_size: S,
    ) -> Result<BTreeSet<O>, PanicError>
    where
        O: Ord,
        F: Fn(&Key) -> Result<O, PanicError>,
        S: Fn(&O) -> usize,
    {
        let mut keys = self
            .to_make_hot
            .iter()
            .map(convert)
            .collect::<Result<BTreeSet<O>, _>>()?;
        let num_eligible = keys.len();
        let mut remaining_bytes = self.max_promotion_bytes_per_block;
        // Visits keys in ascending order and skips any that no longer fits, so one large key
        // doesn't crowd out the smaller ones after it.
        keys.retain(|key| match remaining_bytes.checked_sub(key_size(key)) {
            Some(remaining) => {
                remaining_bytes = remaining;
                true
            },
            None => false,
        });
        let num_dropped = num_eligible - keys.len();
        if num_dropped > 0 {
            COUNTER.inc_with_by(&["promotions_dropped_over_cap"], num_dropped as u64);
        }
        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::on_chain_config::HotStateConfigV1;

    fn read(accu: &mut BlockHotStateOpAccumulator<u64>, keys: &[u64]) {
        accu.add_transaction(std::iter::empty(), keys.iter());
    }

    fn write(accu: &mut BlockHotStateOpAccumulator<u64>, keys: &[u64]) {
        accu.add_transaction(keys.iter(), std::iter::empty());
    }

    fn set(keys: &[u64]) -> BTreeSet<u64> {
        keys.iter().copied().collect()
    }

    /// Promotes with every key counting as 1 byte.
    fn promote(accu: &BlockHotStateOpAccumulator<u64>) -> BTreeSet<u64> {
        accu.get_keys_to_make_hot(|k| Ok(*k), |_| 1).unwrap()
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
        assert_eq!(promote(&forward), expected);
        assert_eq!(promote(&reverse), expected);
    }

    #[test]
    fn cap_skips_keys_that_do_not_fit() {
        let mut accu = BlockHotStateOpAccumulator::<u64>::new_with_config(3);
        read(&mut accu, &[0, 1, 2, 3, 4]);
        // 1 alone exceeds the cap, but the keys after it still fill the remaining bytes.
        let size = |k: &u64| if *k == 1 { 100 } else { 1 };
        assert_eq!(
            accu.get_keys_to_make_hot(|k| Ok(*k), size).unwrap(),
            set(&[0, 2, 3])
        );
    }

    #[test]
    fn written_keys_are_not_promoted() {
        let mut accu = BlockHotStateOpAccumulator::<u64>::new_with_config(100);
        // 2 is read and written in the same txn; the write makes it hot, so it must not also be
        // promoted by the epilogue.
        accu.add_transaction([2u64].iter(), [1u64, 2, 3].iter());
        // A read of an already-written key in a later txn is likewise ignored.
        read(&mut accu, &[2]);
        assert_eq!(promote(&accu), set(&[1, 3]));
    }

    #[test]
    fn write_after_read_removes_promotion() {
        let mut accu = BlockHotStateOpAccumulator::<u64>::new_with_config(100);
        read(&mut accu, &[1, 2]);
        write(&mut accu, &[1]);
        assert_eq!(promote(&accu), set(&[2]));
    }

    #[test]
    fn cap_from_on_chain_config() {
        let on_chain = OnChainHotStateConfig::V1(HotStateConfigV1 {
            max_items_per_shard: vec![],
            max_bytes_per_shard: vec![],
            refresh_interval_versions: 0,
            max_promotion_bytes_per_block: 2,
            defer_cold_writes_to_eviction: false,
        });
        let default_cap = BlockHotStateOpAccumulator::<u64>::DEFAULT_MAX_PROMOTION_BYTES_PER_BLOCK;
        let keys: Vec<_> = (0..default_cap as u64 + 1).collect();
        for (config, expected_cap) in [(Some(&on_chain), 2), (None, default_cap)] {
            let mut accu = BlockHotStateOpAccumulator::<u64>::new(config);
            read(&mut accu, &keys);
            assert_eq!(promote(&accu).len(), expected_cap);
        }
    }
}
