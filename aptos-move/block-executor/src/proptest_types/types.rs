// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::task::{ExecutionStatus, ExecutorTask, TransactionOutput};
use aptos_aggregator::{
    delayed_change::DelayedChange,
    delta_change_set::{delta_add, delta_sub, serialize, DeltaOp},
    resolver::TAggregatorV1View,
};
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    account_address::AccountAddress,
    contract_event::TransactionEvent,
    error::PanicError,
    executable::ModulePath,
    fee_statement::FeeStatement,
    on_chain_config::CurrentTimeMicroseconds,
    state_store::{
        errors::StateViewError,
        state_storage_usage::StateStorageUsage,
        state_value::{StateValue, StateValueMetadata},
        StateViewId, TStateView,
    },
    transaction::BlockExecutableTransaction as Transaction,
    write_set::{TransactionWrite, WriteOp, WriteOpKind},
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_types::{
    module_and_script_storage::code_storage::AptosCodeStorage,
    module_write_set::ModuleWrite,
    resolver::{
        BlockSynchronizationKillSwitch, ResourceGroupSize, TExecutorView, TResourceGroupView,
    },
    resource_group_adapter::{
        decrement_size_for_remove_tag, group_tagged_resource_size, increment_size_for_add_tag,
    },
};
use bytes::Bytes;
use claims::{assert_ge, assert_le, assert_ok};
use move_core_types::{identifier::IdentStr, language_storage::ModuleId, value::MoveTypeLayout};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use move_vm_runtime::Module;
use once_cell::sync::OnceCell;
use proptest::{arbitrary::Arbitrary, collection::vec, prelude::*, proptest, sample::Index};
use proptest_derive::Arbitrary;
use std::{
    collections::{hash_map::DefaultHasher, BTreeMap, BTreeSet, HashMap, HashSet},
    fmt::Debug,
    hash::{Hash, Hasher},
    marker::PhantomData,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
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
}

