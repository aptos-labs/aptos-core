// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use aptos_mvhashmap::{
    types::{MVDataError, MVDataOutput, TxnIndex, Version},
    versioned_data::VersionedData,
    versioned_group_data::VersionedGroupData,
};
use aptos_types::{
    state_store::state_value::StateValueMetadataKind, transaction::BlockExecutableTransaction,
    write_set::TransactionWrite,
};
use derivative::Derivative;
use std::{
    collections::{
        hash_map::{
            Entry,
            Entry::{Occupied, Vacant},
        },
        HashMap,
    },
    sync::Arc,
};

/// The enum variants should not be re-ordered, as it defines a relation
/// Existence < Metadata < Value.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ReadKind {
    Exists,
    Metadata,
    Value,
}

/// The enum captures the state that the transaction execution extracted from
/// a read callback to block executor, in order to be validated by Block-STM.
/// The captured state is fine-grained, e.g. it distinguishes between reading
/// a full value, and other kinds of reads that may access only the metadata
/// information, or check whether data exists at a given key.
#[derive(Derivative)]
#[derivative(Clone(bound = ""), Debug(bound = ""), PartialEq(bound = ""))]
pub(crate) enum DataRead<V> {
    // Version supercedes V comparison.
    Versioned(
        Version,
        // Currently, we are conservative and check the version for equality
        // (version implies value equality, but not vice versa). TODO: when
        // comparing the instances of V is cheaper, compare those instead.
        #[derivative(PartialEq = "ignore", Debug = "ignore")] Arc<V>,
    ),
    Metadata(Option<StateValueMetadataKind>),
    Exists(bool),
    /// Read resolved an aggregatorV1 delta to a value. TODO: deprecate.
    Resolved(u128),
}

// Represents the result of comparing DataReads ('self' and 'other').
#[derive(Debug)]
enum DataReadComparison {
    // Information in 'self' DataRead contains information about the kind of the
    // 'other' DataRead, and is consistent with 'other'.
    Contains,
    // Information in 'self' DataRead contains information about the kind of the
    // 'other' DataRead, but is inconsistent with 'other'.
    Inconsistent,
    // All information about the kind of 'other' is not contained in 'self' kind.
    // For example, exists does not provide enough information about metadata.
    Insufficient,
}

impl<V: TransactionWrite> DataRead<V> {
    // Assigns highest rank to Versioned / Resolved, then Metadata, then Exists.
    // (e.g. versioned read implies metadata and existence information, and
    // metadata information implies existence information).
    fn get_kind(&self) -> ReadKind {
        use DataRead::*;
        match self {
            Versioned(_, _) | Resolved(_) => ReadKind::Value,
            Metadata(_) => ReadKind::Metadata,
            Exists(_) => ReadKind::Exists,
        }
    }

    // A convenience method, since the same key can be read in different modes, producing
    // different DataRead / ReadKinds. Returns true if self has >= kind than other, i.e.
    // contains more or equal information, and is consistent with the information in other.
    fn contains(&self, other: &DataRead<V>) -> DataReadComparison {
        let self_kind = self.get_kind();
        let other_kind = other.get_kind();

        if self_kind < other_kind {
            DataReadComparison::Insufficient
        } else {
            let downcast_eq = if self_kind == other_kind {
                // Optimization to avoid unnecessary clones (e.g. during validation).
                self == other
            } else {
                self.downcast(other_kind)
                    .expect("Downcast to lower kind must succeed")
                    == *other
            };

            if downcast_eq {
                DataReadComparison::Contains
            } else {
                DataReadComparison::Inconsistent
            }
        }
    }

