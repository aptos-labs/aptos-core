// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    errors::{Error, Result},
    scheduler::TxnIndex,
    task::{
        ExecutionStatus, ExecutorTask, ModulePath, Transaction as TransactionType,
        TransactionOutput,
    },
};
use aptos_aggregator::{
    delta_change_set::{delta_add, delta_sub, deserialize, serialize, DeltaOp},
    transaction::AggregatorValue,
};
use aptos_state_view::{StateViewId, TStateView};
use aptos_types::{
    access_path::AccessPath,
    account_address::AccountAddress,
    state_store::state_storage_usage::StateStorageUsage,
    write_set::{TransactionWrite, WriteOp},
};
use claims::assert_none;
use proptest::{arbitrary::Arbitrary, collection::vec, prelude::*, proptest, sample::Index};
use proptest_derive::Arbitrary;
use std::{
    collections::{hash_map::DefaultHasher, BTreeSet, HashMap},
    convert::TryInto,
    fmt::Debug,
    hash::{Hash, Hasher},
    marker::PhantomData,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

// Should not be possible to overflow or underflow, as each delta is at
// most 100 in the tests.
pub(crate) const STORAGE_AGGREGATOR_VALUE: u128 = 100001;

pub(crate) struct DeltaDataView<K, V> {
    pub(crate) phantom: PhantomData<(K, V)>,
}

impl<K, V> TStateView for DeltaDataView<K, V>
where
    K: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + 'static,
    V: Debug + Send + Sync + Debug + Clone + TransactionWrite + 'static,
{
    type Key = K;

    /// Gets the state value for a given state key.
    fn get_state_value(&self, _: &K) -> anyhow::Result<Option<Vec<u8>>> {
        // When aggregator value has to be resolved from storage, pretend it is 100.
        Ok(Some(serialize(&STORAGE_AGGREGATOR_VALUE)))
    }

    fn id(&self) -> StateViewId {
        StateViewId::Miscellaneous
    }

    fn is_genesis(&self) -> bool {
        unreachable!();
    }

    fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
        unreachable!();
    }
}

pub(crate) struct EmptyDataView<K, V> {
    pub(crate) phantom: PhantomData<(K, V)>,
}

impl<K, V> TStateView for EmptyDataView<K, V>
where
    K: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + 'static,
    V: Debug + Send + Sync + Debug + Clone + TransactionWrite + 'static,
{
    type Key = K;

    /// Gets the state value for a given state key.
    fn get_state_value(&self, _: &K) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(None)
    }

    fn id(&self) -> StateViewId {
        StateViewId::Miscellaneous
    }

    fn is_genesis(&self) -> bool {
        unreachable!();
    }

    fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
        unreachable!();
    }
}

///////////////////////////////////////////////////////////////////////////
// Generation of transactions
///////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Hash, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub struct KeyType<K: Hash + Clone + Debug + PartialOrd + Ord + Eq>(
    /// Wrapping the types used for testing to add ModulePath trait implementation (below).
    pub K,
    /// The bool field determines for testing purposes, whether the key will be interpreted
    /// as a module access path. In this case, if a module path is both read and written
    /// during parallel execution, Error::ModulePathReadWrite must be returned and the
    /// block execution must fall back to the sequential execution.
    pub bool,
);

