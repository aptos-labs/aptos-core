// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    types::{
        Incarnation, MVDataError, MVDataOutput, MVGroupError, ShiftedTxnIndex, TxnIndex,
        ValueWithLayout, Version,
    },
    versioned_data::Entry as SizeEntry,
    VersionedData,
};
use anyhow::anyhow;
use aptos_types::{
    error::{code_invariant_error, PanicError},
    write_set::{TransactionWrite, WriteOpKind},
};
use aptos_vm_types::{resolver::ResourceGroupSize, resource_group_adapter::group_size_as_sum};
use claims::assert_some;
use dashmap::DashMap;
use equivalent::Equivalent;
use move_core_types::value::MoveTypeLayout;
use serde::Serialize;
use std::{
    collections::{
        btree_map::{BTreeMap, Entry::Vacant},
        HashSet,
    },
    fmt::Debug,
    hash::Hash,
    sync::Arc,
};

#[derive(Default)]
struct VersionedGroupSize {
    size_entries: BTreeMap<ShiftedTxnIndex, SizeEntry<ResourceGroupSize>>,
    // Determines whether it is safe for size queries to read the value from an entry marked as
    // ESTIMATE. The heuristic checks on every write, whether the same size would be returned
    // after the respective write took effect. Once set, the flag remains set to true.
    // TODO: Handle remove similarly. May want to depend on transaction indices, i.e. if size
    // has changed early in the block, it may not have an influence on much later transactions.
    size_has_changed: bool,
}

/// Maps each key (access path) to an internal VersionedValue.
pub struct VersionedGroupData<K, T, V> {
    // TODO: Optimize the key represetantion to avoid cloning and concatenation for APIs
    // such as get, where only & of the key is needed.
    values: VersionedData<(K, T), V>,
    // TODO: Once AggregatorV1 is deprecated (no V: TransactionWrite trait bound),
    // switch to VersionedVersionedData<K, ResourceGroupSize>.
    // If an entry exists for a group key in Dashmap, the group is considered initialized.
    group_sizes: DashMap<K, VersionedGroupSize>,

    // Stores a set of tags for this group, basically a superset of all tags encountered in
    // group related APIs. The accesses are synchronized with group size entry (for now),
    // but it is stored separately for conflict free read-path for txn materialization
    // (as the contents of group_tags are used in preparing finalized group contents).
    // Note: The contents of group_tags are non-deterministic, but finalize_group filters
    // out tags for which the latest value does not exist. The implementation invariant
    // that the contents observed in the multi-versioned map after index is committed
    // must correspond to the outputs recorded by the committed transaction incarnations.
    // (and the correctness of the outputs is the responsibility of BlockSTM validation).
    group_tags: DashMap<K, HashSet<T>>,
}

// This struct allows us to reference a group key and tag without cloning
#[derive(Clone)]
struct GroupKeyRef<'a, K, T> {
    group_key: &'a K,
    tag: &'a T,
}

// Implement Equivalent for GroupKeyRef so it can be used to look up (K, T) keys
impl<'a, K, T> Equivalent<(K, T)> for GroupKeyRef<'a, K, T>
where
    K: Eq,
    T: Eq,
{
    fn equivalent(&self, key: &(K, T)) -> bool {
        self.group_key == &key.0 && self.tag == &key.1
    }
}

// Implement Hash for GroupKeyRef to satisfy dashmap's key requirements
impl<'a, K: Hash, T: Hash> std::hash::Hash for GroupKeyRef<'a, K, T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Hash the same way as (K, T) would hash
        self.group_key.hash(state);
        self.tag.hash(state);
    }
}

// Implement Debug for better error messages
impl<'a, K: Debug, T: Debug> Debug for GroupKeyRef<'a, K, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupKeyRef")
            .field("group_key", &self.group_key)
            .field("tag", &self.tag)
            .finish()
    }
}