    /// If the reads contains sufficient information, extract this information and generate
    /// a new DataRead of the desired kind (e.g. Metadata kind from Value).
    pub(crate) fn downcast(&self, kind: ReadKind) -> Option<DataRead<V>> {
        let self_kind = self.get_kind();
        if self_kind == kind {
            return Some(self.clone());
        }

        (self_kind > kind).then(|| match (self, &kind) {
            (DataRead::Versioned(_, v), ReadKind::Metadata) => {
                // For deletion, as_state_value_metadata returns None, also asserted by tests.
                DataRead::Metadata(v.as_state_value_metadata())
            },
            (DataRead::Versioned(_, v), ReadKind::Exists) => DataRead::Exists(!v.is_deletion()),
            (DataRead::Resolved(_), ReadKind::Metadata) => DataRead::Metadata(Some(None)),
            (DataRead::Resolved(_), ReadKind::Exists) => DataRead::Exists(true),
            (DataRead::Metadata(maybe_metadata), ReadKind::Exists) => {
                DataRead::Exists(maybe_metadata.is_some())
            },
            (_, _) => unreachable!("{:?}, {:?} must be covered", self_kind, kind),
        })
    }
}

/// Additional state regarding groups that may be provided to the VM during transaction
/// execution and is captured. There may be a DataRead per tag within the group, and also
/// the group size (also computed based on speculative information in MVHashMap).
#[derive(Derivative)]
#[derivative(Default(bound = ""))]
struct GroupRead<T: BlockExecutableTransaction> {
    /// The size of the resource group can be read (used for gas charging).
    speculative_size: Option<u64>,
    /// Reads to individual resources in the group, keyed by a tag.
    inner_reads: HashMap<T::Tag, DataRead<T::Value>>,
}

/// Serves as a "read-set" of a transaction execution, and provides APIs for capturing reads,
/// resolving new reads based on already captured reads when possible, and for validation.
///
/// The intended use is that all reads should be attempted to be resolved from CapturedReads.
/// If not possible, then after proper resolution from MVHashMap/storage, they should be
/// captured. This enforces an invariant that 'capture_read' will never be called with a
/// read that has a kind <= already captured read (for that key / tag).
#[derive(Derivative)]
#[derivative(Default(bound = "", new = "true"))]
pub(crate) struct CapturedReads<T: BlockExecutableTransaction> {
    data_reads: HashMap<T::Key, DataRead<T::Value>>,
    group_reads: HashMap<T::Key, GroupRead<T>>,
    // Currently, we record paths for triggering module R/W fallback.
    // TODO: implement a general functionality once the fallback is removed.
    pub(crate) module_reads: Vec<T::Key>,

    /// If there is a speculative failure (e.g. delta application failure, or an
    /// observed inconsistency), the transaction output is irrelevant (must be
    /// discarded and transaction re-executed). We have a global flag, as which
    /// read observed the inconsistency is irrelevant (moreover, typically,
    /// an error is returned to the VM to wrap up the ongoing execution).
    speculative_failure: bool,
    /// Set if the invarint on CapturedReads intended use is violated. Leads to an alert
    /// and sequential execution fallback.
    incorrect_use: bool,
}

#[derive(Debug)]
enum UpdateResult {
    Inserted,
    Updated,
    IncorrectUse(String),
    Inconsistency(String),
}

impl<T: BlockExecutableTransaction> CapturedReads<T> {
    // Given a hashmap entry for a key, incorporate a new DataRead. This checks
    // consistency and ensures that the most comprehensive read is recorded.
    fn update_entry<K, V: TransactionWrite>(
        entry: Entry<K, DataRead<V>>,
        read: DataRead<V>,
    ) -> UpdateResult {
        match entry {
            Vacant(e) => {
                e.insert(read);
                UpdateResult::Inserted
            },
            Occupied(mut e) => {
                let existing_read = e.get_mut();
                if read.get_kind() <= existing_read.get_kind() {
                    UpdateResult::IncorrectUse(format!(
                        "Incorrect use CaptureReads, read {:?}, existing {:?}",
                        read, existing_read
                    ))
                } else {
                    match read.contains(existing_read) {
                        DataReadComparison::Contains => {
                            *existing_read = read;
                            UpdateResult::Updated
                        },
                        DataReadComparison::Inconsistent => UpdateResult::Inconsistency(format!(
                            "Read {:?} must be consistent with the already stored read {:?}",
                            read, existing_read
                        )),
                        DataReadComparison::Insufficient => unreachable!(
                            "{:?} Insufficient for {:?}, but has higher kind",
                            read, existing_read
                        ),
                    }
                }
            },
        }
    }