impl<K: Hash + Clone + Debug + Eq + PartialOrd + Ord> ModulePath for KeyType<K> {
    fn module_path(&self) -> Option<AccessPath> {
        // Since K is generic, use its hash to assign addresses.
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        let mut hashed_address = vec![1u8; AccountAddress::LENGTH - 8];
        hashed_address.extend_from_slice(&hasher.finish().to_ne_bytes());

        if self.1 {
            Some(AccessPath {
                address: AccountAddress::new(hashed_address.try_into().unwrap()),
                path: b"/foo/b".to_vec(),
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Arbitrary)]
pub struct ValueType<V: Into<Vec<u8>> + Debug + Clone + Eq + Arbitrary>(
    /// Wrapping the types used for testing to add TransactionWrite trait implementation (below).
    pub V,
    /// Determines whether V is going to contain a value (o.w. deletion). This is useful for
    /// testing the bahavior of deleting aggregators, in which case we shouldn't panic
    /// but let the Move-VM handle the read the same as for any deleted resource.
    pub bool,
);

impl<V: Into<Vec<u8>> + Debug + Clone + Eq + Send + Sync + Arbitrary> TransactionWrite
    for ValueType<V>
{
    fn extract_raw_bytes(&self) -> Option<Vec<u8>> {
        if self.1 {
            let mut v = self.0.clone().into();
            v.resize(16, 1);
            Some(v)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
pub struct TransactionGenParams {
    /// Each transaction's write-set consists of between 1 and write_size-1 many writes.
    pub write_size: usize,
    /// Each transaction's read-set consists of between 1 and read_size-1 many reads.
    pub read_size: usize,
    /// The number of different read- and write-sets that an execution of the transaction may have
    /// is going to be between 1 and read_write_alternatives-1, i.e. read_write_alternatives = 2
    /// corresponds to a static transaction, while read_write_alternatives > 1 may lead to dynamic
    /// behavior when executing different incarnations of the transaction.
    pub read_write_alternatives: usize,
}

#[derive(Arbitrary, Debug, Clone)]
#[proptest(params = "TransactionGenParams")]
pub struct TransactionGen<V: Into<Vec<u8>> + Arbitrary + Clone + Debug + Eq + 'static> {
    /// Generate keys and values for possible write-sets based on above transaction gen parameters.
    #[proptest(
        strategy = "vec(vec((any::<Index>(), any::<V>()), 1..params.write_size), 1..params.read_write_alternatives)"
    )]
    keys_modified: Vec<Vec<(Index, V)>>,
    /// Generate keys for possible read-sets of the transaction based on the above parameters.
    #[proptest(
        strategy = "vec(vec(any::<Index>(), 1..params.read_size), 1..params.read_write_alternatives)"
    )]
    keys_read: Vec<Vec<Index>>,
}

/// A naive transaction that could be used to test the correctness and throughput of the system.
/// To test transaction behavior where reads and writes might be dynamic (depend on previously
/// read values), different read and writes sets are generated and used depending on the incarnation
/// counter value. Each execution of the transaction increments the incarnation counter, and its
/// value determines the index for choosing the read & write sets of the particular execution.
#[derive(Debug, Clone)]
pub enum Transaction<K, V> {
    Write {
        /// Incarnation counter for dynamic behavior i.e. incarnations differ in reads and writes.
        incarnation: Arc<AtomicUsize>,
        /// Vector of all possible write-sets and delta-sets of transaction execution (chosen round-robin
        /// depending on the incarnation counter value). Each write set is a vector describing writes, each
        /// to a key with a provided value. Each delta-set contains keys and the corresponding DeltaOps.
        writes_and_deltas: Vec<(Vec<(K, V)>, Vec<(K, DeltaOp)>)>,
        /// Vector of all possible read-sets of the transaction execution (chosen round-robin depending
        /// on the incarnation counter value). Each read set is a vector of keys that are read.
        reads: Vec<Vec<K>>,
    },
    /// Skip the execution of trailing transactions.
    SkipRest,
    /// Abort the execution.
    Abort,
}

impl TransactionGenParams {
    pub fn new_dynamic() -> Self {
        TransactionGenParams {
            write_size: 5,
            read_size: 10,
            read_write_alternatives: 4,
        }
    }
}

impl Default for TransactionGenParams {
    fn default() -> Self {
        TransactionGenParams {
            write_size: 5,
            read_size: 10,
            read_write_alternatives: 2,
        }
    }
}

impl<V: Into<Vec<u8>> + Arbitrary + Clone + Debug + Eq + Sync + Send> TransactionGen<V> {
    fn writes_and_deltas_from_gen<K: Clone + Hash + Debug + Eq + Ord>(
        universe: &[K],
        gen: Vec<Vec<(Index, V)>>,
        module_write_fn: &dyn Fn(usize) -> bool,
        delta_fn: &dyn Fn(usize, &V) -> Option<DeltaOp>,
        allow_deletes: bool,
    ) -> Vec<(Vec<(KeyType<K>, ValueType<V>)>, Vec<(KeyType<K>, DeltaOp)>)> {
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
                        Some(delta) => incarnation_deltas.push((KeyType(key, false), delta)),
                        None => {
                            // One out of 23 writes will be a deletion
                            let is_deletion = allow_deletes
                                && AggregatorValue::from_write(&ValueType(value.clone(), true))
                                    .unwrap()
                                    .into()
                                    % 23
                                    == 0;
                            incarnation_writes.push((
                                KeyType(key, module_write_fn(i)),
                                ValueType(value.clone(), !is_deletion),
                            ));
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
        module_read_fn: &dyn Fn(usize) -> bool,
    ) -> Vec<Vec<KeyType<K>>> {
        let mut ret = vec![];
        for read_gen in gen.into_iter() {
            let mut incarnation_reads: Vec<KeyType<K>> = vec![];
            for idx in read_gen.into_iter() {
                let i = idx.index(universe.len());
                let key = universe[i].clone();
                incarnation_reads.push(KeyType(key, module_read_fn(i)));
            }
            ret.push(incarnation_reads);
        }
        ret
    }

    pub fn materialize<K: Clone + Hash + Debug + Eq + Ord>(
        self,
        universe: &[K],
        // Are writes and reads module access (same access path).
        module_access: (bool, bool),
    ) -> Transaction<KeyType<K>, ValueType<V>> {
        let is_module_write = |_| -> bool { module_access.0 };
        let is_module_read = |_| -> bool { module_access.1 };
        let is_delta = |_, _: &V| -> Option<DeltaOp> { None };

        Transaction::Write {
            incarnation: Arc::new(AtomicUsize::new(0)),
            writes_and_deltas: Self::writes_and_deltas_from_gen(
                universe,
                self.keys_modified,
                &is_module_write,
                &is_delta,
                true,
            ),
            reads: Self::reads_from_gen(universe, self.keys_read, &is_module_read),
        }
    }

    pub fn materialize_with_deltas<K: Clone + Hash + Debug + Eq + Ord>(
        self,
        universe: &[K],
        delta_threshold: usize,
        allow_deletes: bool,
    ) -> Transaction<KeyType<K>, ValueType<V>> {
        let is_module_write = |_| -> bool { false };
        let is_module_read = |_| -> bool { false };
        let is_delta = |i, v: &V| -> Option<DeltaOp> {
            if i >= delta_threshold {
                let val = AggregatorValue::from_write(&ValueType(v.clone(), true))
                    .unwrap()
                    .into();
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

        Transaction::Write {
            incarnation: Arc::new(AtomicUsize::new(0)),
            writes_and_deltas: Self::writes_and_deltas_from_gen(
                universe,
                self.keys_modified,
                &is_module_write,
                &is_delta,
                allow_deletes,
            ),
            reads: Self::reads_from_gen(universe, self.keys_read, &is_module_read),
        }
    }

    pub fn materialize_disjoint_module_rw<K: Clone + Hash + Debug + Eq + Ord>(
        self,
        universe: &[K],
        // keys generated with indices from read_threshold to write_threshold will be
        // treated as module access only in reads. keys generated with indices from
        // write threshold to universe.len() will be treated as module access only in
        // writes. This way there will be module accesses but no intersection.
        read_threshold: usize,
        write_threshold: usize,
    ) -> Transaction<KeyType<K>, ValueType<V>> {
        assert!(read_threshold < universe.len());
        assert!(write_threshold > read_threshold);
        assert!(write_threshold < universe.len());

        let is_module_write = |i| -> bool { i >= write_threshold };
        let is_module_read = |i| -> bool { i >= read_threshold && i < write_threshold };
        let is_delta = |_, _: &V| -> Option<DeltaOp> { None };

        Transaction::Write {
            incarnation: Arc::new(AtomicUsize::new(0)),
            writes_and_deltas: Self::writes_and_deltas_from_gen(
                universe,
                self.keys_modified,
                &is_module_write,
                &is_delta,
                true,
            ),
            reads: Self::reads_from_gen(universe, self.keys_read, &is_module_read),
        }
    }
}

impl<K, V> TransactionType for Transaction<K, V>
where
    K: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + Debug + 'static,
    V: Debug + Send + Sync + Debug + Clone + TransactionWrite + 'static,
{
    type Key = K;
    type Value = V;
}

///////////////////////////////////////////////////////////////////////////
// Naive transaction executor implementation.
///////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct Task<K, V>(PhantomData<(K, V)>);

impl<K, V> Task<K, V> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<K, V> ExecutorTask for Task<K, V>
where
    K: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + Debug + 'static,
    V: Send + Sync + Debug + Clone + TransactionWrite + 'static,
{
    type Argument = ();
    type Error = usize;
    type Output = Output<K, V>;
    type Txn = Transaction<K, V>;

    fn init(_argument: Self::Argument) -> Self {
        Self::new()
    }

    fn execute_transaction(
        &self,
        view: &impl TStateView<Key = K>,
        txn: &Self::Txn,
        txn_idx: TxnIndex,
        _materialize_deltas: bool,
    ) -> ExecutionStatus<Self::Output, Self::Error> {
        match txn {
            Transaction::Write {
                incarnation,
                reads,
                writes_and_deltas,
            } => {
                // Use incarnation counter value as an index to determine the read-
                // and write-sets of the execution. Increment incarnation counter to
                // simulate dynamic behavior when there are multiple possible read-
                // and write-sets (i.e. each are selected round-robin).
                let idx = incarnation.fetch_add(1, Ordering::SeqCst);
                let read_idx = idx % reads.len();
                let write_idx = idx % writes_and_deltas.len();

                // Reads
                let mut reads_result = vec![];
                for k in reads[read_idx].iter() {
                    // TODO: later test errors as well? (by fixing state_view behavior).
                    reads_result.push(view.get_state_value(k).unwrap());
                }
                ExecutionStatus::Success(Output(
                    writes_and_deltas[write_idx].0.clone(),
                    writes_and_deltas[write_idx].1.clone(),
                    reads_result,
                ))
            },
            Transaction::SkipRest => ExecutionStatus::SkipRest(Output(vec![], vec![], vec![])),
            Transaction::Abort => ExecutionStatus::Abort(txn_idx),
        }
    }
}

#[derive(Debug)]
pub struct Output<K, V>(Vec<(K, V)>, Vec<(K, DeltaOp)>, Vec<Option<Vec<u8>>>);

impl<K, V> TransactionOutput for Output<K, V>
where
    K: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + Debug + 'static,
    V: Send + Sync + Debug + Clone + TransactionWrite + 'static,
{
    type Txn = Transaction<K, V>;

    fn get_writes(&self) -> Vec<(K, V)> {
        self.0.clone()
    }

    fn get_deltas(&self) -> Vec<(K, DeltaOp)> {
        self.1.clone()
    }

    fn skip_output() -> Self {
        Self(vec![], vec![], vec![])
    }
}

///////////////////////////////////////////////////////////////////////////
// Sequential Baseline implementation.
///////////////////////////////////////////////////////////////////////////

/// Sequential baseline of execution result for dummy transaction.
pub enum ExpectedOutput<V> {
    Aborted(usize),
    SkipRest(usize, Vec<Vec<(Option<V>, Option<u128>)>>),
    Success(Vec<Vec<(Option<V>, Option<u128>)>>),
    DeltaFailure(usize, Vec<Vec<(Option<V>, Option<u128>)>>),
}

impl<V: Debug + Clone + PartialEq + Eq + TransactionWrite> ExpectedOutput<V> {
    /// Must be invoked after parallel execution to work with dynamic read/writes.
    pub fn generate_baseline<K: Hash + Clone + Eq>(
        txns: &[Transaction<K, V>],
        resolved_deltas: Option<Vec<Vec<(K, WriteOp)>>>,
    ) -> Self {
        let mut current_world = HashMap::new();
        // Delta world stores the latest u128 value of delta aggregator. When empty, the
        // value is derived based on deserializing current_world, or falling back to
        // STORAGE_AGGREGATOR_VAL.
        let mut delta_world = HashMap::new();

        let mut result_vec = vec![];
        for (idx, txn) in txns.iter().enumerate() {
            let delta_writes_at_idx = resolved_deltas.as_ref().map(|delta_writes| {
                delta_writes[idx]
                    .iter()
                    .cloned()
                    .collect::<HashMap<K, WriteOp>>()
            });

            match txn {
                Transaction::Abort => return Self::Aborted(idx),
                Transaction::Write {
                    incarnation,
                    writes_and_deltas,
                    reads,
                } => {
                    // Determine the read and write sets of the latest incarnation
                    // of the transaction. The index for choosing the read and
                    // write sets is based on the value of the incarnation counter
                    // prior to the fetch_add during the last execution.
                    let incarnation = incarnation.load(Ordering::SeqCst);

                    if reads.len() == 1 || writes_and_deltas.len() == 1 {
                        assert!(incarnation > 0, "must run after parallel execution");
                    }

                    // Determine the read-, delta- and write-sets of the latest
                    // incarnation during parallel execution to use for the baseline.
                    let read_set = &reads[(incarnation - 1) % reads.len()];
                    let (write_set, delta_set) =
                        &writes_and_deltas[(incarnation - 1) % writes_and_deltas.len()];

                    let mut result = vec![];
                    for k in read_set.iter() {
                        result.push((current_world.get(k).cloned(), delta_world.get(k).cloned()));
                    }

                    // We ensure that the latest state is always reflected in exactly one of
                    // the hashmaps, by possibly removing an element from the other Hashmap.
                    for (k, v) in write_set.iter() {
                        delta_world.remove(k);
                        current_world.insert(k.clone(), v.clone());
                    }

                    for (k, delta) in delta_set.iter() {
                        let latest_write = current_world.remove(k);

                        match delta_writes_at_idx.as_ref() {
                            Some(delta_writes) => {
                                assert_eq!(delta_writes.len(), delta_set.len());
                                delta_world.insert(
                                    k.clone(),
                                    AggregatorValue::from_write(delta_writes.get(k).unwrap())
                                        .unwrap()
                                        .into(),
                                );
                            },
                            None => {
                                let base = match (&latest_write, delta_world.remove(k)) {
                                    (Some(_), Some(_)) => {
                                        unreachable!(
                                            "Must record latest value or resolved delta, not both"
                                        );
                                    },
                                    // Get base value from the latest write.
                                    (Some(w_value), None) => AggregatorValue::from_write(w_value)
                                        .map(|value| value.into()),
                                    // Get base value from latest resolved aggregator value.
                                    (None, Some(value)) => Some(value),
                                    // Storage always gets resolved to a default constant.
                                    (None, None) => Some(STORAGE_AGGREGATOR_VALUE),
                                };

                                match base {
                                    Some(base) => {
                                        let applied_delta = delta.apply_to(base);
                                        if applied_delta.is_err() {
                                            return Self::DeltaFailure(idx, result_vec);
                                        }
                                        delta_world.insert(k.clone(), applied_delta.unwrap());
                                    },
                                    None => {
                                        // Latest write was a deletion, can't resolve any delta to
                                        // it, must keep the deletion as the latest Op.
                                        current_world.insert(k.clone(), latest_write.unwrap());
                                    },
                                }
                            },
                        }
                    }

                    result_vec.push(result)
                },
                Transaction::SkipRest => return Self::SkipRest(idx, result_vec),
            }
        }
        Self::Success(result_vec)
    }

    fn check_result(expected_results: &[(Option<V>, Option<u128>)], results: &[Option<Vec<u8>>]) {
        expected_results
            .iter()
            .zip(results.iter())
            .for_each(|(expected_result, result)| match result {
                Some(value) => match expected_result {
                    (Some(v), None) => {
                        assert_eq!(v.extract_raw_bytes().unwrap(), *value);
                    },
                    (None, Some(v)) => {
                        assert_eq!(serialize(v), *value);
                    },
                    (Some(_), Some(_)) => unreachable!("A"),
                    (None, None) => {
                        assert_eq!(deserialize(value), STORAGE_AGGREGATOR_VALUE);
                    },
                },
                None => {
                    if let Some(val) = &expected_result.0 {
                        assert_none!(val.extract_raw_bytes());
                    }
                    assert_eq!(expected_result.1, None);
                },
            })
    }

    // Used for testing, hence the function asserts the correctness conditions within
    // itself to be easily traceable in case of an error.
    pub fn assert_output<K>(&self, results: &Result<Vec<Output<K, V>>, usize>) {
        match (self, results) {
            (Self::Aborted(i), Err(Error::UserError(idx))) => {
                assert_eq!(i, idx);
            },
            (Self::SkipRest(skip_at, expected_results), Ok(results)) => {
                // Check_result asserts internally, so no need to return a bool.
                results
                    .iter()
                    .take(*skip_at)
                    .zip(expected_results.iter())
                    .for_each(|(Output(_, _, result), expected_results)| {
                        Self::check_result(expected_results, result)
                    });

                results
                    .iter()
                    .skip(*skip_at)
                    .for_each(|Output(_, _, result)| assert!(result.is_empty()))
            },
            (Self::DeltaFailure(fail_idx, expected_results), Ok(results)) => {
                // Check_result asserts internally, so no need to return a bool.
                results
                    .iter()
                    .take(*fail_idx)
                    .zip(expected_results.iter())
                    .for_each(|(Output(_, _, result), expected_results)| {
                        Self::check_result(expected_results, result)
                    });
            },
            (Self::Success(expected_results), Ok(results)) => results
                .iter()
                .zip(expected_results.iter())
                .for_each(|(Output(_, _, result), expected_result)| {
                    Self::check_result(expected_result, result);
                }),
            _ => panic!("Incomparable execution outcomes"),
        }
    }
}
