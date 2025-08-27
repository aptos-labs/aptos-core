// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::types::delayed_field_mock_serialization::serialize_delayed_field_tuple;
use aptos_aggregator::delta_change_set::{delta_add, delta_sub, serialize, DeltaOp};
use aptos_crypto::HashValue;
use aptos_types::{
    account_address::AccountAddress,
    contract_event::TransactionEvent,
    executable::ModulePath,
    on_chain_config::CurrentTimeMicroseconds,
    state_store::{
        errors::StateViewError,
        state_storage_usage::StateStorageUsage,
        state_value::{StateValue, StateValueMetadata},
        StateViewId, TStateView,
    },
    transaction::BlockExecutableTransaction as Transaction,
    write_set::{TransactionWrite, WriteOpKind},
};
use aptos_vm_types::module_write_set::ModuleWrite;
use bytes::Bytes;
use claims::{assert_ge, assert_le};
use move_core_types::{
    identifier::{IdentStr, Identifier},
    language_storage::ModuleId,
};
use move_vm_runtime::Module;
use proptest::{arbitrary::Arbitrary, collection::vec, prelude::*, proptest, sample::Index};
use proptest_derive::Arbitrary;
use std::{
    collections::{hash_map::DefaultHasher, BTreeMap, BTreeSet, HashMap, HashSet},
    fmt::Debug,
    hash::{Hash, Hasher},
    marker::PhantomData,
    sync::{atomic::AtomicUsize, Arc},
};

// Should not be possible to overflow or underflow, as each delta is at most 100 in the tests.
// TODO: extend to delta failures.
pub(crate) const STORAGE_AGGREGATOR_VALUE: u128 = 100001;
pub(crate) const MAX_GAS_PER_TXN: u64 = 4;
// For some resource group tests we ensure that the groups are never empty because they contain
// a value at RESERVED_TAG (starting from mock storage resolution) that is never deleted.
pub(crate) const RESERVED_TAG: u32 = 0;

pub(crate) struct DeltaDataView<K> {
    pub(crate) initial_values: HashMap<K, u128>,
    pub(crate) default_base_value: u128,
    pub(crate) phantom: PhantomData<K>,
}

impl<K> TStateView for DeltaDataView<K>
where
    K: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + 'static,
{
    type Key = K;

    // Contains mock storage value with STORAGE_AGGREGATOR_VALUE.
    fn get_state_value(&self, key: &K) -> Result<Option<StateValue>, StateViewError> {
        let value = self
            .initial_values
            .get(key)
            .unwrap_or(&self.default_base_value);
        Ok(Some(StateValue::new_legacy(serialize(value).into())))
    }

    fn id(&self) -> StateViewId {
        StateViewId::Miscellaneous
    }

    fn get_usage(&self) -> Result<StateStorageUsage, StateViewError> {
        unreachable!("Not used in tests");
    }
}

pub(crate) struct NonEmptyGroupDataView<K> {
    pub(crate) group_keys_with_delta_tags: HashMap<K, Vec<u32>>,
    // When we are testing with delayed fields, currently deletion is not supported,
    // so we need to return for each key that can contain a delayed field. for groups,
    // the reserved tag is the only such key, and we simply return a value for all
    // non-group keys to ensure the test runs.
    pub(crate) delayed_field_testing: bool,
}

pub(crate) fn default_group_map(tags_with_deltas: &[DeltaHoldingTag]) -> BTreeMap<u32, Bytes> {
    let mut map = BTreeMap::new();
    for tag_config in tags_with_deltas {
        let bytes: Bytes = bcs::to_bytes(&(
            tag_config.base_value,
            // u32::MAX represents storage version.
            u32::MAX,
        ))
        .unwrap()
        .into();
        map.insert(tag_config.tag, bytes);
    }
    map
}

impl<K> TStateView for NonEmptyGroupDataView<K>
where
    K: Debug + PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + 'static,
{
    type Key = K;

    // Contains mock storage value with a non-empty group (w. value at RESERVED_TAG).
    fn get_state_value(&self, key: &K) -> Result<Option<StateValue>, StateViewError> {
        Ok(self
            .group_keys_with_delta_tags
            .get(key)
            .map(|tags| {
                let bytes = bcs::to_bytes(&default_group_map(tags)).unwrap();
                StateValue::new_with_metadata(bytes.into(), raw_metadata(5))
            })
            .or_else(|| {
                self.delayed_field_testing.then(|| {
                    StateValue::new_legacy(serialize_delayed_field_tuple(&(
                        STORAGE_AGGREGATOR_VALUE,
                        // u32::MAX represents storage version.
                        u32::MAX,
                    )))
                })
            }))
    }

    fn id(&self) -> StateViewId {
        StateViewId::Miscellaneous
    }

    fn get_usage(&self) -> Result<StateStorageUsage, StateViewError> {
        unreachable!("Not used in tests");
    }
}

