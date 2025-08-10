// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    registered_dependencies::{take_dependencies, RegisteredReadDependencies},
    types::{
        Incarnation, MVDataError, MVDataOutput, MVGroupError, ShiftedTxnIndex, TxnIndex,
        ValueWithLayout, Version,
    },
    versioned_data::Entry as SizeEntry,
    VersionedData,
};
use anyhow::anyhow;
use aptos_aggregator::types::ReadPosition;
use aptos_infallible::Mutex;
use aptos_types::{
    error::{code_invariant_error, PanicError},
    write_set::{TransactionWrite, WriteOpKind},
};
use aptos_vm_types::{resolver::ResourceGroupSize, resource_group_adapter::group_size_as_sum};
use claims::{assert_ok, assert_some};
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

struct SizeAndDependencies {
    size: ResourceGroupSize,
    dependencies: Mutex<RegisteredReadDependencies>,
}

impl SizeAndDependencies {
    fn from_size(size: ResourceGroupSize) -> Self {
        Self {
            size,
            dependencies: Mutex::new(RegisteredReadDependencies::new()),
        }
    }

    fn from_size_and_dependencies(
        size: ResourceGroupSize,
        dependencies: BTreeMap<TxnIndex, Incarnation>,
    ) -> Self {
        Self {
            size,
            dependencies: Mutex::new(RegisteredReadDependencies::from_dependencies(dependencies)),
        }
    }
}

// TODO(BlockSTMv2): Refactoring of Data and Groups multi-versioned map logic
// so size dependencies can be handled in a unified way.
#[derive(Default)]
struct VersionedGroupSize {
    size_entries: BTreeMap<ShiftedTxnIndex, SizeEntry<SizeAndDependencies>>,
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
    // switch to VersionedData<K, ResourceGroupSize>.
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
        V: TransactionWrite + PartialEq,
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

        // Currently the size & value are written while holding the sizes lock.
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

