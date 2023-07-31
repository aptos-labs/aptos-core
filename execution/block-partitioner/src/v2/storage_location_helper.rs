// Copyright © Aptos Foundation

use crate::v2::TxnFatId;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::collections::btree_set::BTreeSet;
use std::fmt::{Display, Formatter};
use itertools::Itertools;
use aptos_types::state_store::state_key::StateKey;
use aptos_types::transaction::analyzed_transaction::StorageLocation;

/// This structure is only used in `V2Partitioner`.
/// It holds IDs of txns that claimed to access the same storage location and
/// answers some cross-shard dependency queries efficiently.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct StorageLocationHelper {
    pub storage_location: StorageLocation,
    pub anchor_shard_id: usize,
    reads: BTreeSet<usize>,//Content are old txn ids.
    writes: BTreeSet<usize>,//Content are old txn ids.
    pub writer_set: HashSet<usize>,//Content are old txn ids.
    pub promoted_txn_ids: BTreeSet<TxnFatId>,
    pub promoted_writer_ids: BTreeSet<TxnFatId>,
}

impl StorageLocationHelper {
    pub fn new(storage_location: StorageLocation, anchor_shard_id: usize) -> Self {
        Self {
            storage_location,
            anchor_shard_id,
            reads: Default::default(),
            writes: Default::default(),
            writer_set: Default::default(),
            promoted_txn_ids: Default::default(),
            promoted_writer_ids: Default::default(),
        }
    }

    pub fn add_candidate(&mut self, txn_id: usize, is_write: bool) {
        if is_write {
            self.writes.insert(txn_id);
            self.writer_set.insert(txn_id);
        } else {
            self.reads.insert(txn_id);
        }
    }

    /// Remove a txn from candidate pool and add it to the promoted list.
    pub fn promote_txn_id(&mut self, txn_id: usize, round_id: usize, shard_id: usize) {
        let txn_fat_id = TxnFatId {
            round_id,
            shard_id,
            old_txn_idx: txn_id,
        };
        if self.writes.remove(&txn_id) {
            self.promoted_writer_ids.insert(txn_fat_id);
        } else {
            assert!(self.reads.remove(&txn_id));
        }
        self.promoted_txn_ids.insert(txn_fat_id);

    }

    /// Examples.
    /// candidates=[T4(W), T7(W), T8(R), T9(W), T11(W)], start=3, end=4 => false
    /// candidates=[T4(W), T7(W), T8(R), T9(W), T11(W)], start=3, end=5 => true
    /// candidates=[T4(W), T7(W), T8(R), T9(W), T11(W)], start=3, end=6 => true
    /// candidates=[T4(W), T7(W), T8(R), T9(W), T11(W)], start=4, end=4 => false
    /// candidates=[T4(W), T7(W), T8(R), T9(W), T11(W)], start=4, end=6 => true
    /// candidates=[T4(W), T7(W), T8(R), T9(W), T11(W)], start=5, end=6 => false
    /// candidates=[T4(W), T7(W), T8(R), T9(W), T11(W)], start=7, end=9 => true
    /// candidates=[T4(W), T7(W), T8(R), T9(W), T11(W)], start=8, end=9 => false
    /// candidates=[T4(W), T7(W), T8(R), T9(W), T11(W)], start=12, end=4 => false
    /// candidates=[T4(W), T7(W), T8(R), T9(W), T11(W)], start=12, end=5 => true
    pub fn has_write_in_range(&self, start_txn_id: usize, end_txn_id: usize) -> bool {
        if start_txn_id <= end_txn_id {
            self.writes.range(start_txn_id..end_txn_id).next().is_some()
        } else {
            self.writes.range(start_txn_id..).next().is_some() || self.writes.range(..end_txn_id).next().is_some()
        }
    }

    #[cfg(test)]
    pub fn is_writer(&self, old_txn_id: usize) -> bool {
        self.writer_set.contains(&old_txn_id)
    }