impl<
        K: Hash + Clone + Debug + Eq,
        T: Hash + Clone + Debug + Eq + Serialize,
        V: TransactionWrite,
    > VersionedGroupData<K, T, V>
{
    pub(crate) fn empty() -> Self {
        Self {
            values: VersionedData::empty(),
            group_sizes: DashMap::new(),
            group_tags: DashMap::new(),
        }
    }

    pub(crate) fn num_keys(&self) -> usize {
        self.group_sizes.len()
    }

    pub fn set_raw_base_values(
        &self,
        group_key: K,
        base_values: Vec<(T, V)>,
    ) -> anyhow::Result<()> {
        let mut group_sizes = self.group_sizes.entry(group_key.clone()).or_default();

        if let Vacant(entry) = group_sizes.size_entries.entry(ShiftedTxnIndex::zero_idx()) {
            // Perform group size computation if base not already provided.
            let group_size = group_size_as_sum::<T>(
                base_values
                    .iter()
                    .flat_map(|(tag, value)| value.bytes().map(|b| (tag.clone(), b.len()))),
            )
            .map_err(|e| {
                anyhow!(
                    "Tag serialization error in resource group at {:?}: {:?}",
                    group_key.clone(),
                    e
                )
            })?;

            entry.insert(SizeEntry::new(group_size));

            let mut superset_tags = self.group_tags.entry(group_key.clone()).or_default();
            for (tag, value) in base_values.into_iter() {
                superset_tags.insert(tag.clone());
                self.values.set_base_value(
                    (group_key.clone(), tag),
                    ValueWithLayout::RawFromStorage(Arc::new(value)),
                );
            }
        }

        Ok(())
    }

    pub fn update_tagged_base_value_with_layout(
        &self,
        group_key: K,
        tag: T,
        value: V,
        layout: Option<Arc<MoveTypeLayout>>,
    ) {
        self.values.set_base_value(
            (group_key, tag),
            ValueWithLayout::Exchanged(Arc::new(value), layout.clone()),
        );
    }

    /// Writes new resource group values (and size) specified by tag / value pair
    /// iterators. Returns true if a new tag is written compared to the previous
    /// incarnation (set of previous tags provided as a parameter), or if the size
    /// as observed after the new write differs from before the write took place.
    /// In these cases the caller (Block-STM) may have to do certain validations.
    pub fn write(
        &self,
        group_key: K,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        values: impl IntoIterator<Item = (T, (V, Option<Arc<MoveTypeLayout>>))>,
        size: ResourceGroupSize,
        mut prev_tags: HashSet<T>,
    ) -> Result<bool, PanicError> {
        let mut ret = false;
        let mut tags_to_write = vec![];

        {
            let superset_tags = self.group_tags.get(&group_key).ok_or_else(|| {
                // Due to read-before-write.
                code_invariant_error("Group (tags) must be initialized to write to")
            })?;

            for (tag, (value, layout)) in values.into_iter() {
                if !superset_tags.contains(&tag) {
                    tags_to_write.push(tag.clone());
                }

                ret |= !prev_tags.remove(&tag);

                self.values.write(
                    (group_key.clone(), tag),
                    txn_idx,
                    incarnation,
                    Arc::new(value),
                    layout,
                );
            }
        }

        for prev_tag in prev_tags {
            let key = (group_key.clone(), prev_tag);
            self.values.remove(&key, txn_idx);
        }

        if !tags_to_write.is_empty() {
            let mut superset_tags = self
                .group_tags
                .get_mut(&group_key)
                .expect("Group must be initialized");
            superset_tags.extend(tags_to_write);
        }

        let mut group_sizes = self.group_sizes.get_mut(&group_key).ok_or_else(|| {
            // Due to read-before-write.
            code_invariant_error("Group (sizes) must be initialized to write to")
        })?;

        if !(group_sizes.size_has_changed && ret) {
            let (size_changed, update_flag) = group_sizes
                .size_entries
                .range(ShiftedTxnIndex::zero_idx()..ShiftedTxnIndex::new(txn_idx + 1))
                .next_back()
                .ok_or_else(|| {
                    code_invariant_error("Initialized group sizes must contain storage version")
                })
                .map(|(idx, prev_size)| {
                    (
                        prev_size.value != size,
                        // Update the size_has_changed flag if the entry isn't the base value
                        // (which may be non-existent) or if the incarnation > 0.
                        *idx != ShiftedTxnIndex::zero_idx() || incarnation > 0,
                    )
                })?;

            if size_changed {
                ret = true;
                if update_flag {
                    group_sizes.size_has_changed = true;
                }
            }
        }

        group_sizes
            .size_entries
            .insert(ShiftedTxnIndex::new(txn_idx), SizeEntry::new(size));

        Ok(ret)
    }

    /// Mark all entry from transaction 'txn_idx' at access path 'key' as an estimated write
    /// (for future incarnation). Will panic if the entry is not in the data-structure.
    pub fn mark_estimate(&self, group_key: &K, txn_idx: TxnIndex, tags: HashSet<T>) {
        for tag in tags.iter() {
            // Use GroupKeyRef to avoid cloning the group_key
            let key_ref = GroupKeyRef { group_key, tag };
            self.values.mark_estimate(&key_ref, txn_idx);
        }

        self.group_sizes
            .get(group_key)
            .expect("Path must exist")
            .size_entries
            .get(&ShiftedTxnIndex::new(txn_idx))
            .expect("Entry by the txn must exist to mark estimate")
            .mark_estimate();
    }

    /// Remove all entries from transaction 'txn_idx' at access path 'key'.
    pub fn remove(&self, group_key: &K, txn_idx: TxnIndex, tags: HashSet<T>) {
        for tag in tags.iter() {
            let key_ref = GroupKeyRef { group_key, tag };
            self.values.remove(&key_ref, txn_idx);
        }

        // TODO: consider setting size_has_changed flag if e.g. the size observed
        // after remove is different.
        assert_some!(
            self.group_sizes
                .get_mut(group_key)
                .expect("Path must exist")
                .size_entries
                .remove(&ShiftedTxnIndex::new(txn_idx)),
            "Entry for the txn must exist to be deleted"
        );
    }

    /// Read the latest value corresponding to a tag at a given group (identified by key).
    /// Return the size of the group (if requested), as defined above, alongside the version
    /// information (None if storage/pre-block version).
    /// If the layout of the resource is current UnSet, this function sets the layout of the
    /// group to the provided layout.
    pub fn fetch_tagged_data(
        &self,
        group_key: &K,
        tag: &T,
        txn_idx: TxnIndex,
    ) -> Result<(Version, ValueWithLayout<V>), MVGroupError> {
        let key_ref = GroupKeyRef { group_key, tag };
        let initialized = self.group_sizes.contains_key(group_key);

        match self.values.fetch_data(&key_ref, txn_idx) {
            Ok(MVDataOutput::Versioned(version, value)) => Ok((version, value)),
            Err(MVDataError::Uninitialized) => Err(if initialized {
                MVGroupError::TagNotFound
            } else {
                MVGroupError::Uninitialized
            }),
            Err(MVDataError::Dependency(dep_idx)) => Err(MVGroupError::Dependency(dep_idx)),
            Ok(MVDataOutput::Resolved(_))
            | Err(MVDataError::Unresolved(_))
            | Err(MVDataError::DeltaApplicationFailure) => {
                unreachable!("Not using aggregatorV1")
            },
        }
    }

    pub fn get_group_size(
        &self,
        group_key: &K,
        txn_idx: TxnIndex,
    ) -> Result<ResourceGroupSize, MVGroupError> {
        match self.group_sizes.get(group_key) {
            Some(g) => g
                .size_entries
                .range(ShiftedTxnIndex::zero_idx()..ShiftedTxnIndex::new(txn_idx))
                .next_back()
                .map(|(idx, size)| {
                    if size.is_estimate() && g.size_has_changed {
                        Err(MVGroupError::Dependency(
                            idx.idx().expect("May not depend on storage version"),
                        ))
                    } else {
                        Ok(size.value)
                    }
                })
                .unwrap_or(Err(MVGroupError::Uninitialized)),
            None => Err(MVGroupError::Uninitialized),
        }
    }

    pub fn validate_group_size(
        &self,
        group_key: &K,
        txn_idx: TxnIndex,
        group_size_to_validate: ResourceGroupSize,
    ) -> bool {
        self.get_group_size(group_key, txn_idx) == Ok(group_size_to_validate)
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
    ///
    /// The method checks that each committed write op kind is consistent with the existence of
    /// a previous value of the resource (must be creation iff no previous value, deletion or
    /// modification otherwise). When consistent, the output is Ok(..).
    pub fn finalize_group(
        &self,
        group_key: &K,
        txn_idx: TxnIndex,
    ) -> Result<(Vec<(T, ValueWithLayout<V>)>, ResourceGroupSize), PanicError> {
        let superset_tags = self
            .group_tags
            .get(group_key)
            .expect("Group tags must be set")
            .clone();

        let committed_group = superset_tags
            .into_iter()
            .map(
                |tag| match self.fetch_tagged_data(group_key, &tag, txn_idx + 1) {
                    Ok((_, value)) => Ok((value.write_op_kind() != WriteOpKind::Deletion)
                        .then(|| (tag, value.clone()))),
                    Err(MVGroupError::TagNotFound) => Ok(None),
                    Err(e) => Err(code_invariant_error(format!(
                        "Unexpected error in finalize group fetching value {:?}",
                        e
                    ))),
                },
            )
            .collect::<Result<Vec<_>, PanicError>>()?
            .into_iter()
            .flatten()
            .collect();
        Ok((
            committed_group,
            self.get_group_size(group_key, txn_idx + 1).map_err(|e| {
                code_invariant_error(format!(
                    "Unexpected error in finalize group get size {:?}",
                    e
                ))
            })?,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::types::{
        test::{KeyType, TestValue},
        StorageVersion,
    };
    use claims::{
        assert_err, assert_matches, assert_none, assert_ok, assert_ok_eq, assert_some_eq,
    };
    use std::collections::HashMap;
    use test_case::test_case;

    #[should_panic]
    #[test_case(0)]
    #[test_case(1)]
    #[test_case(2)]
    fn group_no_path_exists(test_idx: usize) {
        let ap = KeyType(b"/foo/b".to_vec());
        let map = VersionedGroupData::<KeyType<Vec<u8>>, usize, TestValue>::empty();

        match test_idx {
            0 => {
                map.mark_estimate(&ap, 1, HashSet::new());
            },
            1 => {
                map.remove(&ap, 2, HashSet::new());
            },
            2 => {
                let _ = map.finalize_group(&ap, 0);
            },
            _ => unreachable!("Wrong test index"),
        }
    }

    #[test]
    fn group_write_behavior_changes() {
        let ap_0 = KeyType(b"/foo/a".to_vec());
        let ap_1 = KeyType(b"/foo/b".to_vec());
        let map = VersionedGroupData::<KeyType<Vec<u8>>, usize, TestValue>::empty();
        assert_ok!(map.set_raw_base_values(ap_0.clone(), vec![]));
        assert_ok!(map.set_raw_base_values(ap_1.clone(), vec![]));

        let test_values = vec![
            (0usize, (TestValue::creation_with_len(1), None)),
            (1usize, (TestValue::creation_with_len(1), None)),
        ];
        let test_tags: HashSet<usize> = (0..2).collect();

        // Sizes do need to be accurate with respect to written values for test.
        let fake_size = ResourceGroupSize::Combined {
            num_tagged_resources: 2,
            all_tagged_resources_size: 20,
        };
        let fake_changed_size = ResourceGroupSize::Combined {
            num_tagged_resources: 3,
            all_tagged_resources_size: 20,
        };

        let check_write = |ap: &KeyType<Vec<u8>>,
                           idx,
                           incarnation,
                           size,
                           prev_tags,
                           expected_write_ret,
                           expected_size_changed| {
            assert_ok_eq!(
                map.write(
                    ap.clone(),
                    idx,
                    incarnation,
                    test_values.clone().into_iter(),
                    size,
                    prev_tags,
                ),
                expected_write_ret
            );

            assert_eq!(
                map.group_sizes.get(ap).unwrap().size_has_changed,
                expected_size_changed,
            );
            assert_eq!(
                map.group_sizes
                    .get(ap)
                    .unwrap()
                    .size_entries
                    .get(&ShiftedTxnIndex::new(idx))
                    .unwrap()
                    .value,
                size
            );
        };

        // Incarnation 0 changes behavior due to empty prior tags, leading to write returning Ok(false),
        // but it should not set the size_changed flag.
        check_write(&ap_0, 3, 0, fake_size, HashSet::new(), true, false);
        // However, if the first write is by incarnation >0, then size_has_changed will also be set.
        check_write(&ap_1, 5, 1, fake_size, HashSet::new(), true, true);

        // Incarnation 1 does not change size.
        check_write(&ap_0, 3, 1, fake_size, test_tags.clone(), false, false);
        // Even with incarnation > 0, observed size does not change.
        check_write(&ap_0, 4, 1, fake_size, HashSet::new(), true, false);

        // Incarnation 2 changes size.
        check_write(
            &ap_0,
            3,
            2,
            fake_changed_size,
            test_tags.clone(),
            true,
            true,
        );
        // Once size_changed is set, it stays true.
        check_write(
            &ap_0,
            3,
            3,
            fake_changed_size,
            test_tags.clone(),
            false,
            true,
        );
        check_write(&ap_0, 6, 0, fake_changed_size, HashSet::new(), true, true);
    }

    #[test]
    fn group_initialize_and_write() {
        let ap = KeyType(b"/foo/a".to_vec());
        let ap_empty = KeyType(b"/foo/b".to_vec());

        let map = VersionedGroupData::<KeyType<Vec<u8>>, usize, TestValue>::empty();
        assert_matches!(map.get_group_size(&ap, 3), Err(MVGroupError::Uninitialized));
        assert_matches!(
            map.fetch_tagged_data(&ap, &1, 3),
            Err(MVGroupError::Uninitialized)
        );

        // Does not need to be accurate.
        let idx_3_size = ResourceGroupSize::Combined {
            num_tagged_resources: 2,
            all_tagged_resources_size: 20,
        };
        // Write should fail because group is not initialized (R before W, where
        // and read causes the base values/size to be set).
        assert_err!(map.write(
            ap.clone(),
            3,
            1,
            (0..2).map(|i| (i, (TestValue::creation_with_len(1), None))),
            idx_3_size,
            HashSet::new(),
        ));
        assert_ok!(map.set_raw_base_values(ap.clone(), vec![]));
        // Write should now succeed.
        assert_ok!(map.write(
            ap.clone(),
            3,
            1,
            // tags 0, 1, 2.
            (0..2).map(|i| (i, (TestValue::creation_with_len(1), None))),
            idx_3_size,
            HashSet::new(),
        ));

        // Check sizes.
        assert_ok_eq!(map.get_group_size(&ap, 4), idx_3_size);
        assert_ok_eq!(
            map.get_group_size(&ap, 3),
            ResourceGroupSize::zero_combined()
        );

        // Check values.
        assert_matches!(
            map.fetch_tagged_data(&ap, &1, 3),
            Err(MVGroupError::TagNotFound)
        );
        assert_matches!(
            map.fetch_tagged_data(&ap, &3, 4),
            Err(MVGroupError::TagNotFound)
        );
        // ... but idx = 4 should find the previously stored value.
        assert_eq!(
            map.fetch_tagged_data(&ap, &1, 4).unwrap(),
            (
                Ok((3, 1)),
                ValueWithLayout::Exchanged(Arc::new(TestValue::creation_with_len(1)), None)
            )
        );

        // ap_empty should still be uninitialized.
        assert_matches!(
            map.fetch_tagged_data(&ap_empty, &1, 3),
            Err(MVGroupError::Uninitialized)
        );
        assert_matches!(
            map.get_group_size(&ap_empty, 3),
            Err(MVGroupError::Uninitialized)
        );
    }

    #[test]
    fn group_base_and_write() {
        let ap = KeyType(b"/foo/a".to_vec());
        let map = VersionedGroupData::<KeyType<Vec<u8>>, usize, TestValue>::empty();

        // base tags 0, 1.
        let base_values = vec![
            (0usize, TestValue::creation_with_len(1)),
            (1usize, TestValue::creation_with_len(2)),
        ];
        assert_ok!(map.set_raw_base_values(ap.clone(), base_values));

        assert_ok!(map.write(
            ap.clone(),
            4,
            0,
            // tags 1, 2.
            (1..3).map(|i| (i, (TestValue::creation_with_len(4), None))),
            ResourceGroupSize::zero_combined(),
            HashSet::new(),
        ));

        assert_matches!(
            map.fetch_tagged_data(&ap, &2, 4),
            Err(MVGroupError::TagNotFound)
        );
        assert_matches!(
            map.fetch_tagged_data(&ap, &3, 5),
            Err(MVGroupError::TagNotFound)
        );
        assert_eq!(
            map.fetch_tagged_data(&ap, &2, 5).unwrap(),
            (
                Ok((4, 0)),
                ValueWithLayout::Exchanged(Arc::new(TestValue::creation_with_len(4)), None)
            )
        );
        assert_eq!(
            map.fetch_tagged_data(&ap, &1, 4).unwrap(),
            (
                Err(StorageVersion),
                ValueWithLayout::RawFromStorage(Arc::new(TestValue::creation_with_len(2)))
            )
        );
        assert_eq!(
            map.fetch_tagged_data(&ap, &0, 6).unwrap(),
            (
                Err(StorageVersion),
                ValueWithLayout::RawFromStorage(Arc::new(TestValue::creation_with_len(1)))
            )
        );
    }

    #[test]
    fn group_read_write_estimate() {
        use MVGroupError::*;
        let ap = KeyType(b"/foo/f".to_vec());
        let map = VersionedGroupData::<KeyType<Vec<u8>>, usize, TestValue>::empty();

        let idx_5_size = ResourceGroupSize::Combined {
            num_tagged_resources: 2,
            all_tagged_resources_size: 20,
        };

        assert_ok!(map.set_raw_base_values(ap.clone(), vec![]));
        assert_ok!(map.write(
            ap.clone(),
            5,
            3,
            // tags 0, 1, values are derived from [txn_idx, incarnation] seed.
            (0..2).map(|i| (i, (TestValue::new(vec![5, 3]), None))),
            idx_5_size,
            HashSet::new(),
        ));
        assert_eq!(
            map.fetch_tagged_data(&ap, &1, 12).unwrap(),
            (
                Ok((5, 3)),
                ValueWithLayout::Exchanged(Arc::new(TestValue::new(vec![5, 3])), None)
            )
        );
        assert_ok!(map.write(
            ap.clone(),
            10,
            1,
            // tags 1, 2, values are derived from [txn_idx, incarnation] seed.
            (1..3).map(|i| (i, (TestValue::new(vec![10, 1]), None))),
            ResourceGroupSize::zero_combined(),
            HashSet::new(),
        ));
        assert_eq!(
            map.fetch_tagged_data(&ap, &1, 12).unwrap(),
            (
                Ok((10, 1)),
                ValueWithLayout::Exchanged(Arc::new(TestValue::new(vec![10, 1])), None)
            )
        );

        map.mark_estimate(&ap, 10, (1..3).collect());
        assert_matches!(map.fetch_tagged_data(&ap, &1, 12), Err(Dependency(10)));
        assert_matches!(map.fetch_tagged_data(&ap, &2, 12), Err(Dependency(10)));
        assert_matches!(map.fetch_tagged_data(&ap, &3, 12), Err(TagNotFound));
        assert_eq!(
            map.fetch_tagged_data(&ap, &0, 12).unwrap(),
            (
                Ok((5, 3)),
                ValueWithLayout::Exchanged(Arc::new(TestValue::new(vec![5, 3])), None)
            )
        );
        assert_matches!(map.get_group_size(&ap, 12), Err(Dependency(10)));

        map.remove(&ap, 10, (1..3).collect());
        assert_eq!(
            map.fetch_tagged_data(&ap, &0, 12).unwrap(),
            (
                Ok((5, 3)),
                ValueWithLayout::Exchanged(Arc::new(TestValue::new(vec![5, 3])), None)
            )
        );
        assert_eq!(
            map.fetch_tagged_data(&ap, &1, 12).unwrap(),
            (
                Ok((5, 3)),
                ValueWithLayout::Exchanged(Arc::new(TestValue::new(vec![5, 3])), None)
            )
        );

        // Size should also be removed at 10.
        assert_ok_eq!(map.get_group_size(&ap, 12), idx_5_size);
    }

    #[test]
    fn group_size_changed_dependency() {
        let ap = KeyType(b"/foo/f".to_vec());
        let map = VersionedGroupData::<KeyType<Vec<u8>>, usize, TestValue>::empty();

        let tag: usize = 5;
        let one_entry_len = TestValue::creation_with_len(1).bytes().unwrap().len();
        let two_entry_len = TestValue::creation_with_len(2).bytes().unwrap().len();
        let idx_5_size = group_size_as_sum(vec![(&tag, two_entry_len); 2].into_iter().chain(vec![
            (
                &tag,
                one_entry_len
            );
            3
        ]))
        .unwrap();
        let base_size = group_size_as_sum(vec![(&tag, one_entry_len); 4].into_iter()).unwrap();
        let idx_5_size_with_ones =
            group_size_as_sum(vec![(&tag, one_entry_len); 5].into_iter()).unwrap();

        assert_ok!(map.set_raw_base_values(
            ap.clone(),
            // base tag 1, 2, 3, 4
            (1..5)
                .map(|i| (i, TestValue::creation_with_len(1)))
                .collect(),
        ));
        assert_ok!(map.write(
            ap.clone(),
            5,
            0,
            // tags 0, 1
            (0..2).map(|i| (i, (TestValue::creation_with_len(2), None))),
            idx_5_size,
            HashSet::new(),
        ));

        // Incarnation 0 and base values should not affect size_changed flag.
        assert!(!map.group_sizes.get(&ap).unwrap().size_has_changed);

        assert_ok_eq!(map.get_group_size(&ap, 5), base_size);
        assert!(map.validate_group_size(&ap, 4, base_size));
        assert!(!map.validate_group_size(&ap, 5, idx_5_size));
        assert_ok_eq!(map.get_group_size(&ap, 6), idx_5_size);

        // Despite estimates, should still return size.
        map.mark_estimate(&ap, 5, (0..2).collect());
        assert_ok_eq!(map.get_group_size(&ap, 12), idx_5_size);
        assert!(map.validate_group_size(&ap, 12, idx_5_size));
        assert!(!map.validate_group_size(&ap, 12, ResourceGroupSize::zero_combined()));

        // Different write, same size again.
        assert_ok_eq!(
            map.write(
                ap.clone(),
                5,
                1,
                (0..3).map(|i| (i, (TestValue::creation_with_len(2), None))),
                idx_5_size,
                (0..2).collect(),
            ),
            true
        );
        assert!(!map.group_sizes.get(&ap).unwrap().size_has_changed);
        map.mark_estimate(&ap, 5, (0..2).collect());
        assert_ok_eq!(map.get_group_size(&ap, 12), idx_5_size);
        assert!(map.validate_group_size(&ap, 12, idx_5_size));
        assert!(!map.validate_group_size(&ap, 12, ResourceGroupSize::zero_concrete()));

        // Remove currently does not affect size_has_changed.
        map.remove(&ap, 5, (0..3).collect());
        assert!(!map.group_sizes.get(&ap).unwrap().size_has_changed);
        assert_ok_eq!(map.get_group_size(&ap, 4), base_size);
        assert!(map.validate_group_size(&ap, 6, base_size));

        assert_ok!(map.write(
            ap.clone(),
            5,
            2,
            (0..3).map(|i| (i, (TestValue::creation_with_len(1), None))),
            idx_5_size_with_ones,
            (0..2).collect(),
        ));
        // Size has changed between speculative writes.
        assert!(map.group_sizes.get(&ap).unwrap().size_has_changed);
        assert_ok_eq!(map.get_group_size(&ap, 10), idx_5_size_with_ones);
        assert!(map.validate_group_size(&ap, 10, idx_5_size_with_ones));
        assert!(!map.validate_group_size(&ap, 10, idx_5_size));
        assert_ok_eq!(map.get_group_size(&ap, 3), base_size);

        map.mark_estimate(&ap, 5, (0..3).collect());
        assert_matches!(
            map.get_group_size(&ap, 12),
            Err(MVGroupError::Dependency(5))
        );
        assert!(!map.validate_group_size(&ap, 12, idx_5_size_with_ones));
        assert!(!map.validate_group_size(&ap, 12, idx_5_size));
    }

    #[test]
    fn group_write_tags_change_behavior() {
        let ap = KeyType(b"/foo/1".to_vec());

        let map = VersionedGroupData::<KeyType<Vec<u8>>, usize, TestValue>::empty();
        assert_ok!(map.set_raw_base_values(ap.clone(), vec![],));

        assert_ok_eq!(
            map.write(
                ap.clone(),
                5,
                0,
                // tags 0, 1
                (0..2).map(|i| (i, (TestValue::creation_with_len(2), None))),
                ResourceGroupSize::zero_combined(),
                HashSet::new(),
            ),
            true,
        );
        // Write changes behavior (requiring re-validation) because of tags only when
        // the new tags are not contained in the old tags. Not when a tag is no longer
        // written. This is because no information about a resource in a group is
        // validated by equality (group size and metadata are stored separately) -
        // and in this sense resources in group are like normal resources.
        assert_ok_eq!(
            map.write(
                ap.clone(),
                5,
                1,
                // tags 0 - contained among {0, 1}
                (0..1).map(|i| (i, (TestValue::creation_with_len(2), None))),
                ResourceGroupSize::zero_combined(),
                (0..2).collect(),
            ),
            false
        );
        assert_ok_eq!(
            map.write(
                ap.clone(),
                5,
                2,
                // tags 0, 1 - not contained among {0}
                (0..2).map(|i| (i, (TestValue::creation_with_len(2), None))),
                ResourceGroupSize::zero_combined(),
                (0..1).collect(),
            ),
            true
        );
    }

    fn finalize_group_as_hashmap(
        map: &VersionedGroupData<KeyType<Vec<u8>>, usize, TestValue>,
        key: &KeyType<Vec<u8>>,
        idx: TxnIndex,
    ) -> (
        HashMap<usize, ValueWithLayout<TestValue>>,
        ResourceGroupSize,
    ) {
        let (group, size) = map.finalize_group(key, idx).unwrap();

        (group.into_iter().collect(), size)
    }

    #[test]
    fn group_finalize() {
        let ap = KeyType(b"/foo/f".to_vec());
        let map = VersionedGroupData::<KeyType<Vec<u8>>, usize, TestValue>::empty();

        let base_values: Vec<_> = (1..4)
            .map(|i| (i, TestValue::creation_with_len(i)))
            .collect();

        assert_ok!(map.set_raw_base_values(
            ap.clone(),
            // base tag 1, 2, 3
            base_values.clone(),
        ));
        let base_size = group_size_as_sum(
            base_values
                .into_iter()
                .map(|(tag, value)| (tag, value.bytes().unwrap().len())),
        )
        .unwrap();

        // Does not need to be accurate.
        let idx_3_size = ResourceGroupSize::Combined {
            num_tagged_resources: 2,
            all_tagged_resources_size: 20,
        };
        let idx_5_size = ResourceGroupSize::Combined {
            num_tagged_resources: 5,
            all_tagged_resources_size: 50,
        };
        let idx_7_size = ResourceGroupSize::Combined {
            num_tagged_resources: 7,
            all_tagged_resources_size: 70,
        };
        let idx_8_size = ResourceGroupSize::Combined {
            num_tagged_resources: 8,
            all_tagged_resources_size: 80,
        };

        assert_ok!(map.write(
            ap.clone(),
            7,
            3,
            // insert at 0, remove at 1.
            vec![
                (0, (TestValue::creation_with_len(100), None)),
                (1, (TestValue::deletion(), None)),
            ],
            idx_7_size,
            HashSet::new(),
        ));
        assert_ok!(map.write(
            ap.clone(),
            3,
            0,
            // tags 2, 3
            (2..4).map(|i| (i, (TestValue::creation_with_len(200 + i), None))),
            idx_3_size,
            HashSet::new(),
        ));

        let (finalized_3, size_3) = finalize_group_as_hashmap(&map, &ap, 3);
        // Finalize returns size recorded by txn 3, while get_group_size at txn index
        // 3 must return the size recorded below it.
        assert_eq!(size_3, idx_3_size);
        assert_ok_eq!(map.get_group_size(&ap, 3), base_size,);

        // The value at tag 1 is from base, while 2 and 3 are from txn 3.
        // (Arc compares with value equality)
        assert_eq!(finalized_3.len(), 3);
        assert_some_eq!(
            finalized_3.get(&1),
            &ValueWithLayout::RawFromStorage(Arc::new(TestValue::creation_with_len(1)))
        );
        assert_some_eq!(
            finalized_3.get(&2),
            &ValueWithLayout::Exchanged(Arc::new(TestValue::creation_with_len(202)), None)
        );
        assert_some_eq!(
            finalized_3.get(&3),
            &ValueWithLayout::Exchanged(Arc::new(TestValue::creation_with_len(203)), None)
        );

        assert_ok!(map.write(
            ap.clone(),
            5,
            3,
            vec![
                (3, (TestValue::creation_with_len(303), None)),
                (4, (TestValue::creation_with_len(304), None)),
            ],
            idx_5_size,
            HashSet::new(),
        ));
        // Finalize should work even for indices without writes.
        let (finalized_6, size_6) = finalize_group_as_hashmap(&map, &ap, 6);
        assert_eq!(size_6, idx_5_size);
        assert_eq!(finalized_6.len(), 4);
        assert_some_eq!(
            finalized_6.get(&1),
            &ValueWithLayout::RawFromStorage(Arc::new(TestValue::creation_with_len(1)))
        );
        assert_some_eq!(
            finalized_6.get(&2),
            &ValueWithLayout::Exchanged(Arc::new(TestValue::creation_with_len(202)), None)
        );
        assert_some_eq!(
            finalized_6.get(&3),
            &ValueWithLayout::Exchanged(Arc::new(TestValue::creation_with_len(303)), None)
        );
        assert_some_eq!(
            finalized_6.get(&4),
            &ValueWithLayout::Exchanged(Arc::new(TestValue::creation_with_len(304)), None)
        );

        let (finalized_7, size_7) = finalize_group_as_hashmap(&map, &ap, 7);
        assert_eq!(size_7, idx_7_size);
        assert_eq!(finalized_7.len(), 4);
        assert_some_eq!(
            finalized_7.get(&0),
            &ValueWithLayout::Exchanged(Arc::new(TestValue::creation_with_len(100)), None)
        );
        assert_none!(finalized_7.get(&1));
        assert_some_eq!(
            finalized_7.get(&2),
            &ValueWithLayout::Exchanged(Arc::new(TestValue::creation_with_len(202)), None)
        );
        assert_some_eq!(
            finalized_7.get(&3),
            &ValueWithLayout::Exchanged(Arc::new(TestValue::creation_with_len(303)), None)
        );
        assert_some_eq!(
            finalized_7.get(&4),
            &ValueWithLayout::Exchanged(Arc::new(TestValue::creation_with_len(304)), None)
        );

        assert_ok!(map.write(
            ap.clone(),
            8,
            0,
            // re-insert at 1, remove everything else
            vec![
                (0, (TestValue::deletion(), None)),
                (1, (TestValue::creation_with_len(400), None)),
                (2, (TestValue::deletion(), None)),
                (3, (TestValue::deletion(), None)),
                (4, (TestValue::deletion(), None)),
            ],
            idx_8_size,
            HashSet::new(),
        ));
        let (finalized_8, size_8) = finalize_group_as_hashmap(&map, &ap, 8);
        assert_eq!(size_8, idx_8_size);
        assert_eq!(finalized_8.len(), 1);
        assert_some_eq!(
            finalized_8.get(&1),
            &ValueWithLayout::Exchanged(Arc::new(TestValue::creation_with_len(400)), None)
        );
    }

    // TODO[agg_v2](test) Test with non trivial layout.
    #[test]
    fn group_base_layout() {
        let ap = KeyType(b"/foo/f".to_vec());
        let map = VersionedGroupData::<KeyType<Vec<u8>>, usize, TestValue>::empty();

        assert_ok!(map.set_raw_base_values(ap.clone(), vec![(1, TestValue::creation_with_len(1))],));
        assert_eq!(
            map.fetch_tagged_data(&ap, &1, 6).unwrap(),
            (
                Err(StorageVersion),
                ValueWithLayout::RawFromStorage(Arc::new(TestValue::creation_with_len(1)))
            )
        );

        map.update_tagged_base_value_with_layout(
            ap.clone(),
            1,
            TestValue::creation_with_len(1),
            None,
        );
        assert_eq!(
            map.fetch_tagged_data(&ap, &1, 6).unwrap(),
            (
                Err(StorageVersion),
                ValueWithLayout::Exchanged(Arc::new(TestValue::creation_with_len(1)), None)
            )
        );
    }
}