    #[allow(dead_code)]
    pub(crate) fn capture_group_size(
        &mut self,
        _state_key: T::Key,
        _group_size: u64,
    ) -> anyhow::Result<()> {
        unimplemented!("Group size capturing not implemented");
    }

    #[allow(dead_code)]
    pub(crate) fn group_size(&self, state_key: &T::Key) -> Option<u64> {
        self.group_reads
            .get(state_key)
            .and_then(|group| group.speculative_size)
    }

    // Error means there was a inconsistency in information read (must be due to the
    // speculative nature of reads).
    pub(crate) fn capture_read(
        &mut self,
        state_key: T::Key,
        maybe_tag: Option<T::Tag>,
        read: DataRead<T::Value>,
    ) -> anyhow::Result<()> {
        let ret = match maybe_tag {
            Some(tag) => {
                let group = self.group_reads.entry(state_key).or_default();
                Self::update_entry(group.inner_reads.entry(tag), read)
            },
            None => Self::update_entry(self.data_reads.entry(state_key), read),
        };

        match ret {
            UpdateResult::IncorrectUse(m) => {
                self.incorrect_use = true;
                bail!(m);
            },
            UpdateResult::Inconsistency(m) => {
                // Record speculative failure.
                self.speculative_failure = true;
                bail!(m);
            },
            UpdateResult::Updated | UpdateResult::Inserted => Ok(()),
        }
    }

    // If maybe_tag is provided, then we check the group, otherwise, normal reads.
    pub(crate) fn get_by_kind(
        &self,
        state_key: &T::Key,
        maybe_tag: Option<&T::Tag>,
        kind: ReadKind,
    ) -> Option<DataRead<T::Value>> {
        assert!(
            kind != ReadKind::Metadata || maybe_tag.is_none(),
            "May not request metadata of a group member"
        );

        match maybe_tag {
            Some(tag) => self
                .group_reads
                .get(state_key)
                .and_then(|group| group.inner_reads.get(tag).and_then(|r| r.downcast(kind))),
            None => self
                .data_reads
                .get(state_key)
                .and_then(|r| r.downcast(kind)),
        }
    }

    pub(crate) fn validate_data_reads(
        &self,
        data_map: &VersionedData<T::Key, T::Value>,
        idx_to_validate: TxnIndex,
    ) -> bool {
        if self.speculative_failure {
            return false;
        }

        use MVDataError::*;
        use MVDataOutput::*;
        self.data_reads.iter().all(|(k, r)| {
            match data_map.fetch_data(k, idx_to_validate) {
                Ok(Versioned(version, v)) => {
                    matches!(
                        DataRead::Versioned(version, v).contains(r),
                        DataReadComparison::Contains
                    )
                },
                Ok(Resolved(value)) => matches!(
                    DataRead::Resolved(value).contains(r),
                    DataReadComparison::Contains
                ),
                // Dependency implies a validation failure, and if the original read were to
                // observe an unresolved delta, it would set the aggregator base value in the
                // multi-versioned data-structure, resolve, and record the resolved value.
                Err(Dependency(_))
                | Err(Unresolved(_))
                | Err(DeltaApplicationFailure)
                | Err(Uninitialized) => false,
            }
        })
    }

    pub(crate) fn validate_group_reads(
        &self,
        group_map: &VersionedGroupData<T::Key, T::Tag, T::Value>,
        idx_to_validate: TxnIndex,
    ) -> bool {
        if self.speculative_failure {
            return false;
        }

        self.group_reads.iter().all(|(key, group)| {
            let mut ret = true;
            if let Some(size) = group.speculative_size {
                ret &= Ok(size) == group_map.get_group_size(key, idx_to_validate);
            }

            ret && group.inner_reads.iter().all(|(tag, r)| {
                group_map
                    .read_from_group(key, tag, idx_to_validate)
                    .is_ok_and(|(version, v)| {
                        matches!(
                            DataRead::Versioned(version, v).contains(r),
                            DataReadComparison::Contains
                        )
                    })
            })
        })
    }