    #[cfg(test)]
    pub fn brief(&self) -> String {
        let candidates: BTreeSet<(usize, bool)> = BTreeSet::from_iter(self.reads.iter().map(|t|(*t,false)).chain(self.writes.iter().map(|t|(*t, true))));
        let candidate_strs: Vec<String> = candidates.into_iter().map(|(txn_id, is_write)|{
            let flag = if is_write {"W"} else {"R"};
            format!("{txn_id}({flag})")
        }).collect();
        let candidates_str = candidate_strs.join(",");
        let promoteds: Vec<(TxnFatId, bool)> = self.promoted_txn_ids.iter().map(|fat_id|(*fat_id, self.writer_set.contains(&fat_id.old_txn_idx))).collect();
        let promoted_strs: Vec<String> = promoteds.into_iter().map(|(fat_id, is_write)|{
            let flag = if is_write {"W"} else {"R"};
            format!("({},{})/{}({})", fat_id.round_id, fat_id.shard_id, fat_id.old_txn_idx, flag)
        }).collect();
        let promoteds_str = promoted_strs.join(",");
        format!("{{anchor={}, candidates=[{}], promoted=[{}]}}", self.anchor_shard_id, candidates_str, promoteds_str)
    }
}

#[test]
fn test_storage_location_helper() {
    let mut helper = StorageLocationHelper::new(StorageLocation::Specific(StateKey::raw(vec![])), 0);
    helper.add_candidate(4, true);
    helper.add_candidate(10, true);
    helper.add_candidate(7, true);
    helper.add_candidate(8, false);
    helper.add_candidate(9, true);
    // candidates: T4(W), T7(W), T8(R), T9(W), T10(W)
    // promoted: -
    assert!(!helper.has_write_in_range(4, 4)); // 0-length interval
    assert!(helper.has_write_in_range(4, 5)); // 0-length interval
    assert!(helper.has_write_in_range(5, 10));
    assert!(!helper.has_write_in_range(8, 9));
    assert!(helper.has_write_in_range(11, 5)); // wrapped range
    assert!(!helper.has_write_in_range(11, 4)); // wrapped range
    helper.promote_txn_id(9, 99, 10);
    // candidates: T4(W), T7(W), T8(R), T10(W)
    // promoted: (99,10)/T9(W)
    assert!(helper.has_write_in_range(5, 10));
    helper.promote_txn_id(7, 99, 20);
    // candidates: T4(W), T8(R), T10(W)
    // promoted: (99,10)/T9(W), (99,20)/T7(W)
    assert!(!helper.has_write_in_range(5, 10));
    helper.promote_txn_id(4, 99, 20);
    helper.promote_txn_id(8, 99, 30);
    helper.promote_txn_id(10, 99, 30);
    // candidates: -
    // promoted: (99,10)/T9(W), (99,20)/T4(W), (99,20)/T7(W), (99,30)/T8(R), (99,30)/T10(W)
    assert_eq!(
        vec![TxnFatId::new(99,10,9), TxnFatId::new(99,20,4), TxnFatId::new(99,20,7)],
        helper.promoted_txn_ids.range(TxnFatId::new(98,0,0)..TxnFatId::new(99,20,8)).map(|fat_id|*fat_id).collect::<Vec<_>>()
    );
    assert_eq!(
        vec![TxnFatId::new(99,20,7), TxnFatId::new(99,30,8), TxnFatId::new(99,30,10)],
        helper.promoted_txn_ids.range(TxnFatId::new(99,20,7)..).map(|fat_id|*fat_id).collect::<Vec<_>>()
    );
    assert_eq!(
        vec![TxnFatId::new(99,20,7), TxnFatId::new(99,30,10)],
        helper.promoted_writer_ids.range(TxnFatId::new(99,20,7)..TxnFatId::new(99,40,0)).map(|fat_id|*fat_id).collect::<Vec<_>>()
    );
}
