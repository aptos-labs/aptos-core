// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::delta_change_set::{delta_add, delta_sub, serialize, DeltaOp};
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
use move_vm_types::delayed_values::delayed_field_id::{DelayedFieldID, ExtractUniqueIndex};
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
    pub(crate) phantom: PhantomData<K>,
}

impl<K> TStateView for DeltaDataView<K>
where
    K: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + 'static,
{
    type Key = K;

    // Contains mock storage value with STORAGE_AGGREGATOR_VALUE.
    fn get_state_value(&self, _: &K) -> Result<Option<StateValue>, StateViewError> {
        Ok(Some(StateValue::new_legacy(
            serialize(&STORAGE_AGGREGATOR_VALUE).into(),
        )))
    }

    fn id(&self) -> StateViewId {
        StateViewId::Miscellaneous
    }

    fn get_usage(&self) -> Result<StateStorageUsage, StateViewError> {
        unreachable!("Not used in tests");
    }
}

pub(crate) struct NonEmptyGroupDataView<K> {
    pub(crate) group_keys: HashSet<K>,
    // When we are testing with delayed fields, currently deletion is not supported,
    // so we need to return for each key that can contain a delayed field. for groups,
    // the reserved tag is the only such key, and we simply return a value for all
    // non-group keys to ensure the test runs.
    pub(crate) delayed_field_testing: bool,
}

pub(crate) fn default_group_map() -> BTreeMap<u32, Bytes> {
    let bytes: Bytes = bcs::to_bytes(&(
        STORAGE_AGGREGATOR_VALUE,
        // u32::MAX represents storage version.
        u32::MAX,
    ))
    .unwrap()
    .into();

    BTreeMap::from([(RESERVED_TAG, bytes)])
}

impl<K> TStateView for NonEmptyGroupDataView<K>
where
    K: Debug + PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + 'static,
{
    type Key = K;

    // Contains mock storage value with a non-empty group (w. value at RESERVED_TAG).
    fn get_state_value(&self, key: &K) -> Result<Option<StateValue>, StateViewError> {
        Ok(self
            .group_keys
            .contains(key)
            .then(|| {
                let bytes = bcs::to_bytes(&default_group_map()).unwrap();
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

impl Arbitrary for ValueType {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        vec(any::<u8>(), 17)
            .prop_map(|mut v| {
                let use_value = v[0] < 128;
                v.resize(16, 0);
                ValueType::from_value(v, use_value)
            })
            .boxed()
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

#[derive(Clone, Copy)]
pub(crate) struct TransactionGenParams {
    /// Each transaction's read-set consists of between 1 and read_size many reads.
    read_size: usize,
    /// Each mock execution will produce between 1 and output_size many writes and deltas.
    output_size: usize,
    /// The number of different incarnation behaviors that a mock execution of the transaction
    /// may exhibit. For instance, incarnation_alternatives = 1 corresponds to a "static"
    /// mock execution behavior regardless of the incarnation, while value > 1 may lead to "dynamic",
    /// i.e. different behavior when executing different incarnations of the transaction.
    incarnation_alternatives: usize,
}

#[derive(Arbitrary, Debug, Clone)]
#[proptest(params = "TransactionGenParams")]
pub(crate) struct TransactionGen<V: Into<Vec<u8>> + Arbitrary + Clone + Debug + Eq + 'static> {
    /// Generate keys for possible read-sets of the transaction based on the above parameters.
    #[proptest(
        strategy = "vec(vec(any::<Index>(), 1..=params.read_size), params.incarnation_alternatives)"
    )]
    reads: Vec<Vec<Index>>,
    /// Generate keys and values for possible write-sets based on above transaction gen parameters.
    /// Based on how the test is configured, some of these "writes" will convert to deltas.
    #[proptest(
        strategy = "vec(vec((any::<Index>(), any::<V>()), 1..=params.output_size), \
		    params.incarnation_alternatives)"
    )]
    modifications: Vec<Vec<(Index, V)>>,
    /// Generate gas for different incarnations of the transactions.
    #[proptest(strategy = "vec(any::<Index>(), params.incarnation_alternatives)")]
    gas: Vec<Index>,
    /// Generate seeds for group metadata.
    #[proptest(strategy = "vec(vec(any::<Index>(), 3), params.incarnation_alternatives)")]
    metadata_seeds: Vec<Vec<Index>>,
    /// Generate indices to derive random behavior for querying resource group sizes.
    /// For now hardcoding 3 resource groups.
    #[proptest(
        strategy = "vec((any::<Index>(), any::<Index>(), any::<Index>()), params.incarnation_alternatives)"
    )]
    group_size_indicators: Vec<(Index, Index, Index)>,
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
}