            entry.insert(SizeEntry::new(SizeAndDependencies::from_size(group_size)));

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
        prev_tags: HashSet<T>,
    ) -> Result<bool, PanicError> {
        let mut group_sizes = self.group_sizes.get_mut(&group_key).ok_or_else(|| {
            // Due to read-before-write.
            code_invariant_error("Group (sizes) must be initialized to write to")
        })?;
        let (mut ret, _) = self.data_write_impl::<false>(
            &group_key,
            txn_idx,
            incarnation,
            values,
            prev_tags.iter().collect(),
        )?;

        if !(group_sizes.size_has_changed && ret) {
            let (size_changed, update_flag) = Self::get_latest_entry(
                &group_sizes.size_entries,
                txn_idx,
                ReadPosition::AfterCurrentTxn,
            )
            .ok_or_else(|| {
                code_invariant_error("Initialized group sizes must contain storage version")
            })
            .map(|(idx, prev_size)| {
                (
                    prev_size.value.size != size,
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

        group_sizes.size_entries.insert(
            ShiftedTxnIndex::new(txn_idx),
            SizeEntry::new(SizeAndDependencies::from_size(size)),
        );

        Ok(ret)
    }

    pub fn write_v2(
        &self,
        group_key: K,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        values: impl IntoIterator<Item = (T, (V, Option<Arc<MoveTypeLayout>>))>,
        size: ResourceGroupSize,
        prev_tags: HashSet<&T>,
    ) -> Result<BTreeMap<TxnIndex, Incarnation>, PanicError> {
        let (_, mut invalidated_dependencies) =
            self.data_write_impl::<true>(&group_key, txn_idx, incarnation, values, prev_tags)?;

        // We write data first, without holding the sizes lock, then write size.
        // Hence when size is observed, values should already be written.
        let mut group_sizes = self.group_sizes.get_mut(&group_key).ok_or_else(|| {
            // Currently, we rely on read-before-write to make sure the group would have
            // been initialized, which would have created an entry in group_sizes. Group
            // being initialized sets up data-structures, such as superset_tags, which
            // is used in write_v2, hence the code invariant error. Note that in read API
            // (fetch_tagged_data) we return Uninitialized / TagNotFound errors, because
            // currently that is a part of expected initialization flow.
            // TODO(BlockSTMv2): when we refactor MVHashMap and group initialization logic,
            // also revisit and address the read-before-write assumption.
            code_invariant_error("Group (sizes) must be initialized to write to")
        })?;

        // In store deps, we compute any read dependencies of txns that, based on the
        // index, would now read the same size but from the new entry created at txn_idx.
        // In other words, reads that can be kept valid, even though they were previously
        // reading an entry by a lower txn index. However, if the size has changed, then
        // those read dependencies will be added to invalidated_dependencies, and the
        // store_deps variable will be empty.
        let store_deps: BTreeMap<TxnIndex, Incarnation> = Self::get_latest_entry(
            &group_sizes.size_entries,
            txn_idx,
            ReadPosition::AfterCurrentTxn,
        )
        .map_or_else(BTreeMap::new, |(_, size_entry)| {
            let new_deps = size_entry.value.dependencies.lock().split_off(txn_idx + 1);

            if size_entry.value.size == size {
                // Validation passed.
                new_deps
            } else {
                invalidated_dependencies.extend(new_deps);
                BTreeMap::new()
            }
        });

        group_sizes.size_entries.insert(
            ShiftedTxnIndex::new(txn_idx),
            SizeEntry::new(SizeAndDependencies::from_size_and_dependencies(
                size, store_deps,
            )),
        );

        Ok(invalidated_dependencies.take())
    }

    /// Mark all entry from transaction 'txn_idx' at access path 'key' as an estimated write
    /// (for future incarnation). Will panic if the entry is not in the data-structure.
    pub fn mark_estimate(&self, group_key: &K, txn_idx: TxnIndex, tags: HashSet<&T>) {
        for tag in tags {
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
        self.remove_impl::<false>(
            group_key,
            txn_idx,
            tags.iter().collect(),
            &mut RegisteredReadDependencies::new(),
        )
        .expect("remove_impl with V1 never fails");

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

    pub fn remove_v2(
        &self,
        group_key: &K,
        txn_idx: TxnIndex,
        tags: HashSet<&T>,
    ) -> Result<BTreeMap<TxnIndex, Incarnation>, PanicError> {
        let mut invalidated_dependencies = RegisteredReadDependencies::new();
        self.remove_impl::<true>(group_key, txn_idx, tags, &mut invalidated_dependencies)?;

        let mut group_sizes = self.group_sizes.get_mut(group_key).ok_or_else(|| {
            code_invariant_error(format!(
                "Group sizes at key {:?} must exist for remove_v2",
                group_key
            ))
        })?;
        let removed_size_entry = group_sizes
            .size_entries
            .remove(&ShiftedTxnIndex::new(txn_idx))
            .ok_or_else(|| {
                code_invariant_error(format!(
                    "Group size entry at key {:?} for the txn {} must exist for remove_v2",
                    group_key, txn_idx
                ))
            })?;

        // Handle dependencies for the removed size entry.
        let mut removed_size_deps = take_dependencies(&removed_size_entry.value.dependencies);
        if let Some((_, next_lower_entry)) = Self::get_latest_entry(
            &group_sizes.size_entries,
            txn_idx,
            ReadPosition::BeforeCurrentTxn,
        ) {
            // If the entry that will be read after removal contains the same size,
            // then the dependencies on size can be registered there and not invalidated.
            // In this case, removed_size_deps gets drained.
            if next_lower_entry.value.size == removed_size_entry.value.size {
                next_lower_entry
                    .value
                    .dependencies
                    .lock()
                    .extend_with_higher_dependencies(std::mem::take(&mut removed_size_deps))?;
            }
        }

        // If removed_size_deps was not drained (into the preceding entry's dependencies),
        // then those dependencies also need to be invalidated.
        invalidated_dependencies.extend(removed_size_deps);
        Ok(invalidated_dependencies.take())
    }

    /// Read the latest value corresponding to a tag at a given group (identified by key).
    /// Return the size of the group (if requested), as defined above, alongside the version
    /// information (None if storage/pre-block version).
    /// If the layout of the resource is current UnSet, this function sets the layout of the
    /// group to the provided layout.
    pub fn fetch_tagged_data_no_record(
        &self,
        group_key: &K,
        tag: &T,
        txn_idx: TxnIndex,
    ) -> Result<(Version, ValueWithLayout<V>), MVGroupError> {
        let key_ref = GroupKeyRef { group_key, tag };

        // We are accessing group_sizes and values non-atomically, hence the order matters.
        // It is important that initialization check happens before fetch data below. O.w.
        // we could incorrectly get a TagNotFound error (do not find data, but then find
        // size initialized in between the calls). In fact, we always write size after data,
        // and sometimes (e.g. during initialization) even hold the sizes lock during writes.
        // It is fine to observe initialized = false, but find data, in convert_tagged_data.
        let initialized = self.group_sizes.contains_key(group_key);

        let data_value = self.values.fetch_data_no_record(&key_ref, txn_idx);
        self.convert_tagged_data(data_value, initialized)
    }

    // Used in BlockSTMv2, registers the read dependency on returned data.
    pub fn fetch_tagged_data_and_record_dependency(
        &self,
        group_key: &K,
        tag: &T,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
    ) -> Result<(Version, ValueWithLayout<V>), MVGroupError> {
        let key_ref = GroupKeyRef { group_key, tag };

        // We are accessing group_sizes and values non-atomically, hence the order matters.
        // It is important that initialization check happens before fetch data below. O.w.
        // we could incorrectly get a TagNotFound error (do not find data, but then find
        // size initialized in between the calls). In fact, we always write size after data,
        // and sometimes (e.g. during initialization) even hold the sizes lock during writes.
        // It is fine to observe initialized = false, but find data, in convert_tagged_data.
        // TODO(BlockSTMv2): complete overhaul of initialization logic.
        let initialized = self.group_sizes.contains_key(group_key);

        let data_value =
            self.values
                .fetch_data_and_record_dependency(&key_ref, txn_idx, incarnation);
        self.convert_tagged_data(data_value, initialized)
    }

    // Used in BlockSTMv1 w. certain heuristics for dealing with estimate flag,
    // and does not register the read dependency.
    pub fn get_group_size_no_record(
        &self,
        group_key: &K,
        txn_idx: TxnIndex,
    ) -> Result<ResourceGroupSize, MVGroupError> {
        match self.group_sizes.get(group_key) {
            Some(g) => {
                Self::get_latest_entry(&g.size_entries, txn_idx, ReadPosition::BeforeCurrentTxn)
                    .map_or(Err(MVGroupError::Uninitialized), |(idx, size)| {
                        if size.is_estimate() && g.size_has_changed {
                            Err(MVGroupError::Dependency(
                                idx.idx().expect("May not depend on storage version"),
                            ))
                        } else {
                            Ok(size.value.size)
                        }
                    })
            },
            None => Err(MVGroupError::Uninitialized),
        }
    }

    // Used in BlockSTMv2, registers the read dependency on returned size.
    pub fn get_group_size_and_record_dependency(
        &self,
        group_key: &K,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
    ) -> Result<ResourceGroupSize, MVGroupError> {
        match self.group_sizes.get(group_key) {
            Some(g) => {
                Self::get_latest_entry(&g.size_entries, txn_idx, ReadPosition::BeforeCurrentTxn)
                    .map_or(Err(MVGroupError::Uninitialized), |(_, size)| {
                        // TODO(BlockSTMv2): convert to PanicErrors after MVHashMap refactoring.
                        assert_ok!(size.value.dependencies.lock().insert(txn_idx, incarnation));
                        Ok(size.value.size)
                    })
            },
            None => Err(MVGroupError::Uninitialized),
        }
    }

    pub fn validate_group_size(
        &self,
        group_key: &K,
        txn_idx: TxnIndex,
        group_size_to_validate: ResourceGroupSize,
    ) -> bool {
        self.get_group_size_no_record(group_key, txn_idx) == Ok(group_size_to_validate)
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
                |tag| match self.fetch_tagged_data_no_record(group_key, &tag, txn_idx + 1) {
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
            self.get_group_size_no_record(group_key, txn_idx + 1)
                .map_err(|e| {
                    code_invariant_error(format!(
                        "Unexpected error in finalize group get size {:?}",
                        e
                    ))
                })?,
        ))
    }

    pub fn remove_all_at_or_after(&self, txn_idx: TxnIndex) {
        self.values.remove_all_at_or_after(txn_idx);
        for mut entry in self.group_sizes.iter_mut() {
            entry
                .value_mut()
                .size_entries
                .split_off(&ShiftedTxnIndex::new(txn_idx));
        }
    }
}

// Private methods.
impl<
        K: Hash + Clone + Debug + Eq,
        T: Hash + Clone + Debug + Eq + Serialize,
        V: TransactionWrite + PartialEq,
    > VersionedGroupData<K, T, V>
{
    /// Private utility method to find the latest entry before a given transaction index,
    /// inclusive or exclusive depending on the read position. Encapsulates the common
    /// pattern of range(..ShiftedTxnIndex::new(some index)).next_back()
    fn get_latest_entry<Entry>(
        entries: &BTreeMap<ShiftedTxnIndex, Entry>,
        txn_idx: TxnIndex,
        read_position: ReadPosition,
    ) -> Option<(&ShiftedTxnIndex, &Entry)> {
        let before_idx = match read_position {
            ReadPosition::BeforeCurrentTxn => txn_idx,
            ReadPosition::AfterCurrentTxn => txn_idx + 1,
        };
        entries
            .range(..ShiftedTxnIndex::new(before_idx))
            .next_back()
    }

    // Modifies invalidated dependencies in place via interior mutability.
    fn remove_impl<const V2: bool>(
        &self,
        group_key: &K,
        txn_idx: TxnIndex,
        tags: HashSet<&T>,
        invalidated_deps: &mut RegisteredReadDependencies,
    ) -> Result<(), PanicError> {
        for tag in tags {
            let key_ref = GroupKeyRef { group_key, tag };
            if V2 {
                invalidated_deps.extend(self.values.remove_v2::<_, false>(&key_ref, txn_idx)?);
            } else {
                self.values.remove(&key_ref, txn_idx);
            }
        }
        Ok(())
    }

    // Unified inner implementation interface for BlockSTMv1 and V2. A pair is returned,
    // where only the first element matters for V1, and the second element for V2.
    // For V1, the bool indicates whether a new tag was written as opposed to previous
    // incarnation (which necessitates certain validations).
    // For V2, the BTreeSet contains read dependencies (version of the txn that performed
    // the read) invalidated by writing the values.
    fn data_write_impl<const V2: bool>(
        &self,
        group_key: &K,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        values: impl IntoIterator<Item = (T, (V, Option<Arc<MoveTypeLayout>>))>,
        mut prev_tags: HashSet<&T>,
    ) -> Result<(bool, RegisteredReadDependencies), PanicError> {
        let mut ret_v1 = false;
        // Creating a RegisteredReadDependencies wrapper in order to do proper extending.
        let mut ret_v2 = RegisteredReadDependencies::new();
        let mut tags_to_write = vec![];

        {
            let superset_tags = self.group_tags.get(group_key).ok_or_else(|| {
                // Due to read-before-write.
                code_invariant_error("Group (tags) must be initialized to write to")
            })?;

            for (tag, (value, layout)) in values.into_iter() {
                if !superset_tags.contains(&tag) {
                    tags_to_write.push(tag.clone());
                }

                ret_v1 |= !prev_tags.remove(&tag);

                if V2 {
                    ret_v2.extend(self.values.write_v2::<false>(
                        (group_key.clone(), tag),
                        txn_idx,
                        incarnation,
                        Arc::new(value),
                        layout,
                    )?);
                } else {
                    self.values.write(
                        (group_key.clone(), tag),
                        txn_idx,
                        incarnation,
                        Arc::new(value),
                        layout,
                    );
                }
            }
        }

        if !tags_to_write.is_empty() {
            // We extend here while acquiring a write access (implicit lock), while the
            // processing above only requires a read access.
            self.group_tags
                .get_mut(group_key)
                .expect("Group must be initialized")
                .extend(tags_to_write);
        }

        self.remove_impl::<V2>(group_key, txn_idx, prev_tags, &mut ret_v2)?;

        Ok((ret_v1, ret_v2))
    }

    fn convert_tagged_data(
        &self,
        data_value: anyhow::Result<MVDataOutput<V>, MVDataError>,
        initialized: bool,
    ) -> Result<(Version, ValueWithLayout<V>), MVGroupError> {
        match data_value {
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

    // Test for dependency tracking in V2 interfaces. Notice that due to the implementation,
    // while the size of the base entry is computed at the time of the write, we can determine
    // the recorded sizes for other entries - which allows flexibility in test cases. We test
    // values and sizes separately (and keep the other from invalidating the same dependencies).
    #[test_case(true, true, false; "value test: mid value high, mid size low")]
    #[test_case(true, false, false; "value test: mid value low, mid size low")]
    #[test_case(false, false, true; "size test: mid size high, mid value low")]
    #[test_case(false, false, false; "size test: mid size low, mid value low")]
    fn test_dependency_tracking(
        test_value_or_size: bool,
        mid_value_matches_low: bool,
        mid_size_matches_low: bool,
    ) {
        // Initialize test data
        let group_key = KeyType(b"/group/test".to_vec());
        let tag: usize = 1;

        // Create values and determine which one to use for mid_value
        let base_value = TestValue::creation_with_len(1);
        let high_value = TestValue::creation_with_len(2);
        let mid_value = if mid_value_matches_low {
            base_value.clone()
        } else {
            high_value.clone()
        };

        // Calculate base_size based on the actual values (as it will be computed by set_raw_base_values).
        // Set high size arbitrary and determine mid_size.
        let one_entry_len = base_value.bytes().unwrap().len();
        let base_size = group_size_as_sum(vec![(&tag, one_entry_len)].into_iter()).unwrap();
        let high_size = ResourceGroupSize::Combined {
            num_tagged_resources: 3,
            all_tagged_resources_size: 20,
        };
        let mid_size = if mid_size_matches_low {
            base_size
        } else {
            high_size
        };

        // Fixed indices for our test
        let mid_idx: TxnIndex = 5; // Middle index for the write that may invalidate
        let high_idx: TxnIndex = 10; // High index for the first write
        let inc_1: Incarnation = 1; // Fixed incarnation for simplicity

        // Create a new VersionedGroupData instance and initialize it with base & high values.
        let group_data = VersionedGroupData::<KeyType<Vec<u8>>, usize, TestValue>::empty();
        assert_ok!(
            group_data.set_raw_base_values(group_key.clone(), vec![(tag, base_value.clone())],)
        );
        assert_ok!(group_data.write_v2(
            group_key.clone(),
            high_idx,
            inc_1,
            vec![(tag, (high_value.clone(), None))],
            high_size,
            HashSet::new(),
        ));

        // Define indices for dependencies
        let dependency_indices = [
            15, 20, // > high_idx
            6, 10, // (mid_idx, high_idx]
            2, 5, // ..=mid_idx
        ];

        let mut all_value_deps = BTreeMap::new();
        let mut all_size_deps = BTreeMap::new();
        for idx in dependency_indices {
            if test_value_or_size {
                // Create value dependency
                let res = group_data
                    .fetch_tagged_data_and_record_dependency(&group_key, &tag, idx, inc_1)
                    .unwrap();
                assert_eq!(
                    res.0,
                    if idx > high_idx {
                        Ok((high_idx, inc_1))
                    } else {
                        Err(StorageVersion)
                    }
                );
                assert_eq!(
                    res.1,
                    if idx > high_idx {
                        ValueWithLayout::Exchanged(Arc::new(high_value.clone()), None)
                    } else {
                        ValueWithLayout::RawFromStorage(Arc::new(base_value.clone()))
                    }
                );
                all_value_deps.insert(idx, inc_1);
            } else {
                // Create size dependency
                // Create size dependency
                assert_ok!(group_data.get_group_size_and_record_dependency(&group_key, idx, inc_1));
                all_size_deps.insert(idx, inc_1);
            }
        }

        // Make sure the base value actually matches based on layout / Exchanged.
        group_data.update_tagged_base_value_with_layout(
            group_key.clone(),
            tag,
            base_value.clone(),
            None,
        );
        assert_eq!(
            group_data
                .fetch_tagged_data_and_record_dependency(&group_key, &tag, 2, 1)
                .unwrap(),
            (
                Err(StorageVersion),
                ValueWithLayout::Exchanged(Arc::new(base_value.clone()), None)
            )
        );

        // Write another value at a middle index and check dependency handling.
        let write_invalidated_deps = group_data
            .write_v2(
                group_key.clone(),
                mid_idx,
                inc_1,
                vec![(tag, (mid_value.clone(), None))],
                mid_size,
                HashSet::new(),
            )
            .unwrap();
        let expected_invalidated = all_value_deps
            .clone()
            .into_iter()
            .filter(|&(idx, _)| idx > mid_idx && idx <= high_idx && !mid_value_matches_low)
            .chain(
                all_size_deps
                    .clone()
                    .into_iter()
                    .filter(|&(idx, _)| idx > mid_idx && idx <= high_idx && !mid_size_matches_low),
            )
            .collect::<BTreeMap<_, _>>();
        assert_eq!(write_invalidated_deps, expected_invalidated);
        // Remove the high index entry and check dependency handling
        let remove_invalidated_deps = group_data
            .remove_v2(&group_key, high_idx, HashSet::from([&tag]))
            .unwrap();
        let expected_invalidated = all_value_deps
            .into_iter()
            // matching low value means not matching high in the test
            .filter(|&(idx, _)| idx > high_idx && mid_value_matches_low)
            .chain(
                all_size_deps
                    .clone()
                    .into_iter()
                    .filter(|&(idx, _)| idx > high_idx && mid_size_matches_low),
            )
            .collect::<BTreeMap<_, _>>();
        assert_eq!(remove_invalidated_deps, expected_invalidated);

        // Verify stored size dependencies in the data structure
        let group_sizes = group_data.group_sizes.get(&group_key).unwrap();
        {
            let mid_deps = &group_sizes
                .size_entries
                .get(&ShiftedTxnIndex::new(mid_idx))
                .unwrap()
                .value
                .dependencies;
            assert_eq!(
                take_dependencies(mid_deps),
                all_size_deps
                    .clone()
                    .into_iter()
                    .filter(
                        |&(idx, _)| (idx > mid_idx && idx <= high_idx && mid_size_matches_low)
                            || (idx > high_idx && !mid_size_matches_low)
                    )
                    .collect::<BTreeMap<_, _>>()
            );
        }
        {
            let base_deps = &group_sizes
                .size_entries
                .get(&ShiftedTxnIndex::zero_idx())
                .unwrap()
                .value
                .dependencies;
            assert_eq!(
                take_dependencies(base_deps),
                all_size_deps
                    .into_iter()
                    .filter(|&(idx, _)| idx <= mid_idx)
                    .collect::<BTreeMap<_, _>>()
            );
        }

        // Verify we can access the value and size from mid write
        let (_, value) = group_data
            .fetch_tagged_data_and_record_dependency(
                &group_key, &tag, 21, inc_1, // higher than any idx.
            )
            .unwrap();
        assert_eq!(value, ValueWithLayout::Exchanged(Arc::new(mid_value), None));
        let size = group_data
            .get_group_size_and_record_dependency(&group_key, 21, inc_1)
            .unwrap();
        assert_eq!(size, mid_size);
    }

    // Entries from storage may contain a special layout, which has not been processed yet.
    // It has to be validated (and fail) against a processed (Exchanged) layout (e.g. with
    // IDs for delayed fields contained within). The other case checks that if the correct
    // layout was provided and set (for transaction 0, the previous test instead provides
    // layout for storage version), then it passes validation.
    #[test_case(true; "raw storage layout fails validation")]
    #[test_case(false; "exchanged layout passes validation")]
    fn test_raw_storage_layout_validation(raw_storage_layout: bool) {
        let group_key = KeyType(b"/group/test".to_vec());
        let tag: usize = 1;

        let group_data = VersionedGroupData::<KeyType<Vec<u8>>, usize, TestValue>::empty();
        let base_value = TestValue::creation_with_len(1);
        let one_entry_len = base_value.bytes().unwrap().len();
        let base_size = group_size_as_sum(vec![(&tag, one_entry_len)].into_iter()).unwrap();

        assert_ok!(
            group_data.set_raw_base_values(group_key.clone(), vec![(tag, base_value.clone())])
        );
        if !raw_storage_layout {
            assert_ok!(group_data.write_v2(
                group_key.clone(),
                0,
                1,
                vec![(tag, (base_value.clone(), None))],
                base_size,
                HashSet::new(),
            ));
        }

        let (version, value) = group_data
            .fetch_tagged_data_and_record_dependency(&group_key, &tag, 5, 2)
            .unwrap();
        assert_eq!(
            version,
            if raw_storage_layout {
                Err(StorageVersion)
            } else {
                Ok((0, 1))
            }
        );
        assert_eq!(
            value,
            if raw_storage_layout {
                ValueWithLayout::RawFromStorage(Arc::new(base_value.clone()))
            } else {
                ValueWithLayout::Exchanged(Arc::new(base_value.clone()), None)
            }
        );

        let invalidated_deps = group_data
            .write_v2(
                group_key.clone(),
                2,
                1,
                vec![(tag, (base_value.clone(), None))],
                base_size,
                HashSet::new(),
            )
            .unwrap();
        assert_eq!(
            invalidated_deps,
            if raw_storage_layout {
                BTreeMap::from([(5, 2)])
            } else {
                BTreeMap::new()
            }
        );
    }

    // This tests the case when after a write and remove, some entries in the group may pass
    // validation while others fail. The test ensures the proper dependencies are invalidated
    // and returned. The test setup looks as follows:
    //
    //                    Group Contents:
    //                    TAG 0       TAG 1
    //
    //  txn 10             depB0      depB1
    //  txn 9                B          B
    //                       |          |
    //  txn 8              depA0      depA1
    //  txn 7   test writes (A, B) or (B, A)
    //                       |          |
    //  txn 6                A          A
    //
    //  We ensure that if (A, B) is written, depA1 is invalidated, and vice versa. Then, a
    //  similar test considers the removal of txn 9's output.
    //
    // A != B for validation purposes, but the second parameter determines whether the
    // values are different, or whether the layouts are set for both A and B (in which
    // case validation fails instead of performing a deep layout comparison).
    #[test_case(true, true; "partial: A, B, different values")]
    #[test_case(true, false; "partial: A, B, set layouts")]
    #[test_case(false, true; "partial: B, A, different values")]
    #[test_case(false, false; "partial: B, A, set layouts")]
    fn test_partial_invalidation(case_a_b: bool, different_values: bool) {
        let group_key = KeyType(b"/group/test".to_vec());
        let tag0: usize = 0;
        let tag1: usize = 1;

        let group_data = VersionedGroupData::<KeyType<Vec<u8>>, usize, TestValue>::empty();

        // Create base values A and B
        let value_a = TestValue::creation_with_len(1);
        let value_b = TestValue::creation_with_len(2);
        let invariant_size = ResourceGroupSize::Combined {
            num_tagged_resources: 3,
            all_tagged_resources_size: 20,
        };

        // Create pairs based on different_values parameter
        // When different_values=true: use (value, None) so validation is based on value differences
        // When different_values=false: use same value with (value, Some(layout)) so validation fails due to layout comparison being avoided
        let pair_a = if different_values {
            (value_a.clone(), None)
        } else {
            (value_a.clone(), Some(Arc::new(MoveTypeLayout::Bool)))
        };
        let pair_b = if different_values {
            (value_b.clone(), None)
        } else {
            (value_a.clone(), Some(Arc::new(MoveTypeLayout::Bool))) // Same value as pair_a when different_values=false
        };

        // Need to initialize the group - detected by missing size entry.
        assert_ok!(group_data.set_raw_base_values(group_key.clone(), vec![]));

        // Initial setup: Write A at txn 6. Register dependencies for both tags at txn 8.
        assert_ok!(group_data.write_v2(
            group_key.clone(),
            6,
            1,
            vec![(tag0, pair_a.clone()), (tag1, pair_a.clone())],
            invariant_size,
            HashSet::new(),
        ));
        assert_eq!(
            group_data
                .fetch_tagged_data_and_record_dependency(&group_key, &tag0, 8, 0)
                .unwrap()
                .0,
            Ok((6, 1))
        );
        assert_eq!(
            group_data
                .fetch_tagged_data_and_record_dependency(&group_key, &tag1, 8, 1)
                .unwrap()
                .0,
            Ok((6, 1))
        );

        // Write B at txn 9 & register dependencies at txn 10. Encode tag in incarnation.
        assert_ok!(group_data.write_v2(
            group_key.clone(),
            9,
            1,
            vec![(tag0, pair_b.clone()), (tag1, pair_b.clone())],
            invariant_size,
            HashSet::new(),
        ));
        assert_eq!(
            group_data
                .fetch_tagged_data_and_record_dependency(&group_key, &tag0, 10, 0)
                .unwrap()
                .0,
            Ok((9, 1))
        );
        assert_eq!(
            group_data
                .fetch_tagged_data_and_record_dependency(&group_key, &tag1, 10, 1)
                .unwrap()
                .0,
            Ok((9, 1))
        );

        // Test write at txn 7 based on case_a_b
        let write_value = if case_a_b {
            vec![(tag0, pair_a.clone()), (tag1, pair_b.clone())]
        } else {
            vec![(tag0, pair_b.clone()), (tag1, pair_a.clone())]
        };

        let write_invalidated = group_data
            .write_v2(
                group_key.clone(),
                7,
                1,
                write_value,
                invariant_size,
                HashSet::new(),
            )
            .unwrap();
        let expected_invalidated = if different_values {
            if case_a_b {
                // If writing (A, B), depA1 should be invalidated (incarnation = tag)
                BTreeMap::from([(8, 1)])
            } else {
                // If writing (B, A), depA0 should be invalidated
                BTreeMap::from([(8, 0)])
            }
        } else {
            // When different_values=false, both A and B are the same value with layouts set,
            // so validation fails for both and both dependencies should be invalidated
            BTreeMap::from([(8, 0), (8, 1)])
        };
        assert_eq!(write_invalidated, expected_invalidated);

        // Remove txn 9's output
        let remove_invalidated = group_data
            .remove_v2(&group_key, 9, HashSet::from([&tag0, &tag1]))
            .unwrap();
        let expected_invalidated = if different_values {
            if case_a_b {
                BTreeMap::from([(10, 0)])
            } else {
                BTreeMap::from([(10, 1)])
            }
        } else {
            // When different_values=false, both A and B are the same value with layouts set,
            // so validation fails for both and both dependencies should be invalidated
            BTreeMap::from([(10, 0), (10, 1)])
        };
        assert_eq!(remove_invalidated, expected_invalidated);
    }

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
                    .value
                    .size,
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
        assert_matches!(
            map.get_group_size_no_record(&ap, 3),
            Err(MVGroupError::Uninitialized)
        );
        assert_matches!(
            map.fetch_tagged_data_no_record(&ap, &1, 3),
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
        assert_ok_eq!(map.get_group_size_no_record(&ap, 4), idx_3_size);
        assert_ok_eq!(
            map.get_group_size_no_record(&ap, 3),
            ResourceGroupSize::zero_combined()
        );

        // Check values.
        assert_matches!(
            map.fetch_tagged_data_no_record(&ap, &1, 3),
            Err(MVGroupError::TagNotFound)
        );
        assert_matches!(
            map.fetch_tagged_data_no_record(&ap, &3, 4),
            Err(MVGroupError::TagNotFound)
        );
        // ... but idx = 4 should find the previously stored value.
        assert_eq!(
            map.fetch_tagged_data_no_record(&ap, &1, 4).unwrap(),
            (
                Ok((3, 1)),
                ValueWithLayout::Exchanged(Arc::new(TestValue::creation_with_len(1)), None)
            )
        );

        // ap_empty should still be uninitialized.
        assert_matches!(
            map.fetch_tagged_data_no_record(&ap_empty, &1, 3),
            Err(MVGroupError::Uninitialized)
        );
        assert_matches!(
            map.get_group_size_no_record(&ap_empty, 3),
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
            map.fetch_tagged_data_no_record(&ap, &2, 4),
            Err(MVGroupError::TagNotFound)
        );
        assert_matches!(
            map.fetch_tagged_data_no_record(&ap, &3, 5),
            Err(MVGroupError::TagNotFound)
        );
        assert_eq!(
            map.fetch_tagged_data_no_record(&ap, &2, 5).unwrap(),
            (
                Ok((4, 0)),
                ValueWithLayout::Exchanged(Arc::new(TestValue::creation_with_len(4)), None)
            )
        );
        assert_eq!(
            map.fetch_tagged_data_no_record(&ap, &1, 4).unwrap(),
            (
                Err(StorageVersion),
                ValueWithLayout::RawFromStorage(Arc::new(TestValue::creation_with_len(2)))
            )
        );
        assert_eq!(
            map.fetch_tagged_data_no_record(&ap, &0, 6).unwrap(),
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
            map.fetch_tagged_data_no_record(&ap, &1, 12).unwrap(),
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
            map.fetch_tagged_data_no_record(&ap, &1, 12).unwrap(),
            (
                Ok((10, 1)),
                ValueWithLayout::Exchanged(Arc::new(TestValue::new(vec![10, 1])), None)
            )
        );

        let tags_012: Vec<usize> = (1..3).collect();
        map.mark_estimate(&ap, 10, tags_012.iter().collect());
        assert_matches!(
            map.fetch_tagged_data_no_record(&ap, &1, 12),
            Err(Dependency(10))
        );
        assert_matches!(
            map.fetch_tagged_data_no_record(&ap, &2, 12),
            Err(Dependency(10))
        );
        assert_matches!(
            map.fetch_tagged_data_no_record(&ap, &3, 12),
            Err(TagNotFound)
        );
        assert_eq!(
            map.fetch_tagged_data_no_record(&ap, &0, 12).unwrap(),
            (
                Ok((5, 3)),
                ValueWithLayout::Exchanged(Arc::new(TestValue::new(vec![5, 3])), None)
            )
        );
        assert_matches!(map.get_group_size_no_record(&ap, 12), Err(Dependency(10)));

        map.remove(&ap, 10, (1..3).collect());
        assert_eq!(
            map.fetch_tagged_data_no_record(&ap, &0, 12).unwrap(),
            (
                Ok((5, 3)),
                ValueWithLayout::Exchanged(Arc::new(TestValue::new(vec![5, 3])), None)
            )
        );
        assert_eq!(
            map.fetch_tagged_data_no_record(&ap, &1, 12).unwrap(),
            (
                Ok((5, 3)),
                ValueWithLayout::Exchanged(Arc::new(TestValue::new(vec![5, 3])), None)
            )
        );

        // Size should also be removed at 10.
        assert_ok_eq!(map.get_group_size_no_record(&ap, 12), idx_5_size);
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

        assert_ok_eq!(map.get_group_size_no_record(&ap, 5), base_size);
        assert!(map.validate_group_size(&ap, 4, base_size));
        assert!(!map.validate_group_size(&ap, 5, idx_5_size));
        assert_ok_eq!(map.get_group_size_no_record(&ap, 6), idx_5_size);

        // Despite estimates, should still return size.
        let tags_01: Vec<usize> = (0..2).collect();
        map.mark_estimate(&ap, 5, tags_01.iter().collect());
        assert_ok_eq!(map.get_group_size_no_record(&ap, 12), idx_5_size);
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
        map.mark_estimate(&ap, 5, tags_01.iter().collect());
        assert_ok_eq!(map.get_group_size_no_record(&ap, 12), idx_5_size);
        assert!(map.validate_group_size(&ap, 12, idx_5_size));
        assert!(!map.validate_group_size(&ap, 12, ResourceGroupSize::zero_concrete()));

        // Remove currently does not affect size_has_changed.
        map.remove(&ap, 5, (0..3).collect());
        assert!(!map.group_sizes.get(&ap).unwrap().size_has_changed);
        assert_ok_eq!(map.get_group_size_no_record(&ap, 4), base_size);
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
        assert_ok_eq!(map.get_group_size_no_record(&ap, 10), idx_5_size_with_ones);
        assert!(map.validate_group_size(&ap, 10, idx_5_size_with_ones));
        assert!(!map.validate_group_size(&ap, 10, idx_5_size));
        assert_ok_eq!(map.get_group_size_no_record(&ap, 3), base_size);

        let tags_012: Vec<usize> = (0..3).collect();
        map.mark_estimate(&ap, 5, tags_012.iter().collect());
        assert_matches!(
            map.get_group_size_no_record(&ap, 12),
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
        assert_ok_eq!(map.get_group_size_no_record(&ap, 3), base_size,);

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
            map.fetch_tagged_data_no_record(&ap, &1, 6).unwrap(),
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
            map.fetch_tagged_data_no_record(&ap, &1, 6).unwrap(),
            (
                Err(StorageVersion),
                ValueWithLayout::Exchanged(Arc::new(TestValue::creation_with_len(1)), None)
            )
        );
    }

    #[test]
    fn group_key_ref_equivalence_and_hashing() {
        use std::hash::{DefaultHasher, Hash, Hasher};

        let dashmap: DashMap<(u32, u32), String> = DashMap::new();

        // Test with a range of keys and tags (1..50 x 1..50 = 2500 combinations)
        for k in 1u32..50u32 {
            for t in 1u32..50u32 {
                let tuple_key = (k, t);
                let ref_key = GroupKeyRef {
                    group_key: &k,
                    tag: &t,
                };
                let expected_value = format!("value_{}_{}", k, t);

                // Test 1: Verify that (K, T) and GroupKeyRef hash to the same value
                let mut hasher1 = DefaultHasher::new();
                tuple_key.hash(&mut hasher1);
                let tuple_hash = hasher1.finish();

                let mut hasher2 = DefaultHasher::new();
                ref_key.hash(&mut hasher2);
                let ref_hash = hasher2.finish();

                assert_eq!(
                    tuple_hash, ref_hash,
                    "Tuple ({}, {}) and GroupKeyRef should hash to the same value",
                    k, t
                );

                // Test 2: Test equivalence trait directly
                assert!(
                    ref_key.equivalent(&tuple_key),
                    "GroupKeyRef should be equivalent to corresponding tuple ({}, {})",
                    k,
                    t
                );
                // Test with different values to ensure non-equivalence
                let different_tuple = (k, t + 1000);
                assert!(
                    !ref_key.equivalent(&different_tuple),
                    "GroupKeyRef should not be equivalent to different tuple ({}, {})",
                    k,
                    t + 1000
                );

                // Test 3: Insert using tuple key
                dashmap.insert(tuple_key, expected_value.clone());

                // Test 4: Access using GroupKeyRef - should find the same entry
                let retrieved = dashmap.get(&ref_key);
                assert!(
                    retrieved.is_some(),
                    "Should be able to access entry ({}, {}) using GroupKeyRef",
                    k,
                    t
                );
                assert_eq!(
                    retrieved.unwrap().as_str(),
                    expected_value,
                    "Retrieved value should match expected value for ({}, {})",
                    k,
                    t
                );

                // Test 5: Remove using GroupKeyRef and verify it's the correct entry
                let removed = dashmap.remove(&ref_key);
                assert!(
                    removed.is_some(),
                    "Should be able to remove entry ({}, {}) using GroupKeyRef",
                    k,
                    t
                );
                let (removed_key, removed_value) = removed.unwrap();
                assert_eq!(
                    removed_key, tuple_key,
                    "Removed key should match original tuple key ({}, {})",
                    k, t
                );
                assert_eq!(
                    removed_value, expected_value,
                    "Removed value should match expected value for ({}, {})",
                    k, t
                );

                // Verify entry is actually removed
                assert!(
                    dashmap.get(&ref_key).is_none(),
                    "Entry ({}, {}) should be removed and not accessible",
                    k,
                    t
                );
                assert!(
                    dashmap.get(&tuple_key).is_none(),
                    "Entry ({}, {}) should be removed and not accessible via tuple key",
                    k,
                    t
                );
            }
        }

        // Verify all entries are removed
        assert_eq!(dashmap.len(), 0, "All entries should be removed");
    }
}
