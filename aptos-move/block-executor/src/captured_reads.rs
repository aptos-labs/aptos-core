// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{code_cache_global::GlobalModuleCache, types::InputOutputKey, view::LatestView};
use anyhow::bail;
use aptos_aggregator::{
    delta_math::DeltaHistory,
    types::{DelayedFieldValue, DelayedFieldsSpeculativeError, ReadPosition},
};
use aptos_mvhashmap::{
    types::{
        Incarnation, MVDataError, MVDataOutput, MVDelayedFieldsError, MVGroupError, StorageVersion, TxnIndex, ValueWithLayout, Version
    },
    versioned_data::VersionedData,
    versioned_delayed_fields::TVersionedDelayedFieldView,
    versioned_group_data::VersionedGroupData,
};
use aptos_types::{
    error::{code_invariant_error, PanicError, PanicOr},
    executable::ModulePath,
    state_store::{state_value::StateValueMetadata, TStateView},
    transaction::BlockExecutableTransaction as Transaction,
    write_set::TransactionWrite,
};
use aptos_vm_types::resolver::ResourceGroupSize;
use derivative::Derivative;
use move_core_types::value::MoveTypeLayout;
use move_vm_types::{
    code::{ModuleCode, SyncModuleCache, WithAddress, WithName, WithSize},
    delayed_values::delayed_field_id::DelayedFieldID,
};
use std::{
    collections::{
        hash_map::Entry::{self, Occupied, Vacant},
        BTreeMap, BTreeSet, HashMap, HashSet,
    }, hash::Hash, ops::Deref, sync::Arc
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
#[derivative(Clone(bound = ""), Debug(bound = ""))]
pub(crate) enum DataRead<V: PartialEq + Eq> {
    // Version supersedes V comparison.
    Versioned(
        Version,
        // Currently, we are conservative and check the version for equality
        // (version implies value equality, but not vice versa). TODO: when
        // comparing the instances of V is cheaper, compare those instead.
        #[derivative(Debug = "ignore")] Arc<V>,
        #[derivative(Debug = "ignore")] Option<Arc<MoveTypeLayout>>,
    ),
    Metadata(Option<StateValueMetadata>),
    Exists(bool),
    /// Read resolved an aggregatorV1 delta to a value.
    /// TODO[agg_v1](cleanup): deprecate.
    Resolved(u128),
}

fn data_read_equals<V: PartialEq + Eq>(
    v1: &DataRead<V>,
    v2: &DataRead<V>,
    block_stm_v2: bool,
) -> bool {
    match (v1, v2) {
        (
            DataRead::Versioned(v1_version, v1_value, v1_layout),
            DataRead::Versioned(v2_version, v2_value, v2_layout),
        ) => {
            if !block_stm_v2 {
                v1_version == v2_version
            } else {
                v1_value == v2_value && v1_layout == v2_layout
            }
        },
        (DataRead::Metadata(v1_metadata), DataRead::Metadata(v2_metadata)) => {
            v1_metadata == v2_metadata
        },
        (DataRead::Exists(v1_exists), DataRead::Exists(v2_exists)) => v1_exists == v2_exists,
        (DataRead::Resolved(v1_resolved), DataRead::Resolved(v2_resolved)) => {
            v1_resolved == v2_resolved
        },
        _ => false,
    }
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
            Versioned(_, _, _) | Resolved(_) => ReadKind::Value,
            Metadata(_) => ReadKind::Metadata,
            Exists(_) => ReadKind::Exists,
        }
    }

    // A convenience method, since the same key can be read in different modes, producing
    // different DataRead / ReadKinds. Returns true if self has >= kind than other, i.e.
    // contains more or equal information, and is consistent with the information in other.
    fn contains(&self, other: &DataRead<V>, block_stm_v2: bool) -> DataReadComparison {
        let self_kind = self.get_kind();
        let other_kind = other.get_kind();

        if self_kind < other_kind {
            DataReadComparison::Insufficient
        } else {
            let downcast_eq = if self_kind == other_kind {
                // Optimization to avoid unnecessary clones (e.g. during validation).
                data_read_equals(self, other, block_stm_v2)
            } else {
                data_read_equals(
                    &self
                        .downcast(other_kind)
                        .expect("Downcast to lower kind must succeed"),
                    other,
                    block_stm_v2,
                )
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
            (DataRead::Versioned(_, v, _), ReadKind::Metadata) => {
                // For deletion, as_state_value_metadata returns None, also asserted by tests.
                DataRead::Metadata(v.as_state_value_metadata())
            },
            (DataRead::Versioned(_, v, _), ReadKind::Exists) => DataRead::Exists(!v.is_deletion()),
            (DataRead::Resolved(_), ReadKind::Metadata) => {
                DataRead::Metadata(Some(StateValueMetadata::none()))
            },
            (DataRead::Resolved(_), ReadKind::Exists) => DataRead::Exists(true),
            (DataRead::Metadata(maybe_metadata), ReadKind::Exists) => {
                DataRead::Exists(maybe_metadata.is_some())
            },
            (_, _) => unreachable!("{:?}, {:?} must be covered", self_kind, kind),
        })
    }

    pub(crate) fn from_value_with_layout(version: Version, value: ValueWithLayout<V>) -> Self {
        match value {
            // If value was never exchanged, then metadata can be the highest one without full value.
            // This is because captured read should never be RawFromStorage, and direct validation
            // reads will be checked if they contain the captured read - so downcasting is okay.
            ValueWithLayout::RawFromStorage(v) => DataRead::Metadata(v.as_state_value_metadata()),
            ValueWithLayout::Exchanged(v, layout) => {
                DataRead::Versioned(version, v.clone(), layout)
            },
        }
    }
}

/// Additional state regarding groups that may be provided to the VM during transaction
/// execution and is captured. There may be a DataRead per tag within the group, and also
/// the group size, computed based on speculative information in MVHashMap, by "collecting"
/// over the latest contents present in the group, i.e. the respective tags and values (in
/// this sense, group size is even more speculative than other captured information, as it
/// does not depend on a single "latest" entry, but collected sizes of many "latest" entries).
#[derive(Derivative, Clone)]
#[derivative(Default(bound = ""))]
pub(crate) struct GroupRead<T: Transaction> {
    /// The size of the resource group can be read (used for gas charging).
    pub(crate) collected_size: Option<ResourceGroupSize>,
    /// Reads to individual resources in the group, keyed by a tag.
    pub(crate) inner_reads: HashMap<T::Tag, DataRead<T::Value>>,
}