///////////////////////////////////////////////////////////////////////////
// Generation of transactions
///////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Hash, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub(crate) struct KeyType<K: Hash + Clone + Debug + PartialOrd + Ord + Eq>(
    /// Wrapping the types used for testing to add ModulePath trait implementation (below).
    pub K,
);

impl<K: Hash + Clone + Debug + Eq + PartialOrd + Ord> ModulePath for KeyType<K> {
    fn is_module_path(&self) -> bool {
        false
    }

    fn from_address_and_module_name(_address: &AccountAddress, _module_name: &IdentStr) -> Self {
        unimplemented!()
    }
}

// TODO: this is now very similar to WriteOp, should be a wrapper and remove boilerplate below.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ValueType {
    /// Wrapping the types used for testing to add TransactionWrite trait implementation (below).
    bytes: Option<Bytes>,
    metadata: StateValueMetadata,
    write_op_kind: WriteOpKind,
}

impl Clone for ValueType {
    fn clone(&self) -> Self {
        ValueType::new(
            self.bytes.clone(),
            self.metadata.clone(),
            self.write_op_kind(),
        )
    }
}

impl ValueType {
    pub(crate) fn new(
        bytes: Option<Bytes>,
        metadata: StateValueMetadata,
        kind: WriteOpKind,
    ) -> Self {
        Self {
            bytes,
            metadata,
            write_op_kind: kind,
        }
    }

    /// If use_value is true, we use WriteOpKind::Creation by default, o.w. Deletion.
    /// For resource groups, mock executor updates the WriteOp kind to avoid the consistency
    /// check with existence (not checked for normal resources, storage asserts).
    pub(crate) fn from_value<V: Into<Vec<u8>> + Debug + Clone + Eq + Send + Sync + Arbitrary>(
        value: V,
        use_value: bool,
    ) -> Self {
        Self {
            bytes: use_value.then(|| {
                let mut v = value.clone().into();
                v.resize(16, 1);
                v.into()
            }),
            metadata: StateValueMetadata::none(),
            write_op_kind: if !use_value {
                WriteOpKind::Deletion
            } else {
                WriteOpKind::Creation
            },
        }
    }

    /// If len = 0, treated as Deletion for testing.
    pub(crate) fn with_len_and_metadata(len: usize, metadata: StateValueMetadata) -> Self {
        Self {
            bytes: (len > 0).then_some(vec![100_u8; len].into()),
            metadata,
            write_op_kind: if len == 0 {
                WriteOpKind::Deletion
            } else {
                WriteOpKind::Creation
            },
        }
    }
}

impl TransactionWrite for ValueType {
    fn bytes(&self) -> Option<&Bytes> {
        self.bytes.as_ref()
    }

    fn from_state_value(maybe_state_value: Option<StateValue>) -> Self {
        let (maybe_metadata, maybe_bytes) =
            match maybe_state_value.map(|state_value| state_value.unpack()) {
                Some((maybe_metadata, bytes)) => (maybe_metadata, Some(bytes)),
                None => (StateValueMetadata::none(), None),
            };

        let empty = maybe_bytes.is_none();

        Self {
            bytes: maybe_bytes,
            metadata: maybe_metadata,
            write_op_kind: if empty {
                WriteOpKind::Deletion
            } else {
                WriteOpKind::Creation
            },
        }
    }

    fn write_op_kind(&self) -> WriteOpKind {
        self.write_op_kind.clone()
    }

    fn as_state_value(&self) -> Option<StateValue> {
        self.extract_raw_bytes()
            .map(|bytes| StateValue::new_with_metadata(bytes, self.metadata.clone()))
    }

    fn set_bytes(&mut self, bytes: Bytes) {
        self.bytes = Some(bytes);
    }
}

///////////////////////////////////////////////////////////////////////////
// Generation of transactions
///////////////////////////////////////////////////////////////////////////

/// The following structs are used to configure and generate transactions for proptests.
/// The goal is to move away from imperative logic (e.g. using modulo operators on random
/// values to determine operation types) to a more declarative, strategy-based approach.
///
/// 1. `TransactionGenParams`: Top-level parameters for a test run, including weights
///    for different types of modifications.
/// 2. `ModificationWeights`: A struct to hold the weights for writes, deletions, and deltas,
///    allowing runtime configuration of the operation mix.
/// 3. `Modification<V>`: An enum that semantically represents the intended operation. `proptest`
///    will generate this directly based on the weights in `ModificationWeights`.
/// 4. `TransactionGenData<V>`: A container for the raw data generated by `proptest`, which serves
///    as input to the `MockTransactionBuilder`.
/// 5. `MockTransactionBuilder`: A builder that uses a fluent API to configure and construct
///    a `MockTransaction` from `TransactionGenData`, replacing the old `materialize_*` functions.