impl<K> TStateView for NonEmptyGroupDataView<K>
where
    K: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + 'static,
{
    type Key = K;

    // Contains mock storage value with a non-empty group (w. value at RESERVED_TAG).
    fn get_state_value(&self, key: &K) -> Result<Option<StateValue>, StateViewError> {
        if self.group_keys.contains(key) {
            let group: BTreeMap<u32, Bytes> = BTreeMap::from([(RESERVED_TAG, vec![0].into())]);

            let bytes = bcs::to_bytes(&group).unwrap();
            Ok(Some(StateValue::new_with_metadata(
                bytes.into(),
                raw_metadata(5),
            )))
        } else {
            Ok(None)
        }
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
        self.bytes = bytes.into();
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
#[derive(Clone, Debug)]
pub(crate) struct MockIncarnation<K, E> {
    /// A vector of keys to be read during mock incarnation execution.
    /// bool being false means that the read should be considered as an AggregatorV1 read.
    pub(crate) reads: Vec<(K, bool)>,
    /// A vector of keys and corresponding values to be written during mock incarnation execution.
    /// bool being false means that the write should be considered as an AggregatorV1 write.
    pub(crate) writes: Vec<(K, ValueType, bool)>,
    pub(crate) group_reads: Vec<(K, u32)>,
    pub(crate) group_writes: Vec<(K, StateValueMetadata, HashMap<u32, ValueType>)>,
    // For testing get_module_or_build_with and insert_verified_module interfaces.
    pub(crate) module_reads: Vec<ModuleId>,
    pub(crate) module_writes: Vec<ModuleWrite<ValueType>>,
    /// Keys to query group size for - false is querying size, true is querying metadata.
    pub(crate) group_queries: Vec<(K, bool)>,
    /// A vector of keys and corresponding deltas to be produced during mock incarnation execution.
    pub(crate) deltas: Vec<(K, DeltaOp)>,
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
        reads: Vec<(K, bool)>,
        writes: Vec<(K, ValueType, bool)>,
        deltas: Vec<(K, DeltaOp)>,
        events: Vec<E>,
        metadata_seeds: [u64; 3],
        gas: u64,
    ) -> Self {
        Self {
            reads,
            writes,
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
        reads: Vec<(K, bool)>,
        writes: Vec<(K, ValueType, bool)>,
        deltas: Vec<(K, DeltaOp)>,
        events: Vec<E>,
        gas: u64,
    ) -> Self {
        Self {
            reads,
            writes,
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
        }
    }

    pub(crate) fn from_behaviors(behaviors: Vec<MockIncarnation<K, E>>) -> Self {
        Self::Write {
            incarnation_counter: Arc::new(AtomicUsize::new(0)),
            incarnation_behaviors: behaviors,
        }
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

// TODO: move generation to separate file.
// TODO: consider adding writes to reads (read-before-write). Similar behavior to the Move-VM
// and may force more testing (since we check read results).
impl<V: Into<Vec<u8>> + Arbitrary + Clone + Debug + Eq + Sync + Send> TransactionGen<V> {
    fn writes_and_deltas_from_gen<K: Clone + Hash + Debug + Eq + Ord>(
        // TODO: disentangle writes and deltas.
        universe: &[K],
        gen: Vec<Vec<(Index, V)>>,
        delta_fn: &dyn Fn(usize, &V) -> Option<DeltaOp>,
        allow_deletes: bool,
        delta_threshold: Option<usize>,
    ) -> Vec<(
        /* writes = */ Vec<(KeyType<K>, ValueType, bool)>,
        /* deltas = */ Vec<(KeyType<K>, DeltaOp)>,
    )> {
        let delta_threshold = delta_threshold.unwrap_or(universe.len() + 1);
        let mut ret = vec![];
        for write_gen in gen.into_iter() {
            let mut keys_modified = BTreeSet::new();
            let mut incarnation_writes = vec![];
            let mut incarnation_deltas = vec![];
            for (idx, value) in write_gen.into_iter() {
                let i = idx.index(universe.len());
                let key = universe[i].clone();
                if !keys_modified.contains(&key) {
                    keys_modified.insert(key.clone());
                    match delta_fn(i, &value) {
                        Some(delta) => incarnation_deltas.push((KeyType(key), delta)),
                        None => {
                            // One out of 23 writes will be a deletion
                            let val_u128 = ValueType::from_value(value.clone(), true)
                                .as_u128()
                                .unwrap()
                                .unwrap();
                            let is_deletion = allow_deletes && val_u128 % 23 == 0;
                            let mut value = ValueType::from_value(value.clone(), !is_deletion);
                            value.metadata = raw_metadata((val_u128 >> 64) as u64);
                            incarnation_writes.push((KeyType(key), value, i < delta_threshold));
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
        let delta_threshold = delta_threshold.unwrap_or(universe.len() + 1);
        let mut ret = vec![];
        for read_gen in gen.into_iter() {
            let mut incarnation_reads: Vec<(KeyType<K>, bool)> = vec![];
            for idx in read_gen.into_iter() {
                let i = idx.index(universe.len());
                let key = universe[i].clone();
                incarnation_reads.push((KeyType(key), i < delta_threshold));
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
        delta_fn: &dyn Fn(usize, &V) -> Option<DeltaOp>,
        allow_deletes: bool,
        delta_threshold: Option<usize>,
    ) -> MockTransaction<KeyType<K>, E> {
        let reads = Self::reads_from_gen(universe, self.reads, delta_threshold);
        let gas = Self::gas_from_gen(self.gas);

        let behaviors = Self::writes_and_deltas_from_gen(
            universe,
            self.modifications,
            &delta_fn,
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
                deltas,
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
        // TODO(BlockSTMv2): different testing for modules.
        let is_delta = |_, _: &V| -> Option<DeltaOp> { None };

        self.new_mock_write_txn(universe, &is_delta, true, None)
    }

    pub(crate) fn materialize_modules<
        K: Clone + Hash + Debug + Eq + Ord,
        E: Send + Sync + Debug + Clone + TransactionEvent,
    >(
        self,
        universe: &[K],
    ) -> MockTransaction<KeyType<K>, E> {
        let universe_len = universe.len();
        assert_ge!(universe_len, 3, "Universe must have size >= 3");

        let is_delta = |_, _: &V| -> Option<DeltaOp> { None };
        let mut behaviors = self.new_mock_write_txn(universe, &is_delta, false, None).into_behaviors();

        behaviors.iter_mut().for_each(|behavior| {
            // Handle writes
            let (key_to_convert, mut value, _) = behavior.writes.pop().unwrap();
            let module_id = key_to_module_id(&key_to_convert, universe_len);
            
            // Serialize a module and store it in bytes so deserialization can succeed.
            let mut serialized_bytes = Vec::new();
            Module::new_for_test(module_id.clone()).serialize(&mut serialized_bytes).expect("Failed to serialize compiled module");
            value.bytes = Some(serialized_bytes.into());
            
            behavior.module_writes = vec![ModuleWrite::new(module_id, value)];

            // Handle reads
            let (key_to_convert, _) = behavior.reads.pop().unwrap();
            behavior.module_reads = vec![key_to_module_id(&key_to_convert, universe_len)];
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
    ) -> MockTransaction<KeyType<K>, E> {
        let universe_len = universe.len();
        assert_ge!(universe_len, 3, "Universe must have size >= 3");

        let is_delta = |_, _: &V| -> Option<DeltaOp> { None };

        let group_size_query_indicators =
            Self::group_size_indicator_from_gen(self.group_size_indicators.clone());
        let mut behaviors = self
            .new_mock_write_txn(&universe[0..universe.len() - 3], &is_delta, false, None)
            .into_behaviors();

        let key_to_group = |key: &KeyType<K>| -> Option<(usize, u32)> {
            let mut hasher = DefaultHasher::new();
            key.hash(&mut hasher);
            let bytes = hasher.finish().to_be_bytes();
            // Choose from a smaller universe so different ops have intersection on a key.
            let tag = (bytes[0] % 16) as u32;

            let group_key_idx = bytes[1] % 4;

            (group_key_idx < 3).then_some((group_key_idx as usize, tag))
        };

        for (behavior_idx, behavior) in behaviors.iter_mut().enumerate() {
            let mut reads = vec![];
            let mut group_reads = vec![];
            for read_key in behavior.reads.clone() {
                assert!(read_key.0 != KeyType(universe[universe_len - 1].clone()));
                assert!(read_key.0 != KeyType(universe[universe_len - 2].clone()));
                assert!(read_key.0 != KeyType(universe[universe_len - 3].clone()));
                match key_to_group(&read_key.0) {
                    Some((idx, tag)) => {
                        group_reads.push((KeyType(universe[universe_len - 1 - idx].clone()), tag))
                    },
                    None => reads.push(read_key),
                }
            }

            let mut writes = vec![];
            let mut group_writes = vec![];
            let mut inner_ops = vec![HashMap::new(); 3];
            for (write_key, value, _) in behavior.writes.clone() {
                match key_to_group(&write_key) {
                    Some((key_idx, tag)) => {
                        if tag != RESERVED_TAG || !value.is_deletion() {
                            inner_ops[key_idx].insert(tag, value);
                        }
                    },
                    None => writes.push((write_key, value, true)),
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

            // Group test does not handle deltas (different view, no default storage value).
            assert!(behavior.deltas.is_empty());
            behavior.reads = reads;
            behavior.writes = writes;
            behavior.group_reads = group_reads;
            behavior.group_writes = group_writes;

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

        MockTransaction::from_behaviors(behaviors)
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
        let is_delta = |i, v: &V| -> Option<DeltaOp> {
            if i >= delta_threshold {
                let val = ValueType::from_value(v.clone(), true)
                    .as_u128()
                    .unwrap()
                    .unwrap();
                if val % 10 == 0 {
                    None
                } else if val % 10 < 5 {
                    Some(delta_sub(val % 100, u128::MAX))
                } else {
                    Some(delta_add(val % 100, u128::MAX))
                }
            } else {
                None
            }
        };

        self.new_mock_write_txn(universe, &is_delta, allow_deletes, Some(delta_threshold))
    }
}

///////////////////////////////////////////////////////////////////////////
// Mock transaction executor implementation.
///////////////////////////////////////////////////////////////////////////

pub(crate) struct MockTask<K, E> {
    phantom_data: PhantomData<(K, E)>,
}

impl<K, E> MockTask<K, E> {
    pub fn new() -> Self {
        Self {
            phantom_data: PhantomData,
        }
    }
}

impl<K, E> ExecutorTask for MockTask<K, E>
where
    K: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + Debug + 'static,
    E: Send + Sync + Debug + Clone + TransactionEvent + 'static,
{
    type Error = usize;
    type Output = MockOutput<K, E>;
    type Txn = MockTransaction<K, E>;

    fn init(_environment: &AptosEnvironment, _state_view: &impl TStateView<Key = K>) -> Self {
        Self::new()
    }

    fn execute_transaction(
        &self,
        view: &(impl TExecutorView<K, u32, MoveTypeLayout, ValueType>
              + TResourceGroupView<GroupKey = K, ResourceTag = u32, Layout = MoveTypeLayout>
              + AptosCodeStorage
              + BlockSynchronizationKillSwitch),
        txn: &Self::Txn,
        txn_idx: TxnIndex,
    ) -> ExecutionStatus<Self::Output, Self::Error> {
        match txn {
            MockTransaction::Write {
                incarnation_counter,
                incarnation_behaviors,
            } => {
                // Use incarnation counter value as an index to determine the read-
                // and write-sets of the execution. Increment incarnation counter to
                // simulate dynamic behavior when there are multiple possible read-
                // and write-sets (i.e. each are selected round-robin).
                let idx = incarnation_counter.fetch_add(1, Ordering::SeqCst);

                let behavior = &incarnation_behaviors[idx % incarnation_behaviors.len()];

                // Reads
                let mut module_read_results = vec![];
                for module_id in behavior.module_reads.iter() {
                    module_read_results.push(view.fetch_state_value_metadata(&module_id.address(), &module_id.name()).unwrap());
                }

                let mut read_results = vec![];
                for (key, not_aggregagor_v1) in behavior.reads.iter() {
                    // TODO: later test errors as well? (by fixing state_view behavior).
                    // TODO: also prop test modules

                    let maybe_bytes = if *not_aggregagor_v1 {
                        view.get_resource_bytes(key, None)
                    } else {
                        view.get_aggregator_v1_state_value(key)
                            .map(|maybe_state_value| {
                                maybe_state_value.map(|state_value| state_value.bytes().clone())
                            })
                    };

                    match maybe_bytes {
                        Ok(v) => read_results.push(v.map(Into::into)),
                        Err(_) => read_results.push(None),
                    }
                }
                // Read from groups.
                // TODO: also read group sizes (if there are any group reads).
                for (group_key, resource_tag) in behavior.group_reads.iter() {
                    match view.get_resource_from_group(group_key, resource_tag, None) {
                        Ok(v) => read_results.push(v.map(Into::into)),
                        Err(_) => read_results.push(None),
                    }
                }

                let read_group_size_or_metadata = behavior
                    .group_queries
                    .iter()
                    .flat_map(|(group_key, query_metadata)| {
                        let res = if *query_metadata {
                            GroupSizeOrMetadata::Metadata(
                                match view.get_resource_state_value_metadata(group_key) {
                                    Ok(v) => v,
                                    Err(_) => return None,
                                }
                            )
                        } else {
                            GroupSizeOrMetadata::Size(
                                match view.resource_group_size(group_key) {
                                    Ok(v) => v.get(),
                                    Err(_) => return None,
                                }
                            )
                        };

                        Some((group_key.clone(), res))
                    })
                    .collect();

                let mut group_writes = vec![];
                for (key, metadata, inner_ops) in behavior.group_writes.iter() {
                    let mut new_inner_ops = HashMap::new();

                    let mut new_group_size = match view.resource_group_size(key) {
                        Ok(size) => size,
                        Err(_) => break,
                    };
                    let group_size = new_group_size.clone();

                    let mut speculative_abort = false;
                    for (tag, inner_op) in inner_ops.iter() {
                        let exists = match view
                            .get_resource_from_group(key, tag, None) {
                                Ok(v) => v.is_some(),
                                Err(_) => {
                                    speculative_abort = true;
                                    break;
                                }
                            };
                        assert!(
                            *tag != RESERVED_TAG || exists,
                            "RESERVED_TAG must always be present in groups in tests"
                        );

                        // inner op is either deletion or creation.
                        assert!(!inner_op.is_modification());

                        let maybe_op = if exists {
                            Some(
                                if inner_op.is_creation()
                                    && (inner_op.bytes().unwrap()[0] % 4 < 3
                                        || *tag == RESERVED_TAG)
                                {
                                    ValueType::new(
                                        inner_op.bytes.clone(),
                                        StateValueMetadata::none(),
                                        WriteOpKind::Modification,
                                    )
                                } else {
                                    ValueType::new(
                                        None,
                                        StateValueMetadata::none(),
                                        WriteOpKind::Deletion,
                                    )
                                },
                            )
                        } else {
                            inner_op.is_creation().then(|| inner_op.clone())
                        };

                        if let Some(new_inner_op) = maybe_op {
                            if exists {
                                let old_tagged_value_size =
                                    view.resource_size_in_group(key, tag).unwrap();
                                let old_size =
                                    group_tagged_resource_size(tag, old_tagged_value_size).unwrap();
                                // let _ =
                                // decrement_size_for_remove_tag(&mut new_group_size, old_size);
                                if decrement_size_for_remove_tag(&mut new_group_size, old_size)
                                    .is_err()
                                {
                                    // Check it only happens for speculative executions that may not
                                    // commit by returning incorrect (empty) output.
                                    return ExecutionStatus::Success(MockOutput::skip_output());
                                }
                            }
                            if !new_inner_op.is_deletion() {
                                let new_size = group_tagged_resource_size(
                                    tag,
                                    inner_op.bytes.as_ref().unwrap().len(),
                                )
                                .unwrap();
                                if increment_size_for_add_tag(&mut new_group_size, new_size)
                                    .is_err()
                                {
                                    // Check it only happens for speculative executions that may not
                                    // commit by returning incorrect (empty) output.
                                    return ExecutionStatus::Success(MockOutput::skip_output());
                                }
                            }

                            new_inner_ops.insert(*tag, new_inner_op);
                        }
                    }

                    if speculative_abort {
                        break;
                    }

                    if !new_inner_ops.is_empty() {
                        if group_size.get() > 0
                            && new_group_size == ResourceGroupSize::zero_combined()
                        {
                            // TODO: reserved tag currently prevents this code from being run.
                            // Group got deleted.
                            group_writes.push((
                                key.clone(),
                                ValueType::new(None, metadata.clone(), WriteOpKind::Deletion),
                                new_group_size,
                                new_inner_ops,
                            ));
                        } else {
                            let op_kind = if group_size.get() == 0 {
                                WriteOpKind::Creation
                            } else {
                                WriteOpKind::Modification
                            };

                            // Not testing metadata_op here, always modification.
                            group_writes.push((
                                key.clone(),
                                ValueType::new(Some(Bytes::new()), metadata.clone(), op_kind),
                                new_group_size,
                                new_inner_ops,
                            ));
                        }
                    }
                }

                // generate group_writes.
                ExecutionStatus::Success(MockOutput {
                    writes: behavior
                        .writes
                        .clone()
                        .into_iter()
                        .filter_map(|(k, v, w)| w.then_some((k, v)))
                        .collect(),
                    aggregator_v1_writes: behavior
                        .writes
                        .clone()
                        .into_iter()
                        .filter_map(|(k, v, w)| (!w).then_some((k, v)))
                        .collect(),
                    group_writes,
                    module_writes: behavior.module_writes.clone(),
                    deltas: behavior.deltas.clone(),
                    events: behavior.events.to_vec(),
                    read_results,
                    module_read_results,
                    read_group_size_or_metadata,
                    materialized_delta_writes: OnceCell::new(),
                    total_gas: behavior.gas,
                    skipped: false,
                })
            },
            MockTransaction::SkipRest(gas) => {
                let mut mock_output = MockOutput::skip_output();
                mock_output.total_gas = *gas;
                ExecutionStatus::SkipRest(mock_output)
            },
            MockTransaction::Abort => ExecutionStatus::Abort(txn_idx as usize),
            MockTransaction::InterruptRequested => {
                while !view.interrupt_requested() {}
                ExecutionStatus::SkipRest(MockOutput::skip_output())
            },
        }
    }

    fn is_transaction_dynamic_change_set_capable(_txn: &Self::Txn) -> bool {
        true
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

#[derive(Debug)]
pub(crate) struct MockOutput<K, E> {
    pub(crate) writes: Vec<(K, ValueType)>,
    pub(crate) aggregator_v1_writes: Vec<(K, ValueType)>,
    // Key, metadata_op, inner_ops
    pub(crate) group_writes: Vec<(K, ValueType, ResourceGroupSize, HashMap<u32, ValueType>)>,
    pub(crate) module_writes: Vec<ModuleWrite<ValueType>>,
    pub(crate) deltas: Vec<(K, DeltaOp)>,
    pub(crate) events: Vec<E>,
    pub(crate) read_results: Vec<Option<Vec<u8>>>,
    pub(crate) module_read_results: Vec<Option<StateValueMetadata>>,
    pub(crate) read_group_size_or_metadata: Vec<(K, GroupSizeOrMetadata)>,
    pub(crate) materialized_delta_writes: OnceCell<Vec<(K, WriteOp)>>,
    pub(crate) total_gas: u64,
    pub(crate) skipped: bool,
}

impl<K, E> TransactionOutput for MockOutput<K, E>
where
    K: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + Debug + 'static,
    E: Send + Sync + Debug + Clone + TransactionEvent + 'static,
{
    type Txn = MockTransaction<K, E>;

    // TODO[agg_v2](tests): Assigning MoveTypeLayout as None for all the writes for now.
    // That means, the resources do not have any DelayedFields embedded in them.
    // Change it to test resources with DelayedFields as well.
    fn resource_write_set(&self) -> Vec<(K, Arc<ValueType>, Option<Arc<MoveTypeLayout>>)> {
        self.writes
            .iter()
            .map(|(key, value)| (key.clone(), Arc::new(value.clone()), None))
            .collect()
    }

    fn module_write_set(&self) -> Vec<ModuleWrite<ValueType>> {
        self.module_writes.clone()
    }

    // Aggregator v1 writes are included in resource_write_set for tests (writes are produced
    // for all keys including ones for v1_aggregators without distinguishing).
    fn aggregator_v1_write_set(&self) -> BTreeMap<K, ValueType> {
        self.aggregator_v1_writes.clone().into_iter().collect()
    }

    fn aggregator_v1_delta_set(&self) -> Vec<(K, DeltaOp)> {
        self.deltas.clone()
    }

    fn delayed_field_change_set(&self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        // TODO[agg_v2](tests): add aggregators V2 to the proptest?
        BTreeMap::new()
    }

    fn reads_needing_delayed_field_exchange(
        &self,
    ) -> Vec<(
        <Self::Txn as Transaction>::Key,
        StateValueMetadata,
        Arc<MoveTypeLayout>,
    )> {
        // TODO[agg_v2](tests): add aggregators V2 to the proptest?
        Vec::new()
    }

    fn group_reads_needing_delayed_field_exchange(
        &self,
    ) -> Vec<(<Self::Txn as Transaction>::Key, StateValueMetadata)> {
        // TODO[agg_v2](tests): add aggregators V2 to the proptest?
        Vec::new()
    }

    // TODO[agg_v2](tests): Currently, appending None to all events, which means none of the
    // events have aggregators. Test it with aggregators as well.
    fn get_events(&self) -> Vec<(E, Option<MoveTypeLayout>)> {
        self.events.iter().map(|e| (e.clone(), None)).collect()
    }

    // TODO[agg_v2](cleanup) Using the concrete type layout here. Should we find a way to use generics?
    fn resource_group_write_set(
        &self,
    ) -> Vec<(
        K,
        ValueType,
        ResourceGroupSize,
        BTreeMap<u32, (ValueType, Option<Arc<MoveTypeLayout>>)>,
    )> {
        self.group_writes
            .iter()
            .cloned()
            .map(|(group_key, metadata_v, group_size, inner_ops)| {
                (
                    group_key,
                    metadata_v,
                    group_size,
                    inner_ops.into_iter().map(|(k, v)| (k, (v, None))).collect(),
                )
            })
            .collect()
    }

    fn skip_output() -> Self {
        Self {
            writes: vec![],
            aggregator_v1_writes: vec![],
            group_writes: vec![],
            module_writes: vec![],
            deltas: vec![],
            events: vec![],
            read_results: vec![],
            module_read_results: vec![],
            read_group_size_or_metadata: vec![],
            materialized_delta_writes: OnceCell::new(),
            total_gas: 0,
            skipped: true,
        }
    }

    fn discard_output(_discard_code: move_core_types::vm_status::StatusCode) -> Self {
        Self {
            writes: vec![],
            aggregator_v1_writes: vec![],
            group_writes: vec![],
            module_writes: vec![],
            deltas: vec![],
            events: vec![],
            read_results: vec![],
            module_read_results: vec![],
            read_group_size_or_metadata: vec![],
            materialized_delta_writes: OnceCell::new(),
            total_gas: 0,
            skipped: true,
        }
    }

    fn materialize_agg_v1(
        &self,
        _view: &impl TAggregatorV1View<Identifier = <Self::Txn as Transaction>::Key>,
    ) {
        // TODO[agg_v2](tests): implement this method and compare
        // against sequential execution results v. aggregator v1.
    }

    fn incorporate_materialized_txn_output(
        &self,
        aggregator_v1_writes: Vec<(<Self::Txn as Transaction>::Key, WriteOp)>,
        patched_resource_write_set: Vec<(
            <Self::Txn as Transaction>::Key,
            <Self::Txn as Transaction>::Value,
        )>,
        _patched_events: Vec<<Self::Txn as Transaction>::Event>,
    ) -> Result<(), PanicError> {
        let resources: HashMap<<Self::Txn as Transaction>::Key, <Self::Txn as Transaction>::Value> =
            patched_resource_write_set.clone().into_iter().collect();
        for (key, _, size, _) in &self.group_writes {
            let v = resources.get(key).unwrap();
            if v.is_deletion() {
                assert_eq!(*size, ResourceGroupSize::zero_combined());
            } else {
                assert_eq!(
                    size.get(),
                    resources.get(key).unwrap().bytes().map_or(0, |b| b.len()) as u64
                );
            }
        }

        assert_ok!(self.materialized_delta_writes.set(aggregator_v1_writes));
        // TODO[agg_v2](tests): Set the patched resource write set and events. But that requires the function
        // to take &mut self as input
        Ok(())
    }

    fn set_txn_output_for_non_dynamic_change_set(&self) {
        // TODO[agg_v2](tests): anything to be added here for tests?
    }

    fn fee_statement(&self) -> FeeStatement {
        // First argument is supposed to be total (not important for the test though).
        // Next two arguments are different kinds of execution gas that are counted
        // towards the block limit. We split the total into two pieces for these arguments.
        // TODO: add variety to generating fee statement based on total gas.
        FeeStatement::new(
            self.total_gas,
            self.total_gas / 2,
            (self.total_gas + 1) / 2,
            0,
            0,
        )
    }

    fn output_approx_size(&self) -> u64 {
        // TODO add block output limit testing
        0
    }

    fn get_write_summary(
        &self,
    ) -> HashSet<
        crate::types::InputOutputKey<
            <Self::Txn as Transaction>::Key,
            <Self::Txn as Transaction>::Tag,
        >,
    > {
        HashSet::new()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct MockEvent {
    event_data: Vec<u8>,
}

impl TransactionEvent for MockEvent {
    fn get_event_data(&self) -> &[u8] {
        &self.event_data
    }

    fn set_event_data(&mut self, event_data: Vec<u8>) {
        self.event_data = event_data;
    }
}

// Utility function to convert a key to a module ID
pub(crate) fn key_to_module_id<K: Clone + Hash + Debug + Ord>(key: &KeyType<K>, universe_len: usize) -> ModuleId {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let idx = (hasher.finish() % universe_len as u64) as usize;
    let mut addr = [0u8; AccountAddress::LENGTH];
    addr[AccountAddress::LENGTH - 1] = idx as u8;
    addr[AccountAddress::LENGTH - 2] = (idx >> 8) as u8;
    ModuleId::new(
        AccountAddress::new(addr),
        IdentStr::new("test").unwrap().into(),
    )
}