    pub(crate) fn mark_failure(&mut self) {
        self.speculative_failure = true;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::proptest_types::types::{KeyType, MockEvent, ValueType};
    use aptos_mvhashmap::types::StorageVersion;
    use aptos_types::{
        on_chain_config::CurrentTimeMicroseconds, state_store::state_value::StateValueMetadata,
        transaction::BlockExecutableTransaction,
    };
    use claims::{assert_err, assert_gt, assert_matches, assert_none, assert_ok, assert_some_eq};
    use test_case::test_case;

    #[test]
    fn data_read_kind() {
        // Test the strict ordering of enum variants for the read kinds.
        assert_gt!(ReadKind::Value, ReadKind::Metadata);
        assert_gt!(ReadKind::Metadata, ReadKind::Exists);

        // Test that get_kind returns the proper kind for data read instances.

        assert_eq!(
            DataRead::Versioned(
                Err(StorageVersion),
                Arc::new(ValueType::with_len_and_metadata(1, None))
            )
            .get_kind(),
            ReadKind::Value
        );
        assert_eq!(
            DataRead::Resolved::<ValueType>(200).get_kind(),
            ReadKind::Value
        );
        assert_eq!(
            DataRead::Metadata::<ValueType>(Some(None)).get_kind(),
            ReadKind::Metadata
        );
        assert_eq!(
            DataRead::Metadata::<ValueType>(None).get_kind(),
            ReadKind::Metadata
        );
        assert_eq!(
            DataRead::Exists::<ValueType>(true).get_kind(),
            ReadKind::Exists
        );
        assert_eq!(
            DataRead::Exists::<ValueType>(false).get_kind(),
            ReadKind::Exists
        );
    }

    macro_rules! assert_inconsistent_same_kind {
        ($x:expr, $y:expr) => {{
            assert_ne!($x, $y);
            assert_ne!($y, $x);
            assert_matches!($x.contains(&$y), DataReadComparison::Inconsistent);
            assert_matches!($y.contains(&$x), DataReadComparison::Inconsistent);
        }};
    }

    macro_rules! assert_inconsistent_downcast {
        ($x:expr, $y:expr) => {{
            assert_ne!($x, $y);
            assert_ne!($y, $x);
            assert_matches!($x.contains(&$y), DataReadComparison::Inconsistent);
            assert_matches!($y.contains(&$x), DataReadComparison::Insufficient);
        }};
    }

    macro_rules! assert_contains {
        ($x:expr, $y:expr) => {{
            assert_some_eq!($x.downcast($y.get_kind()), $y);
            assert_matches!($x.contains(&$y), DataReadComparison::Contains);
        }};
    }

    macro_rules! assert_insufficient {
        ($x:expr, $y:expr) => {{
            assert_none!($x.downcast($y.get_kind()));
            assert_matches!($x.contains(&$y), DataReadComparison::Insufficient);
        }};
    }

    #[test]
    fn as_contained_kind() {
        // Legacy state values do not have metadata.
        let versioned_legacy = DataRead::Versioned(
            Err(StorageVersion),
            Arc::new(ValueType::with_len_and_metadata(1, None)),
        );
        let versioned_deletion = DataRead::Versioned(
            Ok((5, 1)),
            Arc::new(ValueType::with_len_and_metadata(0, None)),
        );
        let versioned_with_metadata = DataRead::Versioned(
            Ok((7, 0)),
            Arc::new(ValueType::with_len_and_metadata(2, raw_metadata())),
        );
        let resolved = DataRead::Resolved::<ValueType>(200);
        let deletion_metadata = DataRead::Metadata(None);
        let legacy_metadata = DataRead::Metadata(Some(None));
        let metadata = DataRead::Metadata(Some(raw_metadata()));
        let exists = DataRead::Exists(true);
        let not_exists = DataRead::Exists(false);

        // Test contains & downcast.
        assert_contains!(versioned_legacy, legacy_metadata);
        assert_contains!(resolved, legacy_metadata);
        assert_contains!(versioned_legacy, exists);
        assert_contains!(resolved, exists);
        assert_contains!(legacy_metadata, exists);
        // Same checks for deletion (Resolved cannot be a deletion).
        assert_contains!(versioned_deletion, deletion_metadata);
        assert_contains!(versioned_deletion, not_exists);
        assert_contains!(deletion_metadata, not_exists);
        // Same checks with real metadata.
        assert_contains!(versioned_with_metadata, metadata);
        assert_contains!(versioned_with_metadata, exists);
        assert_contains!(metadata, exists);

        // Test upcast.
        assert_insufficient!(legacy_metadata, versioned_legacy);
        assert_insufficient!(deletion_metadata, versioned_legacy);
        assert_insufficient!(exists, versioned_legacy);
        assert_insufficient!(not_exists, versioned_legacy);
        assert_insufficient!(exists, legacy_metadata);
        assert_insufficient!(not_exists, legacy_metadata);

        // Test inconsistency at the same kind.
        assert_inconsistent_same_kind!(exists, not_exists);
        assert_inconsistent_same_kind!(deletion_metadata, legacy_metadata);
        assert_inconsistent_same_kind!(deletion_metadata, metadata);
        assert_inconsistent_same_kind!(legacy_metadata, metadata);
        assert_inconsistent_same_kind!(versioned_legacy, versioned_with_metadata);
        assert_inconsistent_same_kind!(versioned_legacy, versioned_deletion);
        assert_inconsistent_same_kind!(versioned_legacy, resolved);
        assert_inconsistent_same_kind!(versioned_with_metadata, versioned_deletion);
        assert_inconsistent_same_kind!(versioned_with_metadata, resolved);
        assert_inconsistent_same_kind!(versioned_deletion, resolved);
        // Test inconsistency with downcast.
        assert_inconsistent_downcast!(versioned_legacy, metadata);
        assert_inconsistent_downcast!(versioned_legacy, deletion_metadata);
        assert_inconsistent_downcast!(versioned_legacy, not_exists);
        assert_inconsistent_downcast!(resolved, deletion_metadata);
        assert_inconsistent_downcast!(resolved, metadata);
        assert_inconsistent_downcast!(resolved, not_exists);
        assert_inconsistent_downcast!(versioned_with_metadata, legacy_metadata);
        assert_inconsistent_downcast!(versioned_with_metadata, deletion_metadata);
        assert_inconsistent_downcast!(versioned_with_metadata, not_exists);
        assert_inconsistent_downcast!(versioned_deletion, legacy_metadata);
        assert_inconsistent_downcast!(versioned_deletion, metadata);
        assert_inconsistent_downcast!(versioned_deletion, exists);
        assert_inconsistent_downcast!(metadata, not_exists);
        assert_inconsistent_downcast!(legacy_metadata, not_exists);
        assert_inconsistent_downcast!(deletion_metadata, exists);

        // Test that V is getting ignored in the comparison.
        assert_eq!(
            versioned_legacy,
            DataRead::Versioned(
                Err(StorageVersion),
                Arc::new(ValueType::with_len_and_metadata(10, None))
            )
        );
    }

    #[derive(Clone, Debug)]
    struct TestTransactionType {}

    impl BlockExecutableTransaction for TestTransactionType {
        type Event = MockEvent;
        type Identifier = ();
        type Key = KeyType<u32>;
        type Tag = u32;
        type Value = ValueType;
    }

    macro_rules! assert_update_incorrect_use {
        ($m:expr, $x:expr, $y:expr) => {{
            let original = $m.get(&$x).cloned().unwrap();
            assert_matches!(
                CapturedReads::<TestTransactionType>::update_entry($m.entry($x), $y.clone()),
                UpdateResult::IncorrectUse(_)
            );
            assert_some_eq!($m.get(&$x), &original);
        }};
    }

    macro_rules! assert_update_inconsistency {
        ($m:expr, $x:expr, $y:expr) => {{
            let original = $m.get(&$x).cloned().unwrap();
            assert_matches!(
                CapturedReads::<TestTransactionType>::update_entry($m.entry($x), $y.clone()),
                UpdateResult::Inconsistency(_)
            );
            assert_some_eq!($m.get(&$x), &original);
        }};
    }

    macro_rules! assert_update {
        ($m:expr, $x:expr, $y:expr) => {{
            assert_matches!(
                CapturedReads::<TestTransactionType>::update_entry($m.entry($x), $y.clone()),
                UpdateResult::Updated
            );
            assert_some_eq!($m.get(&$x), &$y);
        }};
    }

    macro_rules! assert_insert {
        ($m:expr, $x:expr, $y:expr) => {{
            assert_matches!(
                CapturedReads::<TestTransactionType>::update_entry($m.entry($x), $y.clone()),
                UpdateResult::Inserted
            );
            assert_some_eq!($m.get(&$x), &$y);
        }};
    }

    fn raw_metadata() -> StateValueMetadataKind {
        Some(StateValueMetadata::new(5, &CurrentTimeMicroseconds {
            microseconds: 7,
        }))
    }

    #[test]
    fn test_update_entry() {
        // Legacy state values do not have metadata.
        let versioned_legacy = DataRead::Versioned(
            Err(StorageVersion),
            Arc::new(ValueType::with_len_and_metadata(1, None)),
        );
        let versioned_deletion = DataRead::Versioned(
            Ok((5, 1)),
            Arc::new(ValueType::with_len_and_metadata(0, None)),
        );
        let versioned_with_metadata = DataRead::Versioned(
            Ok((7, 0)),
            Arc::new(ValueType::with_len_and_metadata(2, raw_metadata())),
        );
        let resolved = DataRead::Resolved::<ValueType>(200);
        let deletion_metadata = DataRead::Metadata(None);
        let legacy_metadata = DataRead::Metadata(Some(None));
        let metadata = DataRead::Metadata(Some(raw_metadata()));
        let exists = DataRead::Exists(true);
        let not_exists = DataRead::Exists(false);

        let mut map: HashMap<u32, DataRead<ValueType>> = HashMap::new();
        assert_none!(map.get(&0));

        // Populate the empty entry.
        assert_insert!(map, 0, not_exists);
        // Update to the same data is not correct use.
        assert_update_incorrect_use!(map, 0, not_exists);
        // Incorrect use (<= kind provided than already captured) takes precedence
        // over inconsistency.
        assert_update_incorrect_use!(map, 0, exists);

        // Update to a consistent higher kind.
        assert_update!(map, 0, deletion_metadata);
        // Update w. consistent lower kind is not correct use.
        assert_update_incorrect_use!(map, 0, not_exists);

        // More of the above behavior, with different kinds.
        assert_update_incorrect_use!(map, 0, deletion_metadata);

        assert_update_incorrect_use!(map, 0, exists);
        assert_update_inconsistency!(map, 0, resolved);
        assert_update_incorrect_use!(map, 0, exists);
        assert_update_inconsistency!(map, 0, versioned_with_metadata);
        // Updated key 0 for the last time.
        assert_update!(map, 0, versioned_deletion);
        assert_update_incorrect_use!(map, 0, not_exists);
        assert_update_incorrect_use!(map, 0, legacy_metadata);
        assert_update_incorrect_use!(map, 0, metadata);
        assert_update_incorrect_use!(map, 0, deletion_metadata);
        assert_update_incorrect_use!(map, 0, versioned_legacy);

        assert_none!(map.get(&1));
        assert_insert!(map, 1, metadata);
        assert_update_incorrect_use!(map, 1, legacy_metadata);
        assert_update_inconsistency!(map, 1, versioned_legacy);
        assert_update_incorrect_use!(map, 1, exists);
        assert_update_inconsistency!(map, 1, versioned_deletion);
        assert_update_incorrect_use!(map, 1, not_exists);
        assert_update_incorrect_use!(map, 1, metadata);
        assert_update_inconsistency!(map, 1, resolved);
        assert_update!(map, 1, versioned_with_metadata);
        assert_update_incorrect_use!(map, 1, metadata);
        assert_update_incorrect_use!(map, 1, not_exists);
        assert_update_incorrect_use!(map, 1, exists);
        assert_update_incorrect_use!(map, 1, legacy_metadata);
        assert_update_incorrect_use!(map, 1, versioned_deletion);

        assert_none!(map.get(&2));
        assert_insert!(map, 2, legacy_metadata);
        assert_update!(map, 2, resolved);
        assert_update_incorrect_use!(map, 2, versioned_legacy);
        assert_update_incorrect_use!(map, 2, legacy_metadata);
        assert_update_incorrect_use!(map, 2, versioned_deletion);
        assert_update_incorrect_use!(map, 2, not_exists);
        assert_update_incorrect_use!(map, 2, metadata);
        assert_update_incorrect_use!(map, 2, exists);
        assert_update_incorrect_use!(map, 2, versioned_with_metadata);
        assert_update_incorrect_use!(map, 2, deletion_metadata);
        assert_update_incorrect_use!(map, 2, resolved);
    }

    fn legacy_reads_by_kind() -> Vec<DataRead<ValueType>> {
        let exists = DataRead::Exists(true);
        let legacy_metadata = DataRead::Metadata(Some(None));
        let versioned_legacy = DataRead::Versioned(
            Err(StorageVersion),
            Arc::new(ValueType::with_len_and_metadata(1, None)),
        );
        vec![exists, legacy_metadata, versioned_legacy]
    }

    fn deletion_reads_by_kind() -> Vec<DataRead<ValueType>> {
        let versioned_deletion = DataRead::Versioned(
            Ok((5, 1)),
            Arc::new(ValueType::with_len_and_metadata(0, None)),
        );
        let deletion_metadata = DataRead::Metadata(None);
        let not_exists = DataRead::Exists(false);
        vec![not_exists, deletion_metadata, versioned_deletion]
    }

    fn with_metadata_reads_by_kind() -> Vec<DataRead<ValueType>> {
        let versioned_with_metadata = DataRead::Versioned(
            Ok((7, 0)),
            Arc::new(ValueType::with_len_and_metadata(2, raw_metadata())),
        );
        let metadata = DataRead::Metadata(Some(raw_metadata()));
        let exists = DataRead::Exists(true);
        vec![exists, metadata, versioned_with_metadata]
    }

    macro_rules! assert_capture_get {
        ($x:expr, $k:expr, $mt:expr, $y:expr) => {{
            let read_kinds = vec![ReadKind::Exists, ReadKind::Metadata, ReadKind::Value];

            for i in 0..3 {
                if $mt.is_none() || i != 1 {
                    // Do not request metadata of group member.
                    assert_none!($x.get_by_kind(&$k, $mt.as_ref(), read_kinds[i].clone()));
                }
            }

            for i in 0..3 {
                assert_ok!($x.capture_read($k.clone(), $mt.clone(), $y[i].clone()));
                for j in 0..i {
                    if $mt.is_none() || j != 1 {
                        // Do not request metadata of group member
                        assert_some_eq!(
                            $x.get_by_kind(&$k, $mt.as_ref(), read_kinds[j].clone()),
                            $y[j]
                        );
                    }
                }
            }
        }};
    }

    #[test_case(false)]
    #[test_case(true)]
    fn capture_and_get_by_kind(use_tag: bool) {
        let mut captured_reads = CapturedReads::<TestTransactionType>::new();
        let legacy_reads = legacy_reads_by_kind();
        let deletion_reads = deletion_reads_by_kind();
        let with_metadata_reads = with_metadata_reads_by_kind();

        assert_capture_get!(
            captured_reads,
            KeyType::<u32>(10, false),
            use_tag.then_some(30),
            legacy_reads
        );
        assert_capture_get!(
            captured_reads,
            KeyType::<u32>(11, false),
            use_tag.then_some(30),
            deletion_reads
        );
        assert_capture_get!(
            captured_reads,
            KeyType::<u32>(15, false),
            use_tag.then_some(30),
            with_metadata_reads
        );
    }

    #[should_panic]
    #[test]
    fn metadata_for_group_member() {
        let captured_reads = CapturedReads::<TestTransactionType>::new();
        captured_reads.get_by_kind(&KeyType::<u32>(21, false), Some(&10), ReadKind::Metadata);
    }

    macro_rules! assert_incorrect_use {
        ($x:expr, $k:expr, $mt:expr, $y:expr) => {{
            assert!(!$x.incorrect_use);

            for i in 0..3 {
                assert_ok!($x.capture_read($k.clone(), $mt.clone(), $y[i].clone()));
                for j in 0..(i + 1) {
                    assert_err!($x.capture_read($k, $mt.clone(), $y[j].clone()));
                    assert!($x.incorrect_use);
                    $x.incorrect_use = false;
                }
            }
        }};
    }

    #[test_case(false)]
    #[test_case(true)]
    fn incorrect_use_flag(use_tag: bool) {
        let mut captured_reads = CapturedReads::<TestTransactionType>::new();
        let legacy_reads = legacy_reads_by_kind();
        let deletion_reads = deletion_reads_by_kind();
        let with_metadata_reads = with_metadata_reads_by_kind();

        let resolved = DataRead::Resolved::<ValueType>(200);
        let mixed_reads = vec![
            deletion_reads[0].clone(),
            with_metadata_reads[1].clone(),
            resolved,
        ];

        assert_incorrect_use!(
            captured_reads,
            KeyType::<u32>(10, false),
            use_tag.then_some(30),
            legacy_reads
        );
        assert_incorrect_use!(
            captured_reads,
            KeyType::<u32>(11, false),
            use_tag.then_some(30),
            deletion_reads
        );
        assert_incorrect_use!(
            captured_reads,
            KeyType::<u32>(15, false),
            use_tag.then_some(30),
            with_metadata_reads
        );

        // Test incorrect with with incompatible types.
        assert!(!captured_reads.incorrect_use);

        for i in 0..3 {
            let key = KeyType::<u32>(20 + i, false);
            assert_ok!(captured_reads.capture_read(
                key,
                use_tag.then_some(30),
                mixed_reads[i as usize].clone()
            ));
            for j in 0..(i + 1) {
                assert_err!(captured_reads.capture_read(
                    key,
                    use_tag.then_some(30),
                    mixed_reads[j as usize].clone()
                ));
                assert!(captured_reads.incorrect_use);
                captured_reads.incorrect_use = false;
            }
        }
    }

    #[test_case(false)]
    #[test_case(true)]
    fn speculative_failure_flag(use_tag: bool) {
        let mut captured_reads = CapturedReads::<TestTransactionType>::new();
        let versioned_legacy = DataRead::Versioned(
            Err(StorageVersion),
            Arc::new(ValueType::with_len_and_metadata(1, None)),
        );
        let resolved = DataRead::Resolved::<ValueType>(200);
        let metadata = DataRead::Metadata(Some(raw_metadata()));
        let deletion_metadata = DataRead::Metadata(None);
        let exists = DataRead::Exists(true);

        assert!(!captured_reads.speculative_failure);
        let key = KeyType::<u32>(20, false);
        assert_ok!(captured_reads.capture_read(key, use_tag.then_some(30), exists));
        assert_err!(captured_reads.capture_read(
            key,
            use_tag.then_some(30),
            deletion_metadata.clone()
        ));
        assert!(captured_reads.speculative_failure);

        captured_reads.speculative_failure = false;
        let key = KeyType::<u32>(21, false);
        assert_ok!(captured_reads.capture_read(key, use_tag.then_some(30), deletion_metadata));
        assert_err!(captured_reads.capture_read(key, use_tag.then_some(30), resolved));
        assert!(captured_reads.speculative_failure);

        captured_reads.speculative_failure = false;
        let key = KeyType::<u32>(22, false);
        assert_ok!(captured_reads.capture_read(key, use_tag.then_some(30), metadata));
        assert_err!(captured_reads.capture_read(key, use_tag.then_some(30), versioned_legacy));
        assert!(captured_reads.speculative_failure);

        captured_reads.speculative_failure = false;
        captured_reads.mark_failure();
        assert!(captured_reads.speculative_failure);
    }
}