#[derive(Clone, Copy, Debug)]
pub(crate) struct ModificationWeights {
    pub write: u32,
    pub deletion: u32,
    pub delta: u32,
}

impl Default for ModificationWeights {
    fn default() -> Self {
        Self {
            write: 8,
            deletion: 1,
            delta: 4,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct TransactionGenParams {
    /// Each transaction's read-set consists of between 1 and read_size many reads.
    pub(crate) read_size: usize,
    /// Each mock execution will produce between 1 and output_size many writes and deltas.
    pub(crate) output_size: usize,
    /// The number of different incarnation behaviors that a mock execution of the transaction
    /// may exhibit.
    pub(crate) incarnation_alternatives: usize,
    /// Weights for generating different types of modifications.
    pub(crate) modification_weights: ModificationWeights,
}

/// A semantic representation of a data modification, generated by `proptest` according
/// to configured weights. This avoids interpreting raw random values with brittle logic.
#[derive(Debug, Clone)]
pub(crate) enum Modification<V> {
    Write(Index, V),
    Deletion(Index),
    Delta(Index, DeltaOp),
}

/// A container for the raw data generated by `proptest`. This struct is the input
/// to the `MockTransactionBuilder`.
#[derive(Debug, Clone)]
pub(crate) struct TransactionGenData<V> {
    /// Generate keys for possible read-sets of the transaction.
    reads: Vec<Vec<Index>>,
    /// Generate a semantic description of modifications for the transaction.
    modifications: Vec<Vec<Modification<V>>>,
    /// Generate gas for different incarnations of the transactions.
    gas: Vec<Index>,
    /// Generate seeds for group metadata.
    metadata_seeds: Vec<Vec<Index>>,
    /// Generate indices to derive random behavior for querying resource group sizes.
    group_size_indicators: Vec<Vec<Index>>,
}

impl<V> Arbitrary for TransactionGenData<V>
where
    V: Into<Vec<u8>> + Arbitrary + Clone + Debug + Eq + 'static,
{
    type Parameters = TransactionGenParams;
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(params: Self::Parameters) -> Self::Strategy {
        let weights = params.modification_weights;

        // Use prop_oneof! to create a dynamic strategy based on runtime weights.
        let modification_strategy = prop_oneof![
            weights.write => (any::<Index>(), any::<V>()).prop_map(|(i, v)| Modification::Write(i, v)),
            weights.deletion => any::<Index>().prop_map(Modification::Deletion),
            weights.delta => (any::<Index>(), any::<u128>()).prop_map(|(i, v)| {
                // This logic is preserved from the original implementation to maintain
                // existing test characteristics, but is now contained in a more
                // structured way.
                let magnitude = v % 100;
                let op = if v % 10 < 5 {
                    delta_sub(magnitude, u128::MAX)
                } else {
                    delta_add(magnitude, u128::MAX)
                };
                Modification::Delta(i, op)
            }),
        ];

        (
            vec(
                vec(any::<Index>(), 1..=params.read_size),
                params.incarnation_alternatives,
            ),
            vec(
                vec(modification_strategy, 1..=params.output_size),
                params.incarnation_alternatives,
            ),
            vec(any::<Index>(), params.incarnation_alternatives),
            vec(vec(any::<Index>(), 3), params.incarnation_alternatives),
            // To maintain behavior but allow flexibility, we generate a vec of 3 indices.
            // The consuming logic can handle a variable number of groups.
            vec(
                vec(any::<Index>(), 3),
                params.incarnation_alternatives,
            ),
        )
            .prop_map(
                |(reads, modifications, gas, metadata_seeds, group_size_indicators)| Self {
                    reads,
                    modifications,
                    gas,
                    metadata_seeds,
                    group_size_indicators,
                },
            )
            .boxed()
    }
}

/// Describes behavior of a particular incarnation of a mock transaction, as keys to be read,
/// as well as writes, deltas and total execution gas charged for this incarnation. Note that
/// writes, deltas and gas become part of the output directly (as part of the mock execution of
/// a given incarnation), so the output of an incarnation does not depend on the values read, which
/// is a limitation for the testing framework. However, IncarnationBehavior allows different
/// behaviors to be exhibited by different incarnations during parallel execution, which happens
/// first and also records the latest incarnations of each transaction (that is committed).
/// Then we can generate the baseline by sequentially executing the behavior prescribed for
/// those latest incarnations.
///
/// TODO(BlockSTMv2): Mock incarnation & behavior generation should also be separated out
/// and refactored into e.g. a builder pattern. In particular, certain materialization methods
/// transform generated resource reads and writes into group or module reads and writes.
/// It would be more natural to maintain an internal builder state of the mock transaction
/// generation process and then finalize it into the desired format. Additionally, the
/// internal fields should contain structs instead of less readable tuples.
#[derive(Clone, Debug)]
pub(crate) struct MockIncarnation<K, E> {
    /// A vector of keys to be read during mock incarnation execution.
    /// bool indicates that the path contains deltas, i.e. AggregatorV1 or DelayedFields.
    pub(crate) resource_reads: Vec<(K, bool)>,
    /// A vector of keys and corresponding values to be written during mock incarnation execution.
    /// bool indicates that the path contains deltas, i.e. AggregatorV1 or DelayedFields.
    pub(crate) resource_writes: Vec<(K, ValueType, bool)>,
    pub(crate) group_reads: Vec<(K, u32, bool)>,
    pub(crate) group_writes: Vec<(K, StateValueMetadata, HashMap<u32, (ValueType, bool)>)>,
    // For testing get_module_or_build_with and insert_verified_module interfaces.
    pub(crate) module_reads: Vec<ModuleId>,
    pub(crate) module_writes: Vec<ModuleWrite<ValueType>>,
    /// Keys to query group size for - false is querying size, true is querying metadata.
    pub(crate) group_queries: Vec<(K, bool)>,
    /// A vector of keys and corresponding deltas to be produced during mock incarnation
    /// execution. For delayed fields in groups, the Option is set to Some(tag).
    pub(crate) deltas: Vec<(K, DeltaOp, Option<u32>)>,
    /// A vector of events.
    pub(crate) events: Vec<E>,
    metadata_seeds: [u64; 3],
    /// total execution gas to be charged for mock incarnation execution.
    pub(crate) gas: u64,
}

impl<K, E> MockIncarnation<K, E> {
    /// Group writes are derived from normal transaction behavior, transforming one MockIncarnation
    /// into another one with group_reads / group_writes / group_queries set. Hence, the constructor
    /// here always sets it to an empty vector.
    pub(crate) fn new_with_metadata_seeds(
        resource_reads: Vec<(K, bool)>,
        resource_writes: Vec<(K, ValueType, bool)>,
        deltas: Vec<(K, DeltaOp, Option<u32>)>,
        events: Vec<E>,
        metadata_seeds: [u64; 3],
        gas: u64,
    ) -> Self {
        Self {
            resource_reads,
            resource_writes,
            group_reads: vec![],
            group_writes: vec![],
            group_queries: vec![],
            module_reads: vec![],
            module_writes: vec![],
            deltas,
            events,
            metadata_seeds,
            gas,
        }
    }

    pub(crate) fn new(
        resource_reads: Vec<(K, bool)>,
        resource_writes: Vec<(K, ValueType, bool)>,
        deltas: Vec<(K, DeltaOp, Option<u32>)>,
        events: Vec<E>,
        gas: u64,
    ) -> Self {
        Self {
            resource_reads,
            resource_writes,
            group_reads: vec![],
            group_writes: vec![],
            group_queries: vec![],
            module_reads: vec![],
            module_writes: vec![],
            deltas,
            events,
            metadata_seeds: [0; 3],
            gas,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DeltaTestKind {
    DelayedFields,
    AggregatorV1,
    None,
}

/// A mock transaction that could be used to test the correctness and throughput of the system.
/// To test transaction behavior where reads and writes might be dynamic (depend on previously
/// read values), different read and writes sets are generated and used depending on the incarnation
/// counter value. Each execution of the transaction increments the incarnation counter, and its
/// value determines the index for choosing the read & write sets of the particular execution.
#[derive(Clone, Debug)]
pub(crate) enum MockTransaction<K, E> {
    InterruptRequested,
    Write {
        /// Incarnation counter, increased during each mock (re-)execution. Allows tracking the final
        /// incarnation for each mock transaction, whose behavior should be reproduced for baseline.
        /// Arc-ed only due to Clone, TODO: clean up the Clone requirement.
        incarnation_counter: Arc<AtomicUsize>,
        /// A vector of mock behaviors prescribed for each incarnation of the transaction, chosen
        /// round robin depending on the incarnation counter value).
        incarnation_behaviors: Vec<MockIncarnation<K, E>>,
        /// If we are testing with deltas, are we testing delayed_fields? (or AggregatorV1).
        delta_test_kind: DeltaTestKind,
    },
    /// Skip the execution of trailing transactions.
    SkipRest(u64),
    /// Abort the execution.
    Abort,
    StateCheckpoint,
}

impl<K, E> MockTransaction<K, E> {
    pub(crate) fn from_behavior(behavior: MockIncarnation<K, E>) -> Self {
        Self::Write {
            incarnation_counter: Arc::new(AtomicUsize::new(0)),
            incarnation_behaviors: vec![behavior],
            delta_test_kind: DeltaTestKind::None,
        }
    }

    pub(crate) fn from_behaviors(behaviors: Vec<MockIncarnation<K, E>>) -> Self {
        Self::Write {
            incarnation_counter: Arc::new(AtomicUsize::new(0)),
            incarnation_behaviors: behaviors,
            delta_test_kind: DeltaTestKind::None,
        }
    }

    pub(crate) fn with_delayed_fields_testing(mut self) -> Self {
        if let Self::Write {
            delta_test_kind, ..
        } = &mut self
        {
            *delta_test_kind = DeltaTestKind::DelayedFields;
        }
        self
    }

    pub(crate) fn with_aggregator_v1_testing(mut self) -> Self {
        if let Self::Write {
            delta_test_kind, ..
        } = &mut self
        {
            *delta_test_kind = DeltaTestKind::AggregatorV1;
        }
        self
    }

    pub(crate) fn into_behaviors(self) -> Vec<MockIncarnation<K, E>> {
        match self {
            Self::Write {
                incarnation_behaviors,
                ..
            } => incarnation_behaviors,
            Self::SkipRest(_) => unreachable!("SkipRest does not contain incarnation behaviors"),
            Self::Abort => unreachable!("Abort does not contain incarnation behaviors"),
            Self::InterruptRequested => {
                unreachable!("InterruptRequested does not contain incarnation behaviors")
            },
            Self::StateCheckpoint => {
                unreachable!("StateCheckpoint does not contain incarnation behaviors")
            },
        }
    }
}

impl<
        K: Debug + Hash + Ord + Clone + Send + Sync + ModulePath + 'static,
        E: Debug + Clone + Send + Sync + TransactionEvent + 'static,
    > Transaction for MockTransaction<K, E>
{
    type Event = E;
    type Key = K;
    type Tag = u32;
    type Value = ValueType;

    fn user_txn_bytes_len(&self) -> usize {
        0
    }

    fn state_checkpoint(_block_id: HashValue) -> Self {
        Self::StateCheckpoint
    }
}

impl TransactionGenParams {
    pub fn new_dynamic() -> Self {
        Self {
            read_size: 10,
            output_size: 5,
            incarnation_alternatives: 5,
            modification_weights: ModificationWeights::default(),
        }
    }

    // The read and write will be converted to a module read and write.
    pub fn new_dynamic_modules_only() -> Self {
        Self {
            read_size: 1,
            output_size: 1,
            incarnation_alternatives: 5,
            modification_weights: ModificationWeights {
                write: 1,
                deletion: 0,
                delta: 0,
            },
        }
    }

    // Last read and write will be converted to module reads and writes.
    pub fn new_dynamic_with_modules() -> Self {
        Self {
            read_size: 3,
            output_size: 3,
            incarnation_alternatives: 5,
            modification_weights: ModificationWeights {
                write: 1,
                deletion: 0,
                delta: 0,
            },
        }
    }

    pub fn with_modification_weights(mut self, weights: ModificationWeights) -> Self {
        self.modification_weights = weights;
        self
    }

    pub fn with_no_deletions(mut self) -> Self {
        self.modification_weights.deletion = 0;
        self
    }
}

impl Default for TransactionGenParams {
    fn default() -> Self {
        Self {
            read_size: 10,
            output_size: 1,
            incarnation_alternatives: 1,
            modification_weights: ModificationWeights::default(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct DeltaHoldingTag {
    pub tag: u32,
    pub base_value: u128,
}

#[derive(Clone)]
pub(crate) struct PerGroupConfig<K> {
    pub key: K,
    pub query_percentage: Option<u8>,
    pub tags_with_deltas: Vec<DeltaHoldingTag>,
}

pub(crate) struct MockTransactionBuilder<'a, K, V> {
    gen_data: TransactionGenData<V>,
    universe: &'a [K],

    // Configuration flags that determine how the final transaction is constructed.
    materialize_as_modules: bool,
    group_config: Option<Vec<PerGroupConfig<K>>>,
    delta_test_kind: DeltaTestKind,
}

impl<'a, K, V> MockTransactionBuilder<'a, K, V>
where
    K: Clone + Hash + Debug + Eq + Ord,
    V: Into<Vec<u8>> + Arbitrary + Clone + Debug + Eq + Sync + Send,
{
    pub fn new(gen_data: TransactionGenData<V>, universe: &'a [K]) -> Self {
        Self {
            gen_data,
            universe,
            materialize_as_modules: false,
            group_config: None,
            delta_test_kind: DeltaTestKind::None,
        }
    }

    pub fn with_modules(mut self) -> Self {
        self.materialize_as_modules = true;
        self
    }

    pub fn with_groups(mut self, config: Vec<PerGroupConfig<K>>) -> Self {
        self.group_config = Some(config);
        self
    }

    pub fn with_deltas(mut self, kind: DeltaTestKind) -> Self {
        self.delta_test_kind = kind;
        self
    }

    pub fn build<E: Send + Sync + Debug + Clone + TransactionEvent>(
        self,
    ) -> MockTransaction<KeyType<K>, E> {
        let mut behaviors = self.generate_base_behaviors();

        if let Some(group_config) = self.group_config {
            self.transform_for_groups(&mut behaviors, &group_config);
        }

        if self.materialize_as_modules {
            self.transform_for_modules(&mut behaviors);
        }

        let mut txn = MockTransaction::from_behaviors(behaviors);
        match self.delta_test_kind {
            DeltaTestKind::DelayedFields => txn.with_delayed_fields_testing(),
            DeltaTestKind::AggregatorV1 => txn.with_aggregator_v1_testing(),
            DeltaTestKind::None => txn,
        }
    }

    fn generate_base_behaviors<E: Send + Sync + Debug + Clone + TransactionEvent>(
        &self,
    ) -> Vec<MockIncarnation<KeyType<K>, E>> {
        let is_delta_path = |key_idx: usize| {
            matches!(
                self.delta_test_kind,
                DeltaTestKind::AggregatorV1 | DeltaTestKind::DelayedFields
            ) && key_idx >= self.universe.len() / 2 // Simplified from delta_threshold
        };

        // 1. Generate reads
        let reads: Vec<Vec<(KeyType<K>, bool)>> = self
            .gen_data
            .reads
            .iter()
            .map(|read_gen| {
                read_gen
                    .iter()
                    .map(|idx| {
                        let i = idx.index(self.universe.len());
                        let key = self.universe[i].clone();
                        (KeyType(key), is_delta_path(i))
                    })
                    .collect()
            })
            .collect();

        // 2. Generate modifications (writes, deletions, deltas)
        let modifications: Vec<(Vec<(KeyType<K>, ValueType, bool)>, Vec<(KeyType<K>, DeltaOp)>)> =
            self.gen_data
                .modifications
                .iter()
                .map(|modification_gen| {
                    let mut keys_modified = BTreeSet::new();
                    let mut incarnation_writes = vec![];
                    let mut incarnation_deltas = vec![];

                    for modification in modification_gen {
                        match modification {
                            Modification::Write(idx, value) => {
                                let i = idx.index(self.universe.len());
                                let key = self.universe[i].clone();
                                if keys_modified.insert(key.clone()) {
                                    let val_u128 = ValueType::from_value(value.clone(), true)
                                        .as_u128()
                                        .unwrap()
                                        .unwrap();
                                    let mut write_value = ValueType::from_value(value.clone(), true);
                                    write_value.metadata = raw_metadata((val_u128 >> 64) as u64);
                                    incarnation_writes
                                        .push((KeyType(key), write_value, is_delta_path(i)));
                                }
                            },
                            Modification::Deletion(idx) => {
                                let i = idx.index(self.universe.len());
                                let key = self.universe[i].clone();
                                if keys_modified.insert(key.clone()) {
                                    let value = ValueType::new(None, StateValueMetadata::none(), WriteOpKind::Deletion);
                                    incarnation_writes.push((KeyType(key), value, is_delta_path(i)));
                                }
                            },
                            Modification::Delta(idx, op) => {
                                let i = idx.index(self.universe.len());
                                let key = self.universe[i].clone();
                                if keys_modified.insert(key.clone()) {
                                    // Deltas are only produced for paths that are configured for it.
                                    if is_delta_path(i) {
                                        incarnation_deltas.push((KeyType(key), *op));
                                    }
                                }
                            },
                        }
                    }
                    (incarnation_writes, incarnation_deltas)
                })
                .collect();

        // 3. Generate gas
        let gas: Vec<u64> = self
            .gen_data
            .gas
            .iter()
            .map(|idx| idx.index(MAX_GAS_PER_TXN as usize + 1) as u64)
            .collect();

        // 4. Generate metadata seeds
        let metadata_seeds: Vec<[u64; 3]> = self
            .gen_data
            .metadata_seeds
            .iter()
            .map(|vec| {
                [
                    vec[0].index(100000) as u64,
                    vec[1].index(100000) as u64,
                    vec[2].index(100000) as u64,
                ]
            })
            .collect();

        // 5. Combine into MockIncarnation behaviors
        modifications
            .into_iter()
            .zip(reads)
            .zip(gas)
            .zip(metadata_seeds)
            .map(|((((writes, deltas), reads), gas), metadata_seeds)| {
                MockIncarnation::new_with_metadata_seeds(
                    reads,
                    writes,
                    deltas
                        .into_iter()
                        .map(|(k, delta)| (k, delta, None))
                        .collect(),
                    vec![], // events
                    metadata_seeds,
                    gas,
                )
            })
            .collect()
    }

    fn transform_for_modules<E>(&self, behaviors: &mut [MockIncarnation<KeyType<K>, E>]) {
        let universe_len = self.universe.len();
        for behavior in behaviors.iter_mut() {
            if behavior.resource_writes.is_empty() || behavior.resource_reads.is_empty() {
                return;
            }

            // Handle writes
            let (key_to_convert, mut value, _) = behavior.resource_writes.pop().unwrap();
            let module_id = key_to_mock_module_id(&key_to_convert, universe_len);

            // Serialize a module and store it in bytes so deserialization can succeed.
            let mut serialized_bytes = vec![];
            Module::new_for_test(module_id.clone())
                .serialize(&mut serialized_bytes)
                .expect("Failed to serialize compiled module");
            value.bytes = Some(serialized_bytes.into());

            behavior.module_writes = vec![ModuleWrite::new(module_id, value)];

            // Handle reads.
            let (key_to_convert, _) = behavior.resource_reads.pop().unwrap();
            behavior.module_reads = vec![key_to_mock_module_id(&key_to_convert, universe_len)];
        }
    }

    fn transform_for_groups<E>(
        &self,
        behaviors: &mut [MockIncarnation<KeyType<K>, E>],
        group_config: &[PerGroupConfig<K>],
    ) {
        let num_groups = group_config.len();
        if num_groups == 0 {
            return;
        }

        let group_keys: Vec<_> = group_config.iter().map(|c| &c.key).collect();

        let group_size_query_indicators: Vec<Vec<u8>> = self
            .gen_data
            .group_size_indicators
            .iter()
            .map(|indices| {
                indices
                    .iter()
                    .map(|idx| idx.index(100) as u8)
                    .collect()
            })
            .collect();

        let key_to_group = |key: &KeyType<K>| -> Option<(usize, u32)> {
            let mut hasher = DefaultHasher::new();
            key.hash(&mut hasher);
            let bytes = hasher.finish().to_be_bytes();
            let tag = (bytes[0] % 16) as u32;

            // Map to one of the configured group keys.
            let group_idx = (bytes[1] as usize) % (num_groups + 1); // +1 to allow non-group ops
            (group_idx < num_groups).then_some((group_idx, tag))
        };

        for (behavior_idx, behavior) in behaviors.iter_mut().enumerate() {
            let mut reads = vec![];
            let mut group_reads = vec![];
            for (read_key, _contains_delta) in behavior.resource_reads.clone() {
                if let Some((idx, tag)) = key_to_group(&read_key) {
                    let is_delayed_field_test =
                        matches!(self.delta_test_kind, DeltaTestKind::DelayedFields);
                    let is_delta_tag = group_config[idx]
                        .tags_with_deltas
                        .iter()
                        .any(|t| t.tag == tag);
                    group_reads.push((
                        KeyType(group_keys[idx].clone()),
                        tag,
                        is_delta_tag && is_delayed_field_test,
                    ))
                } else {
                    // contains_delta logic for non-group reads remains the same.
                    let original_read = behavior
                        .resource_reads
                        .iter()
                        .find(|(k, _)| k == &read_key)
                        .unwrap();
                    reads.push(original_read.clone());
                }
            }

            let mut writes = vec![];
            let mut inner_ops: Vec<HashMap<u32, (ValueType, bool)>> =
                vec![HashMap::new(); num_groups];
            for (write_key, value, _has_delayed_field) in behavior.resource_writes.clone() {
                if let Some((key_idx, tag)) = key_to_group(&write_key) {
                    let is_delta_tag = group_config[key_idx]
                        .tags_with_deltas
                        .iter()
                        .any(|t| t.tag == tag);
                    if !value.is_deletion() || !is_delta_tag {
                        inner_ops[key_idx].insert(tag, (value, is_delta_tag));
                    }
                } else {
                    let original_write = behavior
                        .resource_writes
                        .iter()
                        .find(|(k, _, _)| k == &write_key)
                        .unwrap();
                    writes.push(original_write.clone());
                }
            }

            let mut group_writes = vec![];
            for (idx, inner_ops) in inner_ops.into_iter().enumerate() {
                if !inner_ops.is_empty() {
                    group_writes.push((
                        KeyType(group_keys[idx].clone()),
                        raw_metadata(behavior.metadata_seeds[idx % 3]), // Preserved modulo logic
                        inner_ops,
                    ));
                }
            }

            assert!(
                !matches!(self.delta_test_kind, DeltaTestKind::AggregatorV1)
                    || behavior.deltas.is_empty()
            );
            behavior.resource_reads = reads;
            behavior.resource_writes = writes;
            behavior.group_reads = group_reads;
            behavior.group_writes = group_writes;

            if matches!(self.delta_test_kind, DeltaTestKind::DelayedFields) {
                let mut delta_for_keys: HashMap<K, bool> = HashMap::new();
                behavior.deltas = behavior
                    .deltas
                    .iter()
                    .filter_map(|(key, delta, _maybe_tag)| {
                        if let Some((idx, tag)) = key_to_group(key) {
                            let group_key = group_keys[idx];
                            if group_config[idx]
                                .tags_with_deltas
                                .iter()
                                .any(|t| t.tag == tag)
                                && !*delta_for_keys.entry(group_key.clone()).or_insert(false)
                            {
                                *delta_for_keys.get_mut(group_key).unwrap() = true;
                                Some((KeyType(group_key.clone()), *delta, Some(tag)))
                            } else {
                                None
                            }
                        } else {
                            let original_delta =
                                behavior.deltas.iter().find(|(k, _, _)| k == key).unwrap();
                            Some(original_delta.clone())
                        }
                    })
                    .collect();
            }

            behavior.group_queries = group_config
                .iter()
                .enumerate()
                .filter_map(|(idx, config)| {
                    if let Some(query_percentage) = config.query_percentage {
                        assert_le!(query_percentage, 100, "Must be percentage point (0..100]");
                        let indicators = &group_size_query_indicators[behavior_idx];
                        // Use modulo on the number of available indicators.
                        let indicator = indicators[idx % indicators.len()];
                        (indicator < query_percentage).then(|| {
                            (
                                KeyType(config.key.clone()),
                                indicator % 2 == 0, // preserved logic for metadata vs size
                            )
                        })
                    } else {
                        None
                    }
                })
                .collect();
        }
    }
}

/// A simple enum to represent either a write or a delta operation result
enum WriteDeltaVariant<W, D> {
    Write(W),
    Delta(D),
}

fn is_delta_on(index: usize, delta_threshold: Option<usize>) -> bool {
    delta_threshold.is_some_and(|threshold| threshold <= index)
}

pub(crate) fn raw_metadata(v: u64) -> StateValueMetadata {
    StateValueMetadata::legacy(v, &CurrentTimeMicroseconds { microseconds: v })
}

#[derive(Debug)]
pub(crate) enum GroupSizeOrMetadata {
    Size(u64),
    Metadata(Option<StateValueMetadata>),
}

// Utility function to convert a key to a mock module ID. It hashes the key
// to compute a consistent mock account address, with a fixed "test" module name.
pub(crate) fn key_to_mock_module_id<K: Clone + Hash + Debug + Ord>(
    key: &KeyType<K>,
    universe_len: usize,
) -> ModuleId {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let idx = (hasher.finish() % universe_len as u64) as usize;
    let mut addr = [0u8; AccountAddress::LENGTH];
    addr[AccountAddress::LENGTH - 1] = idx as u8;
    addr[AccountAddress::LENGTH - 2] = (idx >> 8) as u8;
    ModuleId::new(AccountAddress::new(addr), Identifier::new("test").unwrap())
}

#[cfg(test)]
mod tests {
    use crate::types::delayed_field_mock_serialization::{
        deserialize_to_delayed_field_u128, serialize_delayed_field_tuple,
    };
    use test_case::test_case;

    #[test_case((0u128, 0u32) ; "zero values")]
    #[test_case((1u128, 42u32) ; "small values")]
    #[test_case((u128::MAX, u32::MAX) ; "maximum values")]
    #[test_case((12345678u128, 87654321u32) ; "large values")]
    fn test_serialize_deserialize_delayed_field_tuple(tuple: (u128, u32)) {
        // Serialize and then deserialize
        let serialized = serialize_delayed_field_tuple(&tuple);
        let deserialized = deserialize_to_delayed_field_u128(&serialized).unwrap();

        assert_eq!(
            tuple, deserialized,
            "Serialization/deserialization failed for tuple ({}, {})",
            tuple.0, tuple.1
        );
    }

    #[test]
    fn test_deserialize_delayed_field_tuple_invalid_data() {
        // Test with invalid data that's too short
        let invalid_data = vec![1, 2, 3];
        let result = deserialize_to_delayed_field_u128(&invalid_data);
        assert!(
            result.is_err(),
            "Expected deserialization to fail with too short data"
        );

        // Test with empty data
        let empty_data: Vec<u8> = vec![];
        let result = deserialize_to_delayed_field_u128(&empty_data);
        assert!(
            result.is_err(),
            "Expected deserialization to fail with empty data"
        );
    }
}