/// Defines different ways `DelayedFieldResolver` can be used to read its values
/// from the state.
/// The enum variants should not be re-ordered, as it defines a relation
/// HistoryBounded < Value
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DelayedFieldReadKind {
    /// The returned value is guaranteed to be correct.
    HistoryBounded,
    /// The returned value is based on last committed value, ignoring
    /// any pending changes.
    Value,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DelayedFieldRead {
    // Represents a full read - that value has been returned to the caller,
    // meaning that read is valid only if value is identical.
    Value {
        value: DelayedFieldValue,
    },
    // Represents a restricted read - where a range of values that satisfy DeltaHistory
    // are all valid and produce the same outcome.
    // Only boolean outcomes of "try_add_delta" operations have been returned to the caller,
    // and so we need to respect that those return the same outcome when doing the validation.
    // Running inner_aggregator_value is kept only for internal bookkeeping - and is used to
    // as a value against which results are computed, but is not checked for read validation.
    // Only aggregators can be in the HistoryBounded state.
    HistoryBounded {
        restriction: DeltaHistory,
        max_value: u128,
        inner_aggregator_value: u128,
    },
}

impl DelayedFieldRead {
    fn get_kind(&self) -> DelayedFieldReadKind {
        use DelayedFieldRead::*;
        match self {
            Value { .. } => DelayedFieldReadKind::Value,
            HistoryBounded { .. } => DelayedFieldReadKind::HistoryBounded,
        }
    }

    /// If the reads contains sufficient information, return it, otherwise return None
    pub(crate) fn filter_by_kind(
        &self,
        min_kind: DelayedFieldReadKind,
    ) -> Option<DelayedFieldRead> {
        let self_kind = self.get_kind();
        // Respecting the ordering based on information: HistoryBounded < Value
        if self_kind >= min_kind {
            Some(self.clone())
        } else {
            None
        }
    }

    // A convenience method, since the same key can be read in different modes, producing
    // different DataRead / ReadKinds. Returns true if self has >= kind than other, i.e.
    // contains more or equal information, and is consistent with the information in other.
    fn contains(&self, other: &DelayedFieldRead) -> DataReadComparison {
        use DelayedFieldRead::*;
        match (&self, other) {
            (Value { value: v1, .. }, Value { value: v2, .. }) => {
                if v1 == v2 {
                    DataReadComparison::Contains
                } else {
                    DataReadComparison::Inconsistent
                }
            },
            (
                HistoryBounded {
                    restriction: h1,
                    inner_aggregator_value: v1,
                    max_value: m1,
                },
                HistoryBounded {
                    restriction: h2,
                    inner_aggregator_value: v2,
                    max_value: m2,
                },
            ) => {
                if v1 == v2 && m1 == m2 && h1.stricter_than(h2) {
                    DataReadComparison::Contains
                } else {
                    DataReadComparison::Inconsistent
                }
            },
            (HistoryBounded { .. }, Value { .. }) => DataReadComparison::Insufficient,
            (
                Value { value: v1 },
                HistoryBounded {
                    restriction: h2,
                    max_value: m2,
                    ..
                },
            ) => {
                if let Ok(v1) = v1.clone().into_aggregator_value() {
                    if h2.validate_against_base_value(v1, *m2).is_ok() {
                        DataReadComparison::Contains
                    } else {
                        DataReadComparison::Inconsistent
                    }
                } else {
                    DataReadComparison::Inconsistent
                }
            },
        }
    }
}

/// Represents a module read, either from global module cache that spans multiple blocks, or from
/// per-block cache used by block executor to add committed modules. When transaction reads a
/// module, it should first check the read-set here, to ensure that if some module A has been read,
/// the same A is read again within the same transaction.
enum ModuleRead<DC, VC, S> {
    /// Read from the global module cache. Modules in this cache have storage version, but require
    /// different validation - a check that they have not been overridden.
    GlobalCache(Arc<ModuleCode<DC, VC, S>>),
    /// Read from per-block cache that contains committed (by specified transaction) and newly
    /// loaded from storage (i.e., not yet moved to global module cache) modules.
    PerBlockCache(Option<(Arc<ModuleCode<DC, VC, S>>, Option<TxnIndex>)>),
}

/// Represents a result of a read from [CapturedReads] when they are used as the transaction-level
/// cache.
#[derive(Debug, Eq, PartialEq)]
pub enum CacheRead<T> {
    Hit(T),
    Miss,
}

/// Serves as a "read-set" of a transaction execution, and provides APIs for capturing reads,
/// resolving new reads based on already captured reads when possible, and for validation.
///
/// The intended use is that all reads should be attempted to be resolved from CapturedReads.
/// If not possible, then after proper resolution from MVHashMap/storage, they should be
/// captured. This enforces an invariant that 'capture_read' will never be called with a
/// read that has a kind <= already captured read (for that key / tag).
/// BlockSTMv2 adds incarnation to the CapturedReads, for now it's inside the v2 flag.
pub(crate) struct CapturedReads<T: Transaction, K, DC, VC, S> {
    data_reads: HashMap<T::Key, DataRead<T::Value>>,
    group_reads: HashMap<T::Key, GroupRead<T>>,
    delayed_field_reads: HashMap<DelayedFieldID, DelayedFieldRead>,
    pub(crate) aggregator_v1_reads: HashSet<T::Key>,

    #[deprecated]
    pub(crate) deprecated_module_reads: Vec<T::Key>,
    module_reads: hashbrown::HashMap<K, ModuleRead<DC, VC, S>>,

    /// If there is a speculative failure (e.g. delta application failure, or an observed
    /// inconsistency), the transaction output is irrelevant (must be discarded and transaction
    /// re-executed). We have two global flags, one for speculative failures regarding
    /// delayed fields, and the second for all other speculative failures, because these
    /// require different validation behavior (delayed fields are validated commit-time).
    delayed_field_speculative_failure: bool,
    non_delayed_field_speculative_failure: bool,
    /// Set if the invariant on CapturedReads intended use is violated. Leads to an alert
    /// and sequential execution fallback.
    incorrect_use: bool,

    block_stm_v2_incarnation: Option<Incarnation>,
}

impl<T: Transaction, K, DC, VC, S> Default for CapturedReads<T, K, DC, VC, S> {
    fn default() -> Self {
        Self::new(None)
    }
}

impl<T: Transaction, K, DC, VC, S> CapturedReads<T, K, DC, VC, S> {
    #[allow(deprecated)]
    pub(crate) fn new(block_stm_v2_incarnation: Option<Incarnation>) -> Self {
        Self {
            data_reads: HashMap::new(),
            group_reads: HashMap::new(),
            delayed_field_reads: HashMap::new(),
            aggregator_v1_reads: HashSet::new(),
            deprecated_module_reads: Vec::new(),
            module_reads: hashbrown::HashMap::new(),
            delayed_field_speculative_failure: false,
            non_delayed_field_speculative_failure: false,
            incorrect_use: false,
            block_stm_v2_incarnation,
        }
    }
}

#[derive(Debug)]
enum UpdateResult {
    Inserted,
    Updated,
    IncorrectUse(String),
    Inconsistency(String),
}

impl<T, K, DC, VC, S> CapturedReads<T, K, DC, VC, S>
where
    T: Transaction,
    K: Hash + Eq + Ord + Clone,
    VC: Deref<Target = Arc<DC>>,
    S: WithSize,
{
    // Return an iterator over the captured reads.
    pub(crate) fn get_read_values_with_delayed_fields<SV: TStateView<Key = T::Key>>(
        &self,
        view: &LatestView<T, SV>,
        delayed_write_set_ids: &HashSet<DelayedFieldID>,
        skip: &HashSet<T::Key>,
    ) -> Result<BTreeMap<T::Key, (StateValueMetadata, u64, Arc<MoveTypeLayout>)>, PanicError> {
        self.data_reads
            .iter()
            .filter_map(|(key, data_read)| {
                if skip.contains(key) {
                    return None;
                }

                if let DataRead::Versioned(_version, value, Some(layout)) = data_read {
                    view.filter_value_for_exchange(value, layout, delayed_write_set_ids, key)
                } else {
                    None
                }
            })
            .collect()
    }

    // Return an iterator over the captured group reads that contain a delayed field
    pub(crate) fn get_group_read_values_with_delayed_fields<'a>(
        &'a self,
        skip: &'a HashSet<T::Key>,
    ) -> impl Iterator<Item = (&T::Key, &GroupRead<T>)> {
        self.group_reads.iter().filter(|(key, group_read)| {
            !skip.contains(key)
                && group_read
                    .inner_reads
                    .iter()
                    .any(|(_, data_read)| matches!(data_read, DataRead::Versioned(_, _, Some(_))))
        })
    }

    // Given a hashmap entry for a key, incorporate a new DataRead. This checks
    // consistency and ensures that the most comprehensive read is recorded.
    fn update_entry<Q, V: TransactionWrite>(
        entry: Entry<Q, DataRead<V>>,
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
                    // TODO:(BlockSTMv2): should never be comparing Value to Value so flag doesn't matter.
                    match read.contains(existing_read, false) {
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

    pub(crate) fn capture_group_size(
        &mut self,
        group_key: T::Key,
        group_size: ResourceGroupSize,
    ) -> anyhow::Result<()> {
        let group = self.group_reads.entry(group_key).or_default();

        if let Some(recorded_size) = group.collected_size {
            if recorded_size != group_size {
                bail!("Inconsistent recorded group size");
            }
        }

        group.collected_size = Some(group_size);
        Ok(())
    }

    pub(crate) fn group_size(&self, group_key: &T::Key) -> Option<ResourceGroupSize> {
        self.group_reads
            .get(group_key)
            .and_then(|group| group.collected_size)
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
                self.non_delayed_field_speculative_failure = true;
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

    pub(crate) fn capture_delayed_field_read(
        &mut self,
        id: DelayedFieldID,
        update: bool,
        read: DelayedFieldRead,
    ) -> Result<(), PanicOr<DelayedFieldsSpeculativeError>> {
        let result = match self.delayed_field_reads.entry(id) {
            Vacant(e) => {
                e.insert(read);
                UpdateResult::Inserted
            },
            Occupied(mut e) => {
                let existing_read = e.get_mut();
                let read_kind = read.get_kind();
                let existing_kind = existing_read.get_kind();
                if read_kind < existing_kind || (!update && read_kind == existing_kind) {
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
        };

        match result {
            UpdateResult::IncorrectUse(m) => {
                self.incorrect_use = true;
                Err(code_invariant_error(m).into())
            },
            UpdateResult::Inconsistency(_) => {
                // Record speculative failure.
                self.delayed_field_speculative_failure = true;
                Err(PanicOr::Or(DelayedFieldsSpeculativeError::InconsistentRead))
            },
            UpdateResult::Updated | UpdateResult::Inserted => Ok(()),
        }
    }

    pub(crate) fn capture_delayed_field_read_error<E: std::fmt::Debug>(&mut self, e: &PanicOr<E>) {
        match e {
            PanicOr::CodeInvariantError(_) => self.incorrect_use = true,
            PanicOr::Or(_) => self.delayed_field_speculative_failure = true,
        };
    }

    pub(crate) fn get_delayed_field_by_kind(
        &self,
        id: &DelayedFieldID,
        min_kind: DelayedFieldReadKind,
    ) -> Option<DelayedFieldRead> {
        self.delayed_field_reads
            .get(id)
            .and_then(|r| r.filter_by_kind(min_kind))
    }

    pub(crate) fn is_incorrect_use(&self) -> bool {
        self.incorrect_use
    }

    fn validate_data_reads_impl<'a>(
        &'a self,
        mut iter: impl Iterator<Item = (&'a T::Key, &'a DataRead<T::Value>)>,
        data_map: &VersionedData<T::Key, T::Value>,
        idx_to_validate: TxnIndex,
    ) -> bool {
        use MVDataError::*;
        use MVDataOutput::*;
        iter.all(|(key, read)| {
            match data_map.fetch_data(key, idx_to_validate) {
                Ok(Versioned(version, value)) => {
                    matches!(
                        DataRead::from_value_with_layout(version, value).contains(read, self.block_stm_v2_incarnation.is_some()),
                        DataReadComparison::Contains
                    )
                },
                Ok(Resolved(value)) => {
                    matches!(
                        DataRead::Resolved(value).contains(read, self.block_stm_v2_incarnation.is_some()),
                        DataReadComparison::Contains
                    )
                },
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

    pub(crate) fn validate_aggregator_v1_reads(
        &self,
        data_map: &VersionedData<T::Key, T::Value>,
        aggregator_write_keys: &BTreeSet<T::Key>,
        idx_to_validate: TxnIndex,
    ) -> Result<bool, PanicError> {
        let mut aggregator_v1_iterable = Vec::with_capacity(self.aggregator_v1_reads.len());
        for k in &self.aggregator_v1_reads {
            match self.data_reads.get(k) {
                Some(data_read) => aggregator_v1_iterable.push((k, data_read)),
                None => {
                    return Err(code_invariant_error(format!(
                        "Aggregator v1 read {:?} not found among captured data reads",
                        k
                    )));
                },
            }
        }

        let ret = self.validate_data_reads_impl(
            aggregator_v1_iterable.into_iter(),
            data_map,
            idx_to_validate,
        );

        if ret {
            // Additional invariant check (that AggregatorV1 reads are captured for aggregator write keys). 
            // Moreover no extra work when aggregator_v1_reads is empty, which happens for BlockSTMv1.
            for k in aggregator_write_keys {
                if self.data_reads.get(k).is_some() && !self.aggregator_v1_reads.contains(k) {
                    return Err(code_invariant_error(format!(
                        "Captured read at aggregator key {:?} not found among AggregatorV1 reads",
                        k
                    )));
                }
            }
        }

        Ok(ret)
    }

    pub(crate) fn validate_data_reads(
        &self,
        data_map: &VersionedData<T::Key, T::Value>,
        idx_to_validate: TxnIndex,
    ) -> bool {
        if self.non_delayed_field_speculative_failure {
            return false;
        }

        self.validate_data_reads_impl(self.data_reads.iter(), data_map, idx_to_validate)
    }

    /// Records the read to global cache that spans across multiple blocks.
    pub(crate) fn capture_global_cache_read(&mut self, key: K, read: Arc<ModuleCode<DC, VC, S>>) {
        self.module_reads.insert(key, ModuleRead::GlobalCache(read));
    }

    /// Records the read to per-block level cache.
    pub(crate) fn capture_per_block_cache_read(
        &mut self,
        key: K,
        read: Option<(Arc<ModuleCode<DC, VC, S>>, Option<TxnIndex>)>,
    ) {
        self.module_reads
            .insert(key, ModuleRead::PerBlockCache(read));
    }

    /// If the module has been previously read, returns it.
    pub(crate) fn get_module_read(
        &self,
        key: &K,
    ) -> CacheRead<Option<(Arc<ModuleCode<DC, VC, S>>, Option<TxnIndex>)>> {
        match self.module_reads.get(key) {
            Some(ModuleRead::PerBlockCache(read)) => CacheRead::Hit(read.clone()),
            Some(ModuleRead::GlobalCache(read)) => {
                // From global cache, we return a storage version.
                CacheRead::Hit(Some((read.clone(), None)))
            },
            None => CacheRead::Miss,
        }
    }

    pub(crate) fn validate_module_reads(
        &self,
        global_module_cache: &GlobalModuleCache<K, DC, VC, S>,
        per_block_module_cache: &SyncModuleCache<K, DC, VC, S, Option<TxnIndex>>,
        maybe_updated_module_keys: Option<&BTreeSet<K>>,
    ) -> bool {
        if self.non_delayed_field_speculative_failure {
            return false;
        }

        let validate = |key: &K, read: &ModuleRead<DC, VC, S>| match read {
            ModuleRead::GlobalCache(_) => global_module_cache.contains_not_overridden(key),
            ModuleRead::PerBlockCache(previous) => {
                let current_version = per_block_module_cache.get_module_version(key);
                let previous_version = previous.as_ref().map(|(_, version)| *version);
                current_version == previous_version
            },
        };

        match maybe_updated_module_keys {
            Some(updated_module_keys) if updated_module_keys.len() <= self.module_reads.len() => {
                // When updated_module_keys is smaller, iterate over it and lookup in module_reads
                updated_module_keys
                    .iter()
                    .filter(|&k| self.module_reads.contains_key(k))
                    .all(|key| validate(key, self.module_reads.get(key).unwrap()))
            },
            Some(updated_module_keys) => {
                // When module_reads is smaller, iterate over it and filter by updated_module_keys
                self.module_reads
                    .iter()
                    .filter(|(k, _)| updated_module_keys.contains(k))
                    .all(|(key, read)| validate(key, read))
            },
            None => self.module_reads.iter().all(|(key, read)| validate(key, read)),
        }
    }

    pub(crate) fn block_stm_v2_incarnation(&self) -> Option<Incarnation> {
        self.block_stm_v2_incarnation
    }

    pub(crate) fn validate_group_reads(
        &self,
        group_map: &VersionedGroupData<T::Key, T::Tag, T::Value>,
        idx_to_validate: TxnIndex,
    ) -> bool {
        use MVGroupError::*;

        if self.non_delayed_field_speculative_failure {
            return false;
        }

        self.group_reads.iter().all(|(key, group)| {
            let mut ret = true;
            if let Some(size) = group.collected_size {
                ret = group_map.validate_group_size(key, idx_to_validate, size);
            }

            ret && group.inner_reads.iter().all(|(tag, r)| {
                match group_map.fetch_tagged_data(key, tag, idx_to_validate) {
                    Ok((version, v)) => {
                        matches!(
                            DataRead::from_value_with_layout(version, v)
                                .contains(r, self.block_stm_v2_incarnation.is_some()),
                            DataReadComparison::Contains
                        )
                    },
                    Err(TagNotFound) => {
                        let sentinel_deletion =
                            Arc::<T::Value>::new(TransactionWrite::from_state_value(None));
                        assert!(sentinel_deletion.is_deletion());
                        matches!(
                            DataRead::Versioned(Err(StorageVersion), sentinel_deletion, None)
                                .contains(r, self.block_stm_v2_incarnation.is_some()),
                            DataReadComparison::Contains
                        )
                    },
                    Err(Dependency(_)) => {
                        // panic!("Dependency {}", idx_to_validate);
                        false
                    }
                    Err(Uninitialized) => {
                        unreachable!("May not be uninitialized if captured for validation");
                    },
                }
            })
        })
    }

    // This validation needs to be called at commit time
    // (as it internally uses read_latest_predicted_value to get the current value).
    pub(crate) fn validate_delayed_field_reads(
        &self,
        delayed_fields: &dyn TVersionedDelayedFieldView<DelayedFieldID>,
        idx_to_validate: TxnIndex,
    ) -> Result<bool, PanicError> {
        if self.delayed_field_speculative_failure {
            return Ok(false);
        }

        use MVDelayedFieldsError::*;
        for (id, read_value) in &self.delayed_field_reads {
            match delayed_fields.read_latest_predicted_value(
                id,
                idx_to_validate,
                ReadPosition::BeforeCurrentTxn,
            ) {
                Ok(current_value) => match read_value {
                    DelayedFieldRead::Value { value, .. } => {
                        if value != &current_value {
                            return Ok(false);
                        }
                    },
                    DelayedFieldRead::HistoryBounded {
                        restriction,
                        max_value,
                        ..
                    } => match restriction.validate_against_base_value(
                        current_value.into_aggregator_value()?,
                        *max_value,
                    ) {
                        Ok(_) => {},
                        Err(_) => {
                            return Ok(false);
                        },
                    },
                },
                Err(NotFound) | Err(Dependency(_)) | Err(DeltaApplicationFailure) => {
                    return Ok(false);
                },
            }
        }
        Ok(true)
    }

    pub(crate) fn mark_failure(&mut self, delayed_field_failure: bool) {
        if delayed_field_failure {
            self.delayed_field_speculative_failure = true;
        } else {
            self.non_delayed_field_speculative_failure = true;
        }
    }

    pub(crate) fn mark_incorrect_use(&mut self) {
        self.incorrect_use = true;
    }
}

impl<T, K, DC, VC, S> CapturedReads<T, K, DC, VC, S>
where
    T: Transaction,
    K: Hash + Eq + Ord + Clone + WithAddress + WithName,
    VC: Deref<Target = Arc<DC>>,
{
    pub(crate) fn get_read_summary(&self) -> HashSet<InputOutputKey<T::Key, T::Tag>> {
        let mut ret = HashSet::new();
        for (key, read) in &self.data_reads {
            if let DataRead::Versioned(_, _, _) = read {
                ret.insert(InputOutputKey::Resource(key.clone()));
            }
        }

        for (key, group_reads) in &self.group_reads {
            for (tag, read) in &group_reads.inner_reads {
                if let DataRead::Versioned(_, _, _) = read {
                    ret.insert(InputOutputKey::Group(key.clone(), tag.clone()));
                }
            }
        }

        // TODO(loader_v2): Test summaries are the same.
        #[allow(deprecated)]
        for key in &self.deprecated_module_reads {
            ret.insert(InputOutputKey::Resource(key.clone()));
        }
        for key in self.module_reads.keys() {
            let key = T::Key::from_address_and_module_name(key.address(), key.name());
            ret.insert(InputOutputKey::Resource(key));
        }

        for (key, read) in &self.delayed_field_reads {
            if let DelayedFieldRead::Value { .. } = read {
                ret.insert(InputOutputKey::DelayedField(*key));
            }
        }

        ret
    }
}

#[derive(Derivative)]
#[derivative(Default(bound = "", new = "true"))]
pub(crate) struct UnsyncReadSet<T: Transaction, K> {
    pub(crate) resource_reads: HashSet<T::Key>,
    pub(crate) group_reads: HashMap<T::Key, HashSet<T::Tag>>,
    pub(crate) delayed_field_reads: HashSet<DelayedFieldID>,
    module_reads: HashSet<K>,
}

impl<T, K> UnsyncReadSet<T, K>
where
    T: Transaction,
    K: Hash + Eq + Ord + Clone + WithAddress + WithName,
{
    /// Captures the module read for sequential execution.
    pub(crate) fn capture_module_read(&mut self, key: K) {
        self.module_reads.insert(key);
    }

    pub(crate) fn get_read_summary(&self) -> HashSet<InputOutputKey<T::Key, T::Tag>> {
        let mut ret = HashSet::new();
        for key in &self.resource_reads {
            ret.insert(InputOutputKey::Resource(key.clone()));
        }

        for (key, group_reads) in &self.group_reads {
            for tag in group_reads {
                ret.insert(InputOutputKey::Group(key.clone(), tag.clone()));
            }
        }

        for key in &self.module_reads {
            let key = T::Key::from_address_and_module_name(key.address(), key.name());
            ret.insert(InputOutputKey::Resource(key));
        }

        for key in &self.delayed_field_reads {
            ret.insert(InputOutputKey::DelayedField(*key));
        }

        ret
    }
}

#[cfg(test)]
mod test {
    // TODO(BlockSTMv2): test compare_data_reads with true.
    use super::*;
    use crate::{
        code_cache_global::GlobalModuleCache,
        proptest_types::{
            mock_executor::MockEvent,
            types::{raw_metadata, KeyType, ValueType},
        },
    };
    use aptos_mvhashmap::{types::StorageVersion, MVHashMap};
    use claims::{assert_err, assert_gt, assert_matches, assert_none, assert_ok, assert_ok_eq};
    use move_vm_types::{
        code::{
            mock_deserialized_code, mock_verified_code, MockDeserializedCode, MockExtension,
            MockVerifiedCode, ModuleCache,
        },
        delayed_values::delayed_field_id::DelayedFieldID,
    };
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
                Arc::new(ValueType::with_len_and_metadata(
                    1,
                    StateValueMetadata::none()
                )),
                None,
            )
            .get_kind(),
            ReadKind::Value
        );
        assert_eq!(
            DataRead::Resolved::<ValueType>(200).get_kind(),
            ReadKind::Value
        );
        assert_eq!(
            DataRead::Metadata::<ValueType>(Some(StateValueMetadata::none())).get_kind(),
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
            assert!(!data_read_equals(&$x, &$y, false));
            assert!(!data_read_equals(&$y, &$x, false));
            assert_matches!($x.contains(&$y, false), DataReadComparison::Inconsistent);
            assert_matches!($y.contains(&$x, false), DataReadComparison::Inconsistent);
        }};
    }

    macro_rules! assert_inconsistent_downcast {
        ($x:expr, $y:expr) => {{
            assert!(!data_read_equals(&$x, &$y, false));
            assert!(!data_read_equals(&$y, &$x, false));
            assert_matches!($x.contains(&$y, false), DataReadComparison::Inconsistent);
            assert_matches!($y.contains(&$x, false), DataReadComparison::Insufficient);
        }};
    }

    macro_rules! assert_contains {
        ($x:expr, $y:expr) => {{
            assert!(data_read_equals(
                &$x.downcast($y.get_kind()).unwrap(),
                &$y,
                false
            ));
            assert_matches!($x.contains(&$y, false), DataReadComparison::Contains);
        }};
    }

    macro_rules! assert_insufficient {
        ($x:expr, $y:expr) => {{
            assert_none!($x.downcast($y.get_kind()));
            assert_matches!($x.contains(&$y, false), DataReadComparison::Insufficient);
        }};
    }

    #[test]
    fn as_contained_kind() {
        // Legacy state values do not have metadata.
        let versioned_legacy = DataRead::Versioned(
            Err(StorageVersion),
            Arc::new(ValueType::with_len_and_metadata(
                1,
                StateValueMetadata::none(),
            )),
            None,
        );
        let versioned_deletion = DataRead::Versioned(
            Ok((5, 1)),
            Arc::new(ValueType::with_len_and_metadata(
                0,
                StateValueMetadata::none(),
            )),
            None,
        );
        let versioned_with_metadata = DataRead::Versioned(
            Ok((7, 0)),
            Arc::new(ValueType::with_len_and_metadata(2, raw_metadata(1))),
            None,
        );
        let resolved = DataRead::Resolved::<ValueType>(200);
        let deletion_metadata = DataRead::Metadata(None);
        let legacy_metadata = DataRead::Metadata(Some(StateValueMetadata::none()));
        let metadata = DataRead::Metadata(Some(raw_metadata(1)));
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
        assert!(data_read_equals(
            &versioned_legacy,
            &DataRead::Versioned(
                Err(StorageVersion),
                Arc::new(ValueType::with_len_and_metadata(
                    10,
                    StateValueMetadata::none()
                )),
                None,
            ),
            false
        ));
    }

    #[derive(Clone, Debug)]
    struct TestTransactionType {}

    impl Transaction for TestTransactionType {
        type Event = MockEvent;
        type Key = KeyType<u32>;
        type Tag = u32;
        type Value = ValueType;

        fn user_txn_bytes_len(&self) -> usize {
            0
        }
    }

    macro_rules! assert_update_incorrect_use {
        ($m:expr, $x:expr, $y:expr) => {{
            let original = $m.get(&$x).cloned().unwrap();
            assert_matches!(
                CapturedReads::<
                    TestTransactionType,
                    u32,
                    MockDeserializedCode,
                    MockVerifiedCode,
                    MockExtension,
                >::update_entry($m.entry($x), $y.clone()),
                UpdateResult::IncorrectUse(_)
            );
            assert!(data_read_equals(&$m.get(&$x).unwrap(), &original, false));
        }};
    }

    macro_rules! assert_update_inconsistency {
        ($m:expr, $x:expr, $y:expr) => {{
            let original = $m.get(&$x).cloned().unwrap();
            assert_matches!(
                CapturedReads::<
                    TestTransactionType,
                    u32,
                    MockDeserializedCode,
                    MockVerifiedCode,
                    MockExtension,
                >::update_entry($m.entry($x), $y.clone()),
                UpdateResult::Inconsistency(_)
            );
            assert!(data_read_equals(&$m.get(&$x).unwrap(), &original, false));
        }};
    }

    macro_rules! assert_update {
        ($m:expr, $x:expr, $y:expr) => {{
            assert_matches!(
                CapturedReads::<
                    TestTransactionType,
                    u32,
                    MockDeserializedCode,
                    MockVerifiedCode,
                    MockExtension,
                >::update_entry($m.entry($x), $y.clone()),
                UpdateResult::Updated
            );
            assert!(data_read_equals(&$m.get(&$x).unwrap(), &$y, false));
        }};
    }

    macro_rules! assert_insert {
        ($m:expr, $x:expr, $y:expr) => {{
            assert_matches!(
                CapturedReads::<
                    TestTransactionType,
                    u32,
                    MockDeserializedCode,
                    MockVerifiedCode,
                    MockExtension,
                >::update_entry($m.entry($x), $y.clone()),
                UpdateResult::Inserted
            );
            assert!(data_read_equals(&$m.get(&$x).unwrap(), &$y, false));
        }};
    }

    #[test]
    fn test_update_entry() {
        // Legacy state values do not have metadata.
        let versioned_legacy = DataRead::Versioned(
            Err(StorageVersion),
            Arc::new(ValueType::with_len_and_metadata(
                1,
                StateValueMetadata::none(),
            )),
            None,
        );
        let versioned_deletion = DataRead::Versioned(
            Ok((5, 1)),
            Arc::new(ValueType::with_len_and_metadata(
                0,
                StateValueMetadata::none(),
            )),
            None,
        );
        let versioned_with_metadata = DataRead::Versioned(
            Ok((7, 0)),
            Arc::new(ValueType::with_len_and_metadata(2, raw_metadata(1))),
            None,
        );
        let resolved = DataRead::Resolved::<ValueType>(200);
        let deletion_metadata = DataRead::Metadata(None);
        let legacy_metadata = DataRead::Metadata(Some(StateValueMetadata::none()));
        let metadata = DataRead::Metadata(Some(raw_metadata(1)));
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
        let legacy_metadata = DataRead::Metadata(Some(StateValueMetadata::none()));
        let versioned_legacy = DataRead::Versioned(
            Err(StorageVersion),
            Arc::new(ValueType::with_len_and_metadata(
                1,
                StateValueMetadata::none(),
            )),
            None,
        );
        vec![exists, legacy_metadata, versioned_legacy]
    }

    fn deletion_reads_by_kind() -> Vec<DataRead<ValueType>> {
        let versioned_deletion = DataRead::Versioned(
            Ok((5, 1)),
            Arc::new(ValueType::with_len_and_metadata(
                0,
                StateValueMetadata::none(),
            )),
            None,
        );
        let deletion_metadata = DataRead::Metadata(None);
        let not_exists = DataRead::Exists(false);
        vec![not_exists, deletion_metadata, versioned_deletion]
    }

    fn with_metadata_reads_by_kind() -> Vec<DataRead<ValueType>> {
        let versioned_with_metadata = DataRead::Versioned(
            Ok((7, 0)),
            Arc::new(ValueType::with_len_and_metadata(2, raw_metadata(1))),
            None,
        );
        let metadata = DataRead::Metadata(Some(raw_metadata(1)));
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
                        assert!(data_read_equals(
                            &$x.get_by_kind(&$k, $mt.as_ref(), read_kinds[j].clone())
                                .unwrap(),
                            &$y[j],
                            false
                        ));
                        //assert_some_eq!(
                        //    $x.get_by_kind(&$k, $mt.as_ref(), read_kinds[j].clone()),
                        //    $y[j]
                        //);
                    }
                }
            }
        }};
    }

    #[test_case(false)]
    #[test_case(true)]
    fn capture_and_get_by_kind(use_tag: bool) {
        let mut captured_reads = CapturedReads::<
            TestTransactionType,
            u32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new(None);
        let legacy_reads = legacy_reads_by_kind();
        let deletion_reads = deletion_reads_by_kind();
        let with_metadata_reads = with_metadata_reads_by_kind();

        assert_capture_get!(
            captured_reads,
            KeyType::<u32>(10),
            use_tag.then_some(30),
            legacy_reads
        );
        assert_capture_get!(
            captured_reads,
            KeyType::<u32>(11),
            use_tag.then_some(30),
            deletion_reads
        );
        assert_capture_get!(
            captured_reads,
            KeyType::<u32>(15),
            use_tag.then_some(30),
            with_metadata_reads
        );
    }

    #[should_panic]
    #[test]
    fn metadata_for_group_member() {
        let captured_reads = CapturedReads::<
            TestTransactionType,
            u32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new(None);
        captured_reads.get_by_kind(&KeyType::<u32>(21), Some(&10), ReadKind::Metadata);
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
        let mut captured_reads = CapturedReads::<
            TestTransactionType,
            u32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new(None);
        let legacy_reads = legacy_reads_by_kind();
        let deletion_reads = deletion_reads_by_kind();
        let with_metadata_reads = with_metadata_reads_by_kind();

        let resolved = DataRead::Resolved::<ValueType>(200);
        let mixed_reads = [
            deletion_reads[0].clone(),
            with_metadata_reads[1].clone(),
            resolved,
        ];

        assert_incorrect_use!(
            captured_reads,
            KeyType::<u32>(10),
            use_tag.then_some(30),
            legacy_reads
        );
        assert_incorrect_use!(
            captured_reads,
            KeyType::<u32>(11),
            use_tag.then_some(30),
            deletion_reads
        );
        assert_incorrect_use!(
            captured_reads,
            KeyType::<u32>(15),
            use_tag.then_some(30),
            with_metadata_reads
        );

        // Test incorrect with with incompatible types.
        assert!(!captured_reads.incorrect_use);

        for i in 0..3 {
            let key = KeyType::<u32>(20 + i);
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
        let mut captured_reads = CapturedReads::<
            TestTransactionType,
            u32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new(None);
        let versioned_legacy = DataRead::Versioned(
            Err(StorageVersion),
            Arc::new(ValueType::with_len_and_metadata(
                1,
                StateValueMetadata::none(),
            )),
            None,
        );
        let resolved = DataRead::Resolved::<ValueType>(200);
        let metadata = DataRead::Metadata(Some(raw_metadata(1)));
        let deletion_metadata = DataRead::Metadata(None);
        let exists = DataRead::Exists(true);

        assert!(!captured_reads.non_delayed_field_speculative_failure);
        assert!(!captured_reads.delayed_field_speculative_failure);
        let key = KeyType::<u32>(20);
        assert_ok!(captured_reads.capture_read(key, use_tag.then_some(30), exists));
        assert_err!(captured_reads.capture_read(
            key,
            use_tag.then_some(30),
            deletion_metadata.clone()
        ));
        assert!(captured_reads.non_delayed_field_speculative_failure);
        assert!(!captured_reads.delayed_field_speculative_failure);

        let mvhashmap = MVHashMap::<KeyType<u32>, u32, ValueType, DelayedFieldID>::new();

        captured_reads.non_delayed_field_speculative_failure = false;
        captured_reads.delayed_field_speculative_failure = false;
        let key = KeyType::<u32>(21);
        assert_ok!(captured_reads.capture_read(key, use_tag.then_some(30), deletion_metadata));
        assert_err!(captured_reads.capture_read(key, use_tag.then_some(30), resolved));
        assert!(captured_reads.non_delayed_field_speculative_failure);
        assert!(!captured_reads.validate_data_reads(mvhashmap.data(), 0));
        assert!(!captured_reads.validate_group_reads(mvhashmap.group_data(), 0));
        assert!(!captured_reads.delayed_field_speculative_failure);
        assert_ok_eq!(
            captured_reads.validate_delayed_field_reads(mvhashmap.delayed_fields(), 0),
            true
        );

        captured_reads.non_delayed_field_speculative_failure = false;
        captured_reads.delayed_field_speculative_failure = false;
        let key = KeyType::<u32>(22);
        assert_ok!(captured_reads.capture_read(key, use_tag.then_some(30), metadata));
        assert_err!(captured_reads.capture_read(key, use_tag.then_some(30), versioned_legacy));
        assert!(captured_reads.non_delayed_field_speculative_failure);
        assert!(!captured_reads.delayed_field_speculative_failure);

        let mut captured_reads = CapturedReads::<
            TestTransactionType,
            u32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new(None);
        captured_reads.non_delayed_field_speculative_failure = false;
        captured_reads.delayed_field_speculative_failure = false;
        captured_reads.mark_failure(true);
        assert!(!captured_reads.non_delayed_field_speculative_failure);
        assert!(captured_reads.validate_data_reads(mvhashmap.data(), 0));
        assert!(captured_reads.validate_group_reads(mvhashmap.group_data(), 0));
        assert!(captured_reads.delayed_field_speculative_failure);
        assert_ok_eq!(
            captured_reads.validate_delayed_field_reads(mvhashmap.delayed_fields(), 0),
            false
        );

        captured_reads.mark_failure(true);
        assert!(!captured_reads.non_delayed_field_speculative_failure);
        assert!(captured_reads.delayed_field_speculative_failure);

        captured_reads.delayed_field_speculative_failure = false;
        captured_reads.mark_failure(false);
        assert!(captured_reads.non_delayed_field_speculative_failure);
        assert!(!captured_reads.delayed_field_speculative_failure);
        captured_reads.mark_failure(true);
        assert!(captured_reads.non_delayed_field_speculative_failure);
        assert!(captured_reads.delayed_field_speculative_failure);
    }

    #[test]
    fn test_speculative_failure_for_module_reads() {
        let mut captured_reads = CapturedReads::<
            TestTransactionType,
            u32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new(None);
        let global_module_cache = GlobalModuleCache::empty();
        let per_block_module_cache = SyncModuleCache::empty();

        assert!(captured_reads.validate_module_reads(&global_module_cache, &per_block_module_cache, None));
        captured_reads.mark_failure(true);
        assert!(captured_reads.validate_module_reads(&global_module_cache, &per_block_module_cache, None));
        captured_reads.mark_failure(false);
        assert!(
            !captured_reads.validate_module_reads(&global_module_cache, &per_block_module_cache, None)
        );
    }

    #[test]
    fn test_global_cache_module_reads() {
        let mut captured_reads = CapturedReads::<
            TestTransactionType,
            u32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new(None);
        let mut global_module_cache = GlobalModuleCache::empty();
        let per_block_module_cache = SyncModuleCache::empty();

        let module_0 = mock_verified_code(0, MockExtension::new(8));
        global_module_cache.insert(0, module_0.clone());
        captured_reads.capture_global_cache_read(0, module_0);

        let module_1 = mock_verified_code(1, MockExtension::new(8));
        global_module_cache.insert(1, module_1.clone());
        captured_reads.capture_global_cache_read(1, module_1);

        assert!(captured_reads.validate_module_reads(&global_module_cache, &per_block_module_cache, None));

        // Now, mark one of the entries in invalid. Validations should fail!
        global_module_cache.mark_overridden(&1);
        let valid =
            captured_reads.validate_module_reads(&global_module_cache, &per_block_module_cache, None);
        assert!(!valid);

        // Without invalid module (and if it is not captured), validation should pass.
        assert!(global_module_cache.remove(&1));
        captured_reads.module_reads.remove(&1);
        assert!(captured_reads.validate_module_reads(&global_module_cache, &per_block_module_cache, None));

        // Validation fails if we captured a cross-block module which does not exist anymore.
        assert!(global_module_cache.remove(&0));
        let valid =
            captured_reads.validate_module_reads(&global_module_cache, &per_block_module_cache, None);
        assert!(!valid);
    }

    #[test]
    fn test_block_cache_module_reads_are_recorded() {
        let mut captured_reads = CapturedReads::<
            TestTransactionType,
            u32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new(None);
        let per_block_module_cache: SyncModuleCache<u32, _, MockVerifiedCode, _, _> =
            SyncModuleCache::empty();

        let a = mock_deserialized_code(0, MockExtension::new(8));
        per_block_module_cache
            .insert_deserialized_module(
                0,
                a.code().deserialized().as_ref().clone(),
                a.extension().clone(),
                Some(2),
            )
            .unwrap();
        captured_reads.capture_per_block_cache_read(0, Some((a, Some(2))));
        assert!(matches!(
            captured_reads.get_module_read(&0),
            CacheRead::Hit(Some(_))
        ));

        captured_reads.capture_per_block_cache_read(1, None);
        assert!(matches!(
            captured_reads.get_module_read(&1),
            CacheRead::Hit(None)
        ));

        assert!(matches!(
            captured_reads.get_module_read(&2),
            CacheRead::Miss
        ));
    }

    #[test]
    fn test_block_cache_module_reads() {
        let mut captured_reads = CapturedReads::<
            TestTransactionType,
            u32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new(None);
        let global_module_cache = GlobalModuleCache::empty();
        let per_block_module_cache = SyncModuleCache::empty();

        let a = mock_deserialized_code(0, MockExtension::new(8));
        per_block_module_cache
            .insert_deserialized_module(
                0,
                a.code().deserialized().as_ref().clone(),
                a.extension().clone(),
                Some(10),
            )
            .unwrap();
        captured_reads.capture_per_block_cache_read(0, Some((a, Some(10))));
        captured_reads.capture_per_block_cache_read(1, None);

        assert!(captured_reads.validate_module_reads(&global_module_cache, &per_block_module_cache, None));

        let b = mock_deserialized_code(1, MockExtension::new(8));
        per_block_module_cache
            .insert_deserialized_module(
                1,
                b.code().deserialized().as_ref().clone(),
                b.extension().clone(),
                Some(12),
            )
            .unwrap();

        // Entry did not exist before and now exists.
        let valid =
            captured_reads.validate_module_reads(&global_module_cache, &per_block_module_cache, None);
        assert!(!valid);

        captured_reads.module_reads.remove(&1);
        assert!(captured_reads.validate_module_reads(&global_module_cache, &per_block_module_cache, None));

        // Version has been republished, with a higher transaction index. Should fail validation.
        let a = mock_deserialized_code(0, MockExtension::new(8));
        per_block_module_cache
            .insert_deserialized_module(
                0,
                a.code().deserialized().as_ref().clone(),
                a.extension().clone(),
                Some(20),
            )
            .unwrap();

        let valid =
            captured_reads.validate_module_reads(&global_module_cache, &per_block_module_cache, None);
        assert!(!valid);
    }

    #[test]
    fn test_global_and_block_cache_module_reads() {
        let mut captured_reads = CapturedReads::<
            TestTransactionType,
            u32,
            MockDeserializedCode,
            MockVerifiedCode,
            MockExtension,
        >::new(None);
        let mut global_module_cache = GlobalModuleCache::empty();
        let per_block_module_cache = SyncModuleCache::empty();

        // Module exists in global cache.
        let m = mock_verified_code(0, MockExtension::new(8));
        global_module_cache.insert(0, m.clone());
        captured_reads.capture_global_cache_read(0, m);
        assert!(captured_reads.validate_module_reads(&global_module_cache, &per_block_module_cache, None));

        // Assume we republish this module: validation must fail.
        let a = mock_deserialized_code(100, MockExtension::new(8));
        global_module_cache.mark_overridden(&0);
        per_block_module_cache
            .insert_deserialized_module(
                0,
                a.code().deserialized().as_ref().clone(),
                a.extension().clone(),
                Some(10),
            )
            .unwrap();

        let valid =
            captured_reads.validate_module_reads(&global_module_cache, &per_block_module_cache, None);
        assert!(!valid);

        // Assume we re-read the new correct version. Then validation should pass again.
        captured_reads.capture_per_block_cache_read(0, Some((a, Some(10))));
        assert!(captured_reads.validate_module_reads(&global_module_cache, &per_block_module_cache, None));
        assert!(!global_module_cache.contains_not_overridden(&0));
    }
}
