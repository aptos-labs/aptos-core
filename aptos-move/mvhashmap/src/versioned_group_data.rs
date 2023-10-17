// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::{Flag, Incarnation, MVGroupError, ShiftedTxnIndex, TxnIndex, Version};
use aptos_types::write_set::TransactionWrite;
use claims::assert_some;
use crossbeam::utils::CachePadded;
use dashmap::DashMap;
use move_core_types::value::MoveTypeLayout;
use serde::Serialize;
use std::{
    collections::{btree_map::BTreeMap, HashMap},
    fmt::Debug,
    hash::Hash,
    sync::Arc,
};

struct GroupEntry<V> {
    incarnation: Incarnation,
    // Note: can be a raw pointer (different data-structure holds the value during the
    // lifetime), but would require unsafe access.
    value: Arc<V>,
    layout: Option<Arc<MoveTypeLayout>>,
    flag: Flag,
}

impl<V: TransactionWrite> GroupEntry<V> {
    fn new(incarnation: Incarnation, value: Arc<V>, layout: Option<Arc<MoveTypeLayout>>) -> Self {
        Self {
            incarnation,
            value,
            layout,
            flag: Flag::Done,
        }
    }
}

/// Represents a group value, i.e. a key that does not correspond to a single value,
/// but instead a collection of values each associated with a tag.
///
/// Implementation note: due to DashMap in VersionedGroupData, the updates are atomic.
/// If this changes, we must maintain invariants on insertion / deletion order among
/// members (e.g. versioned_map then idx_to_update, deletion vice versa).
pub(crate) struct VersionedGroupValue<T, V> {
    /// While versioned_map maps tags to versioned entries for the tag, idx_to_update
    /// maps a transaction index to all corresponding group updates. ShiftedTxnIndex is used
    /// to dedicated index 0 for base (storage version, prior to block execution) values.
    versioned_map: HashMap<T, BTreeMap<ShiftedTxnIndex, CachePadded<GroupEntry<V>>>>,
    /// Mapping transaction indices to the set of group member updates. As it is required
    /// to provide base values from storage, and since all versions including storage are
    /// represented in the same data-structure, the key set corresponds to all relevant
    /// tags (group membership is not fixed, see aip-9).
    /// Note: if we do not garbage collect final idx_to_update contents until the end of
    /// block execution (lifetime of the data-structure), then we can have other structures
    /// hold raw pointers to the values as an optimization.
    idx_to_update: BTreeMap<ShiftedTxnIndex, CachePadded<HashMap<T, Arc<V>>>>,

    /// Group contents corresponding to the latest committed version.
    committed_group: HashMap<T, Arc<V>>,
}

/// Maps each key (access path) to an internal VersionedValue.
pub struct VersionedGroupData<K, T, V> {
    group_values: DashMap<K, VersionedGroupValue<T, V>>,
}

impl<T: Hash + Clone + Debug + Eq + Serialize, V: TransactionWrite> Default
    for VersionedGroupValue<T, V>
{
    fn default() -> Self {
        Self {
            versioned_map: HashMap::new(),
            idx_to_update: BTreeMap::new(),
            committed_group: HashMap::new(),
        }
    }
}

impl<T: Hash + Clone + Debug + Eq + Serialize, V: TransactionWrite> VersionedGroupValue<T, V> {
    fn set_base_values(
        &mut self,
        shifted_idx: ShiftedTxnIndex,
        incarnation: Incarnation,
        // TODO add layout to values
        values: impl IntoIterator<Item = (T, V)>,
    ) {
        match self.idx_to_update.get(&shifted_idx) {
            Some(previous) => {
                // base value may have already been provided due to a concurrency race.
                // If maybe_layout is None, they are required to be identical.
                // If maybe_layout is Some, there might have been an exchange
                // Assert the length of bytes for efficiency (instead of full equality)
                for (tag, v) in values.into_iter() {
                    let prev_v = previous
                        .get(&tag)
                        .expect("Reading twice from storage must be consistent");
                    assert!(v.bytes_len() == prev_v.bytes_len());
                }
            },
            None => self.write(shifted_idx, incarnation, values),
        }
    }