// TODO: try and test different strategies.
impl TransactionGenParams {
    pub fn new_dynamic() -> Self {
        TransactionGenParams {
            read_size: 10,
            output_size: 5,
            incarnation_alternatives: 5,
        }
    }

    // The read and write will be converted to a module read and write.
    pub fn new_dynamic_modules_only() -> Self {
        TransactionGenParams {
            read_size: 1,
            output_size: 1,
            incarnation_alternatives: 5,
        }
    }

    // Last read and write will be converted to module reads and writes.
    pub fn new_dynamic_with_modules() -> Self {
        TransactionGenParams {
            read_size: 3,
            output_size: 3,
            incarnation_alternatives: 5,
        }
    }
}

impl Default for TransactionGenParams {
    fn default() -> Self {
        TransactionGenParams {
            read_size: 10,
            output_size: 1,
            incarnation_alternatives: 1,
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

impl<V: Into<Vec<u8>> + Arbitrary + Clone + Debug + Eq + Sync + Send> TransactionGen<V> {
    /// Determines whether to generate a delta operation or a write operation based on parameters
    ///
    /// # Arguments
    /// * `is_delta_path` - attempt to generate a delta value first
    /// * `value` - The value to process
    /// * `delta_threshold` - All indices below this threshold will be writes (not deltas)
    /// * `allow_deletes` - Whether deletion operations are allowed
    ///
    /// # Returns
    /// Either a delta operation or a write operation with its is_aggregator_v1 flag
    fn generate_write_or_delta<KeyType>(
        is_delta_path: bool,
        value: &V,
        key: KeyType,
        allow_deletes: bool,
    ) -> WriteDeltaVariant<(KeyType, ValueType), (KeyType, DeltaOp)> {
        // First check if this should be a delta
        if is_delta_path {
            let val_u128 = ValueType::from_value(value.clone(), true)
                .as_u128()
                .unwrap()
                .unwrap();

            // Not all values become deltas - some remain as normal writes
            if val_u128 % 10 != 0 {
                let delta = if val_u128 % 10 < 5 {
                    delta_sub(val_u128 % 100, u128::MAX)
                } else {
                    delta_add(val_u128 % 100, u128::MAX)
                };
                return WriteDeltaVariant::Delta((key, delta));
            }
        }

        // Otherwise create a normal write
        let val_u128 = ValueType::from_value(value.clone(), true)
            .as_u128()
            .unwrap()
            .unwrap();
        let is_deletion = allow_deletes && val_u128 % 23 == 0;
        let mut write_value = ValueType::from_value(value.clone(), !is_deletion);
        write_value.metadata = raw_metadata((val_u128 >> 64) as u64);

        WriteDeltaVariant::Write((key, write_value))
    }

    fn writes_and_deltas_from_gen<K: Clone + Hash + Debug + Eq + Ord>(
        // TODO: disentangle writes and deltas.
        universe: &[K],
        gen: Vec<Vec<(Index, V)>>,
        allow_deletes: bool,
        delta_threshold: Option<usize>,
    ) -> Vec<(
        /* writes = */ Vec<(KeyType<K>, ValueType, bool)>,
        /* deltas = */ Vec<(KeyType<K>, DeltaOp)>,
    )> {
        let mut ret = Vec::with_capacity(gen.len());
        for write_gen in gen.into_iter() {
            let mut keys_modified = BTreeSet::new();
            let mut incarnation_writes = vec![];
            let mut incarnation_deltas = vec![];
            for (idx, value) in write_gen.into_iter() {
                let i = idx.index(universe.len());
                let key = universe[i].clone();
                if !keys_modified.contains(&key) {
                    keys_modified.insert(key.clone());

                    let is_delta_path = is_delta_on(i, delta_threshold);
                    match Self::generate_write_or_delta(
                        is_delta_path,
                        &value,
                        KeyType(key),
                        allow_deletes,
                    ) {
                        WriteDeltaVariant::Write((key, value)) => {
                            incarnation_writes.push((key, value, is_delta_path));
                        },
                        WriteDeltaVariant::Delta((key, delta)) => {
                            incarnation_deltas.push((key, delta));
                        },
                    }
                }
            }
            ret.push((incarnation_writes, incarnation_deltas));
        }
        ret
    }

    fn reads_from_gen<K: Clone + Hash + Debug + Eq + Ord>(
        universe: &[K],
        gen: Vec<Vec<Index>>,
        delta_threshold: Option<usize>,
    ) -> Vec<Vec<(KeyType<K>, bool)>> {
        let mut ret = vec![];
        for read_gen in gen.into_iter() {
            let mut incarnation_reads: Vec<(KeyType<K>, bool)> = vec![];
            for idx in read_gen.into_iter() {
                let i = idx.index(universe.len());
                let key = universe[i].clone();
                incarnation_reads.push((KeyType(key), is_delta_on(i, delta_threshold)));
            }
            ret.push(incarnation_reads);
        }
        ret
    }

    fn gas_from_gen(gas_gen: Vec<Index>) -> Vec<u64> {
        // TODO: generalize gas charging.
        gas_gen
            .into_iter()
            .map(|idx| idx.index(MAX_GAS_PER_TXN as usize + 1) as u64)
            .collect()
    }

    fn group_size_indicator_from_gen(
        group_size_query_gen: Vec<(Index, Index, Index)>,
    ) -> Vec<(u8, u8, u8)> {
        group_size_query_gen
            .into_iter()
            .map(|(idx1, idx2, idx3)| {
                (
                    idx1.index(100) as u8,
                    idx2.index(100) as u8,
                    idx3.index(100) as u8,
                )
            })
            .collect()
    }

    fn new_mock_write_txn<
        K: Clone + Hash + Debug + Eq + Ord,
        E: Debug + Clone + TransactionEvent,
    >(
        self,
        universe: &[K],
        allow_deletes: bool,
        delta_threshold: Option<usize>,
    ) -> MockTransaction<KeyType<K>, E> {
        let reads = Self::reads_from_gen(universe, self.reads, delta_threshold);
        let gas = Self::gas_from_gen(self.gas);

        let behaviors = Self::writes_and_deltas_from_gen(
            universe,
            self.modifications,
            allow_deletes,
            delta_threshold,
        )
        .into_iter()
        .zip(reads)
        .zip(gas)
        .zip(
            self.metadata_seeds
                .into_iter()
                .map(|vec| {
                    [
                        vec[0].index(100000) as u64,
                        vec[1].index(100000) as u64,
                        vec[2].index(100000) as u64,
                    ]
                })
                .collect::<Vec<_>>(),
        )
        .map(|((((writes, deltas), reads), gas), metadata_seeds)| {
            MockIncarnation::new_with_metadata_seeds(
                reads,
                writes,
                // materialize_groups sets the Option<u32> to a tag as needed.
                deltas
                    .into_iter()
                    .map(|(k, delta)| (k, delta, None))
                    .collect(),
                vec![], // events
                metadata_seeds,
                gas,
            )
        })
        .collect();

        MockTransaction::from_behaviors(behaviors)
    }

    pub(crate) fn materialize<
        K: Clone + Hash + Debug + Eq + Ord,
        E: Send + Sync + Debug + Clone + TransactionEvent,
    >(
        self,
        universe: &[K],
    ) -> MockTransaction<KeyType<K>, E> {
        self.new_mock_write_txn(universe, true, None)
    }

    pub(crate) fn materialize_modules<
        K: Clone + Hash + Debug + Eq + Ord,
        E: Send + Sync + Debug + Clone + TransactionEvent,
    >(
        self,
        universe: &[K],
    ) -> MockTransaction<KeyType<K>, E> {
        let universe_len = universe.len();

        let mut behaviors = self
            .new_mock_write_txn(universe, false, None)
            .into_behaviors();

        behaviors.iter_mut().for_each(|behavior| {
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
        });

        MockTransaction::from_behaviors(behaviors)
    }

    // Generates a mock txn without group reads/writes and converts it to have group
    // operations. Last 3 keys of the universe are used as group keys.
    pub(crate) fn materialize_groups<
        K: Clone + Hash + Debug + Eq + Ord,
        E: Send + Sync + Debug + Clone + TransactionEvent,
    >(
        self,
        universe: &[K],
        group_size_query_pcts: [Option<u8>; 3],
        delta_threshold: Option<usize>,
    ) -> MockTransaction<KeyType<K>, E> {
        let universe_len = universe.len();
        assert_ge!(universe_len, 3, "Universe must have size >= 3");

        let group_size_query_indicators =
            Self::group_size_indicator_from_gen(self.group_size_indicators.clone());
        let mut behaviors = self
            .new_mock_write_txn(&universe[0..universe.len() - 3], false, delta_threshold)
            .into_behaviors();

        let key_to_group = |key: &KeyType<K>| -> Option<(usize, u32, bool)> {
            let mut hasher = DefaultHasher::new();
            key.hash(&mut hasher);
            let bytes = hasher.finish().to_be_bytes();
            // Choose from a smaller universe so different ops have intersection on a key.
            let tag = (bytes[0] % 16) as u32;

            let group_key_idx = bytes[1] % 4;

            // 3/4 of the time key will map to group - rest are normal resource accesses.
            (group_key_idx < 3).then_some((group_key_idx as usize, tag, group_key_idx > 0))
        };

        for (behavior_idx, behavior) in behaviors.iter_mut().enumerate() {
            let mut reads = vec![];
            let mut group_reads = vec![];
            for (read_key, contains_delta) in behavior.resource_reads.clone() {
                assert!(read_key != KeyType(universe[universe_len - 1].clone()));
                assert!(read_key != KeyType(universe[universe_len - 2].clone()));
                assert!(read_key != KeyType(universe[universe_len - 3].clone()));
                match key_to_group(&read_key) {
                    Some((idx, tag, has_delayed_field)) => {
                        // Custom logic for has_delayed_fields for groups: shadowing
                        // the flag of the original read.
                        group_reads.push((
                            KeyType(universe[universe_len - 1 - idx].clone()),
                            tag,
                            // Reserved tag is configured to have delayed fields.
                            has_delayed_field && tag == RESERVED_TAG && delta_threshold.is_some(),
                        ))
                    },
                    None => reads.push((read_key, contains_delta)),
                }
            }

            let mut writes = vec![];
            let mut group_writes = vec![];
            let mut inner_ops = vec![HashMap::new(); 3];
            for (write_key, value, has_delayed_field) in behavior.resource_writes.clone() {
                match key_to_group(&write_key) {
                    Some((key_idx, tag, has_delayed_field)) => {
                        // Same shadowing of has_delayed_field variable and logic as above.
                        if tag != RESERVED_TAG || !value.is_deletion() {
                            inner_ops[key_idx]
                                .insert(tag, (value, has_delayed_field && tag == RESERVED_TAG));
                        }
                    },
                    None => {
                        writes.push((write_key, value, has_delayed_field));
                    },
                }
            }
            for (idx, inner_ops) in inner_ops.into_iter().enumerate() {
                if !inner_ops.is_empty() {
                    group_writes.push((
                        KeyType(universe[universe_len - 1 - idx].clone()),
                        raw_metadata(behavior.metadata_seeds[idx]),
                        inner_ops,
                    ));
                }
            }

            // Group test does not handle deltas for aggregator v1(different view, no default
            // storage value). However, it does handle deltas (added below) for delayed fields.
            assert!(delta_threshold.is_some() || behavior.deltas.is_empty());
            behavior.resource_reads = reads;
            behavior.resource_writes = writes;
            behavior.group_reads = group_reads;
            behavior.group_writes = group_writes;

            if delta_threshold.is_some() {
                // TODO: We can have a threshold over which we create a delta for RESERVED_TAG,
                // because currently only RESERVED_TAG in a group contains a delayed field.
                let mut delta_for_keys = [false; 3];
                behavior.deltas = behavior
                    .deltas
                    .iter()
                    .filter_map(|(key, delta, maybe_tag)| {
                        if let Some((idx, _, has_delayed_field)) = key_to_group(key) {
                            if has_delayed_field && !delta_for_keys[idx] {
                                delta_for_keys[idx] = true;
                                Some((
                                    KeyType(universe[universe_len - 1 - idx].clone()),
                                    *delta,
                                    Some(RESERVED_TAG),
                                ))
                            } else {
                                None
                            }
                        } else {
                            Some((key.clone(), *delta, *maybe_tag))
                        }
                    })
                    .collect();
            }

            behavior.group_queries = group_size_query_pcts
                .iter()
                .enumerate()
                .filter_map(|(idx, size_query_pct)| match size_query_pct {
                    Some(size_query_pct) => {
                        assert_le!(*size_query_pct, 100, "Must be percentage point (0..100]");
                        let indicator = match idx {
                            0 => group_size_query_indicators[behavior_idx].0,
                            1 => group_size_query_indicators[behavior_idx].1,
                            2 => group_size_query_indicators[behavior_idx].2,
                            _ => unreachable!("Test uses 3 groups"),
                        };
                        (indicator < *size_query_pct).then(|| {
                            (
                                KeyType(universe[universe_len - 1 - idx].clone()),
                                // TODO: handle metadata queries more uniformly w. size.
                                indicator % 2 == 0,
                            )
                        })
                    },
                    None => None,
                })
                .collect();
        }

        // When delayed fields are not enabled, the flag is ignored, so we can always
        // set with_delayed_fields here.
        if delta_threshold.is_some() {
            MockTransaction::from_behaviors(behaviors).with_delayed_fields_testing()
        } else {
            MockTransaction::from_behaviors(behaviors)
        }
    }

    pub(crate) fn materialize_with_deltas<
        K: Clone + Hash + Debug + Eq + Ord,
        E: Send + Sync + Debug + Clone + TransactionEvent,
    >(
        self,
        universe: &[K],
        delta_threshold: usize,
        allow_deletes: bool,
    ) -> MockTransaction<KeyType<K>, E> {
        // Enable delta generation for this specific method
        self.new_mock_write_txn(universe, allow_deletes, Some(delta_threshold))
            .with_aggregator_v1_testing()
    }
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

// ID is just the unique index as u128.
pub(crate) fn serialize_from_delayed_field_u128(value_or_id: u128, version: u32) -> Bytes {
    let tuple = (value_or_id, version);
    serialize_delayed_field_tuple(&tuple)
}

pub(crate) fn serialize_from_delayed_field_id(
    delayed_field_id: DelayedFieldID,
    version: u32,
) -> Bytes {
    let tuple = (delayed_field_id.extract_unique_index() as u128, version);
    serialize_delayed_field_tuple(&tuple)
}

fn serialize_delayed_field_tuple(value: &(u128, u32)) -> Bytes {
    bcs::to_bytes(value)
        .expect("Failed to serialize (u128, u32) tuple")
        .into()
}

/// The width of the delayed field is not used in the tests, and fixed as 8 for
/// all delayed field constructions. However, only the real ID is actually
/// serialized and deserialized (together with the version).
pub(crate) fn deserialize_to_delayed_field_u128(bytes: &[u8]) -> Result<(u128, u32), bcs::Error> {
    bcs::from_bytes::<(u128, u32)>(bytes)
}

pub(crate) fn deserialize_to_delayed_field_id(
    bytes: &[u8],
) -> Result<(DelayedFieldID, u32), bcs::Error> {
    let (id, version) = bcs::from_bytes::<(u128, u32)>(bytes)?;
    Ok((DelayedFieldID::from((id as u32, 8)), version))
}

#[cfg(test)]
mod tests {
    use super::*;
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