    fn write(
        &mut self,
        shifted_idx: ShiftedTxnIndex,
        incarnation: Incarnation,
        // TODO add layout to values
        values: impl IntoIterator<Item = (T, V)>,
    ) {
        let arc_map = values
            .into_iter()
            .map(|(tag, v)| {
                let arc_v = Arc::new(v);

                // Update versioned_map.
                let tag_entry = self.versioned_map.entry(tag.clone()).or_default();
                tag_entry.insert(
                    shifted_idx.clone(),
                    // TODO layout shouldn't be none
                    CachePadded::new(GroupEntry::new(incarnation, arc_v.clone(), None)),
                );

                (tag, arc_v)
            })
            .collect();

        let zero = ShiftedTxnIndex::zero();
        let base_idx = shifted_idx == zero;

        self.idx_to_update
            .insert(shifted_idx, CachePadded::new(arc_map));
        if base_idx {
            self.commit_idx(zero);
        }
    }

    fn mark_estimate(&mut self, txn_idx: TxnIndex) {
        let shifted_idx = ShiftedTxnIndex::new(txn_idx);
        let idx_updates = self
            .idx_to_update
            .get(&shifted_idx)
            .expect("Group updates must exist at the index to mark estimate");

        // estimate flag lives in GroupEntry, w. value in versioned_map to simplify reading
        // based on txn_idx and tag. marking estimates occurs per txn (data MVHashMap exposes
        // the interface for txn_idx & key). Hence, we must mark tags individually.
        for (tag, _) in idx_updates.iter() {
            self.versioned_map
                .get_mut(tag)
                .expect("Versioned entry must exist for tag")
                .get_mut(&shifted_idx)
                .expect("Versioned entry must exist")
                .flag = Flag::Estimate;
        }
    }

    fn delete(&mut self, txn_idx: TxnIndex) {
        let shifted_idx = ShiftedTxnIndex::new(txn_idx);
        // Delete idx updates first, then entries.
        let idx_updates = self
            .idx_to_update
            .remove(&shifted_idx)
            .expect("Group updates must exist at the index to mark estimate");

        // Similar to mark_estimate, need to delete an individual entry for each tag.
        for (tag, _) in idx_updates.iter() {
            assert_some!(
                self.versioned_map
                    .get_mut(tag)
                    .expect("Versioned entry must exist for tag")
                    .remove(&shifted_idx),
                "Entry for tag / idx must exist to be deleted"
            );
        }
    }

    // Records and returns pointers for the latest committed value for each tag in the group.
    fn commit_idx(&mut self, shifted_idx: ShiftedTxnIndex) -> HashMap<T, Arc<V>> {
        let idx_updates = self
            .idx_to_update
            .get(&shifted_idx)
            .expect("Group updates must exist at the index to commit");
        for (tag, v) in idx_updates.iter() {
            if v.is_deletion() {
                self.committed_group.remove(tag);
            } else {
                self.committed_group.insert(tag.clone(), v.clone());
            }
        }

        self.committed_group.clone()
    }

    fn get_latest_tagged_value(
        &self,
        tag: &T,
        txn_idx: TxnIndex,
    ) -> Result<(Version, Arc<V>, Option<Arc<MoveTypeLayout>>), MVGroupError> {
        let common_error = || -> MVGroupError {
            if self.idx_to_update.contains_key(&ShiftedTxnIndex::zero()) {
                MVGroupError::TagNotFound
            } else {
                MVGroupError::Uninitialized
            }
        };

        self.versioned_map
            .get(tag)
            .ok_or(common_error())
            .and_then(|tree| {
                match tree
                    .range(ShiftedTxnIndex::zero()..ShiftedTxnIndex::new(txn_idx))
                    .next_back()
                {
                    Some((idx, entry)) => {
                        if entry.flag == Flag::Estimate {
                            Err(MVGroupError::Dependency(
                                idx.idx()
                                    .expect("Base version cannot be marked as estimate"),
                            ))
                        } else {
                            Ok((
                                idx.idx().map(|idx| (idx, entry.incarnation)),
                                entry.value.clone(),
                                entry.layout.clone(),
                            ))
                        }
                    },
                    None => Err(common_error()),
                }
            })
    }

    fn get_latest_group_size(&self, txn_idx: TxnIndex) -> Result<u64, MVGroupError> {
        if !self.idx_to_update.contains_key(&ShiftedTxnIndex::zero()) {
            return Err(MVGroupError::Uninitialized);
        }

        self.versioned_map
            .iter()
            .try_fold(0_u64, |len, (tag, tree)| {
                match tree
                    .range(ShiftedTxnIndex::zero()..ShiftedTxnIndex::new(txn_idx))
                    .next_back()
                {
                    Some((idx, entry)) => {
                        if entry.flag == Flag::Estimate {
                            Err(MVGroupError::Dependency(
                                idx.idx().expect("May not depend on storage version"),
                            ))
                        } else {
                            let delta = entry.value.bytes_len() as u64
                                + bcs::serialized_size(tag)
                                    .map_err(|_| MVGroupError::TagSerializationError)?
                                    as u64;
                            Ok(len + delta)
                        }
                    },
                    None => Ok(len),
                }
            })
    }
}

impl<
        K: Hash + Clone + Debug + Eq,
        T: Hash + Clone + Debug + Eq + Serialize,
        V: TransactionWrite,
    > VersionedGroupData<K, T, V>
{
    pub(crate) fn new() -> Self {
        Self {
            group_values: DashMap::new(),
        }
    }

    pub fn provide_base_values(&self, key: K, base_values: impl IntoIterator<Item = (T, V)>) {
        // Incarnation is irrelevant for storage version, set to 0.
        self.group_values.entry(key).or_default().set_base_values(
            ShiftedTxnIndex::zero(),
            0,
            base_values,
        );
    }

    pub fn write(
        &self,
        key: K,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        values: impl IntoIterator<Item = (T, V)>,
    ) {
        self.group_values.entry(key).or_default().write(
            ShiftedTxnIndex::new(txn_idx),
            incarnation,
            values,
        );
    }

    /// Mark all entry from transaction 'txn_idx' at access path 'key' as an estimated write
    /// (for future incarnation). Will panic if the entry is not in the data-structure.
    pub fn mark_estimate(&self, key: &K, txn_idx: TxnIndex) {
        self.group_values
            .get_mut(key)
            .expect("Path must exist")
            .mark_estimate(txn_idx);
    }

    /// Delete all entries from transaction 'txn_idx' at access path 'key'. Will panic
    /// if the corresponding entry does not exist.
    pub fn delete(&self, key: &K, txn_idx: TxnIndex) {
        self.group_values
            .get_mut(key)
            .expect("Path must exist")
            .delete(txn_idx);
    }

    /// Read the latest value corresponding to a tag at a given group (identified by key).
    /// Return the size of the group (if requested), as defined above, alongside the version
    /// information (None if storage/pre-block version).
    pub fn read_from_group(
        &self,
        key: &K,
        tag: &T,
        txn_idx: TxnIndex,
    ) -> anyhow::Result<(Version, Arc<V>, Option<Arc<MoveTypeLayout>>), MVGroupError> {
        match self.group_values.get(key) {
            Some(g) => g.get_latest_tagged_value(tag, txn_idx),
            None => Err(MVGroupError::Uninitialized),
        }
    }

    /// Returns the sum of latest sizes of all group members (and their respective tags),
    /// collected based on the list of recorded tags. If the latest entry at any tag was
    /// marked as an estimate, a dependency is returned. Note: it would be possible to
    /// process estimated entry sizes, but would have to mark that if after the re-execution
    /// the entry size changes, then re-execution must reduce validation idx.
    pub fn get_group_size(&self, key: &K, txn_idx: TxnIndex) -> Result<u64, MVGroupError> {
        match self.group_values.get(key) {
            Some(g) => g.get_latest_group_size(txn_idx),
            None => Err(MVGroupError::Uninitialized),
        }
    }

    /// For a given key that corresponds to a group, and an index of a transaction the last
    /// incarnation of which wrote to at least one tag of the group, finalizes the latest
    /// contents of the group. This method works on pointers only and is relatively lighweight,
    /// while subsequent post-processing can clone and serialize the whole group. Note: required
    /// since the output of the block executor still needs to return the whole group contents.
    ///
    /// The method must be called when all transactions <= txn_idx are actually committed, and
    /// the values pointed by weak are guaranteed to be fixed and available during the lifetime
    /// of the data-structure itself.
    pub fn commit_group(&self, key: &K, txn_idx: TxnIndex) -> HashMap<T, Arc<V>> {
        let mut v = self.group_values.get_mut(key).expect("Path must exist");

        v.commit_idx(ShiftedTxnIndex::new(txn_idx))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::types::{
        test::{KeyType, TestValue},
        StorageVersion,
    };
    use claims::{assert_matches, assert_none, assert_ok_eq, assert_some_eq};
    use test_case::test_case;

    #[should_panic]
    #[test_case(0)]
    #[test_case(1)]
    #[test_case(2)]
    fn group_no_path_exists(test_idx: usize) {
        let ap = KeyType(b"/foo/b".to_vec());
        let map = VersionedGroupData::<KeyType<Vec<u8>>, usize, TestValue>::new();

        match test_idx {
            0 => {
                map.mark_estimate(&ap, 1);
            },
            1 => {
                map.delete(&ap, 2);
            },
            2 => {
                map.commit_group(&ap, 0);
            },
            _ => unreachable!("Wrong test index"),
        }
    }

    #[test]
    fn group_uninitialized() {
        let ap_0 = KeyType(b"/foo/a".to_vec());
        let ap_1 = KeyType(b"/foo/b".to_vec());
        let ap_2 = KeyType(b"/foo/c".to_vec());

        let map = VersionedGroupData::<KeyType<Vec<u8>>, usize, TestValue>::new();
        assert_matches!(
            map.get_group_size(&ap_0, 3),
            Err(MVGroupError::Uninitialized)
        );

        map.write(
            ap_1.clone(),
            3,
            1,
            // tags 0, 1, 2.
            (0..2).map(|i| (i, TestValue::with_len(1))),
        );

        // Size should be uninitialized even if the output of lower txn is stored
        // (as long as the base isn't provided).
        assert_matches!(
            map.get_group_size(&ap_1, 3),
            Err(MVGroupError::Uninitialized)
        );
        assert_matches!(
            map.get_group_size(&ap_1, 4),
            Err(MVGroupError::Uninitialized)
        );
        // for reading a tag at ap_1, w.o. returning size, idx = 3 is Uninitialized.
        assert_matches!(
            map.read_from_group(&ap_1, &1, 3),
            Err(MVGroupError::Uninitialized)
        );
        // ... but idx = 4 should find the previously stored value.
        assert_eq!(
            map.read_from_group(&ap_1, &1, 4).unwrap(),
            // Arc compares by value, no return size, incarnation.
            (Ok((3, 1)), Arc::new(TestValue::with_len(1)), None)
        );
        // ap_0 should still be uninitialized.
        assert_matches!(
            map.read_from_group(&ap_0, &1, 3),
            Err(MVGroupError::Uninitialized)
        );

        map.write(
            ap_2.clone(),
            4,
            0,
            // tags 1, 2.
            (1..3).map(|i| (i, TestValue::with_len(4))),
        );
        assert_matches!(
            map.read_from_group(&ap_2, &2, 4),
            Err(MVGroupError::Uninitialized)
        );
        map.provide_base_values(
            ap_2.clone(),
            // base tags 0, 1.
            (0..2).map(|i| (i, TestValue::with_len(2))),
        );

        // Tag not found vs not initialized,
        assert_matches!(
            map.read_from_group(&ap_2, &2, 4),
            Err(MVGroupError::TagNotFound)
        );
        assert_matches!(
            map.read_from_group(&ap_2, &4, 5),
            Err(MVGroupError::TagNotFound)
        );
        // vs finding a versioned entry from txn 4, vs from storage.
        assert_eq!(
            map.read_from_group(&ap_2, &2, 5).unwrap(),
            (Ok((4, 0)), Arc::new(TestValue::with_len(4)), None)
        );
        assert_eq!(
            map.read_from_group(&ap_2, &0, 5).unwrap(),
            (Err(StorageVersion), Arc::new(TestValue::with_len(2)), None)
        );
    }

    #[test]
    fn group_read_write_estimate() {
        use MVGroupError::*;
        let ap = KeyType(b"/foo/f".to_vec());
        let map = VersionedGroupData::<KeyType<Vec<u8>>, usize, TestValue>::new();

        map.write(
            ap.clone(),
            5,
            3,
            // tags 0, 1, values are derived from [txn_idx, incarnation] seed.
            (0..2).map(|i| (i, TestValue::new(vec![5, 3]))),
        );
        assert_eq!(
            map.read_from_group(&ap, &1, 12).unwrap(),
            (Ok((5, 3)), Arc::new(TestValue::new(vec![5, 3])), None)
        );
        map.write(
            ap.clone(),
            10,
            1,
            // tags 1, 2, values are derived from [txn_idx, incarnation] seed.
            (1..3).map(|i| (i, TestValue::new(vec![10, 1]))),
        );
        assert_eq!(
            map.read_from_group(&ap, &1, 12).unwrap(),
            (Ok((10, 1)), Arc::new(TestValue::new(vec![10, 1])), None)
        );

        map.mark_estimate(&ap, 10);
        assert_matches!(map.read_from_group(&ap, &1, 12), Err(Dependency(10)));
        assert_matches!(map.read_from_group(&ap, &2, 12), Err(Dependency(10)));
        assert_matches!(map.read_from_group(&ap, &3, 12), Err(Uninitialized));
        assert_eq!(
            map.read_from_group(&ap, &0, 12).unwrap(),
            (Ok((5, 3)), Arc::new(TestValue::new(vec![5, 3])), None)
        );

        map.delete(&ap, 10);
        assert_eq!(
            map.read_from_group(&ap, &0, 12).unwrap(),
            (Ok((5, 3)), Arc::new(TestValue::new(vec![5, 3])), None)
        );
        assert_eq!(
            map.read_from_group(&ap, &1, 12).unwrap(),
            (Ok((5, 3)), Arc::new(TestValue::new(vec![5, 3])), None)
        );
    }

    #[test]
    fn latest_group_size() {
        use MVGroupError::*;
        let ap = KeyType(b"/foo/f".to_vec());
        let map = VersionedGroupData::<KeyType<Vec<u8>>, usize, TestValue>::new();

        map.write(
            ap.clone(),
            5,
            3,
            // tags 0, 1
            (0..2).map(|i| (i, TestValue::with_len(2))),
        );
        assert_matches!(map.get_group_size(&ap, 12), Err(Uninitialized));

        map.provide_base_values(
            ap.clone(),
            // base tag 1, 2, 3, 4
            (1..5).map(|i| (i, TestValue::with_len(1))),
        );

        let tag: usize = 5;
        let tag_len = bcs::serialized_size(&tag).unwrap();
        let one_entry_len = TestValue::with_len(1).bytes_len();
        let two_entry_len = TestValue::with_len(2).bytes_len();
        let three_entry_len = TestValue::with_len(3).bytes_len();
        let four_entry_len = TestValue::with_len(4).bytes_len();
        let exp_size = 2 * two_entry_len + 3 * one_entry_len + 5 * tag_len;
        assert_ok_eq!(map.get_group_size(&ap, 12), exp_size as u64);

        map.write(
            ap.clone(),
            10,
            1,
            // tags 4, 5
            (4..6).map(|i| (i, TestValue::with_len(3))),
        );
        let exp_size_12 = exp_size + 2 * three_entry_len + tag_len - one_entry_len;
        assert_ok_eq!(map.get_group_size(&ap, 12), exp_size_12 as u64);
        assert_ok_eq!(map.get_group_size(&ap, 10), exp_size as u64);

        map.mark_estimate(&ap, 5);
        assert_matches!(map.get_group_size(&ap, 12), Err(Dependency(5)));
        let exp_size_4 = 4 * (tag_len + one_entry_len);
        assert_ok_eq!(map.get_group_size(&ap, 4), exp_size_4 as u64);

        map.write(
            ap.clone(),
            6,
            1,
            (0..2).map(|i| (i, TestValue::with_len(4))),
        );
        let exp_size_7 = 2 * four_entry_len + 3 * one_entry_len + 5 * tag_len;
        assert_ok_eq!(map.get_group_size(&ap, 7), exp_size_7 as u64);
        assert_matches!(map.get_group_size(&ap, 6), Err(Dependency(5)));

        map.delete(&ap, 5);
        assert_ok_eq!(map.get_group_size(&ap, 6), exp_size_4 as u64);
    }

    #[test]
    fn group_commit_idx() {
        let ap = KeyType(b"/foo/f".to_vec());
        let map = VersionedGroupData::<KeyType<Vec<u8>>, usize, TestValue>::new();

        map.provide_base_values(
            ap.clone(),
            // base tag 1, 2, 3
            (1..4).map(|i| (i, TestValue::from_u128(i as u128))),
        );
        map.write(
            ap.clone(),
            7,
            3,
            // insert at 0, remove at 1.
            vec![
                (0, TestValue::from_u128(100_u128)),
                (1, TestValue::deletion()),
            ],
        );
        map.write(
            ap.clone(),
            3,
            0,
            // tags 2, 3
            (2..4).map(|i| (i, TestValue::from_u128(200 + i as u128))),
        );
        let committed_3 = map.commit_group(&ap, 3);
        // The value at tag 1 is from base, while 2 and 3 are from txn 3.
        // (Arc compares with value equality)
        assert_eq!(committed_3.len(), 3);
        assert_some_eq!(committed_3.get(&1), &Arc::new(TestValue::from_u128(1)));
        assert_some_eq!(
            committed_3.get(&2),
            &Arc::new(TestValue::from_u128(200 + 2))
        );
        assert_some_eq!(
            committed_3.get(&3),
            &Arc::new(TestValue::from_u128(200 + 3))
        );

        map.write(
            ap.clone(),
            5,
            3,
            // tags 3, 4
            (3..5).map(|i| (i, TestValue::from_u128(300 + i as u128))),
        );
        let committed_5 = map.commit_group(&ap, 5);
        assert_eq!(committed_5.len(), 4);
        assert_some_eq!(committed_5.get(&1), &Arc::new(TestValue::from_u128(1)));
        assert_some_eq!(
            committed_5.get(&2),
            &Arc::new(TestValue::from_u128(200 + 2))
        );
        assert_some_eq!(
            committed_5.get(&3),
            &Arc::new(TestValue::from_u128(300 + 3))
        );
        assert_some_eq!(
            committed_5.get(&4),
            &Arc::new(TestValue::from_u128(300 + 4))
        );

        let committed_7 = map.commit_group(&ap, 7);
        assert_eq!(committed_7.len(), 4);
        assert_some_eq!(committed_7.get(&0), &Arc::new(TestValue::from_u128(100)));
        assert_none!(committed_7.get(&1));
        assert_some_eq!(
            committed_7.get(&2),
            &Arc::new(TestValue::from_u128(200 + 2))
        );
        assert_some_eq!(
            committed_7.get(&3),
            &Arc::new(TestValue::from_u128(300 + 3))
        );
        assert_some_eq!(
            committed_7.get(&4),
            &Arc::new(TestValue::from_u128(300 + 4))
        );

        map.write(
            ap.clone(),
            8,
            0,
            // re-insert at 1, delete everything else
            vec![
                (0, TestValue::deletion()),
                (1, TestValue::from_u128(400_u128)),
                (2, TestValue::deletion()),
                (3, TestValue::deletion()),
                (4, TestValue::deletion()),
            ],
        );
        let committed_8 = map.commit_group(&ap, 8);
        assert_eq!(committed_8.len(), 1);
        assert_some_eq!(committed_8.get(&1), &Arc::new(TestValue::from_u128(400)));
    }
}
