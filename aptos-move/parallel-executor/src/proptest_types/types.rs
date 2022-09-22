// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    executor::{MVHashMapView, ReadResult},
    task::{
        ExecutionStatus, ExecutorTask, ModulePath, Transaction as TransactionType,
        TransactionOutput,
    },
};
use aptos_aggregator::{
    delta_change_set::{delta_add, delta_sub, serialize, DeltaOp},
    transaction::AggregatorValue,
};
use aptos_types::{
    access_path::AccessPath, account_address::AccountAddress, write_set::TransactionWrite,
};
use proptest::{arbitrary::Arbitrary, collection::vec, prelude::*, proptest, sample::Index};
use proptest_derive::Arbitrary;
use std::{
    collections::{btree_map::BTreeMap, hash_map::DefaultHasher, BTreeSet},
    convert::TryInto,
    fmt::Debug,
    hash::{Hash, Hasher},
    marker::PhantomData,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

///////////////////////////////////////////////////////////////////////////
// Generation of transactions
///////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Hash, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum PathKind {
    Module,
    Data,
}

#[derive(Clone, Copy, Hash, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub struct KeyType<K: Hash + Clone + Debug + PartialOrd + Ord + Eq>(
    /// Wrapping the types used for testing to add ModulePath trait implementation (below).
    pub K,
    /// Determine for testing purposes, whether the key will be interpreted as a module
    /// access path. In this case, if a module path is both read and written during parallel
    /// execution, Error::ModulePathReadWrite must be returned and the block execution must
    /// fall back to the sequential execution.
    pub PathKind,
);

impl<K: Hash + Clone + Debug + Eq + PartialOrd + Ord> ModulePath for KeyType<K> {
    fn module_path(&self) -> Option<AccessPath> {
        // Since K is generic, use its hash to assign addresses.
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        let mut hashed_address = vec![1u8; AccountAddress::LENGTH - 8];
        hashed_address.extend_from_slice(&hasher.finish().to_ne_bytes());

        if self.1 == PathKind::Module {
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

pub trait FromU128 {
    fn from_u128(value: u128) -> Self;
}

impl FromU128 for [u8; 32] {
    fn from_u128(value: u128) -> Self {
        let mut v = serialize(&value);
        v.resize(32, 0);
        let mut ret: [u8; 32] = [0; 32];
        for (i, val) in v.into_iter().enumerate() {
            ret[i] = val;
        }
        ret
    }
}

impl FromU128 for Vec<u8> {
    fn from_u128(value: u128) -> Self {
        serialize(&value)
    }
}

impl<V: FromU128 + Into<Vec<u8>> + Debug + Clone + Eq + Send + Sync + Arbitrary> FromU128
    for ValueType<V>
{
    fn from_u128(value: u128) -> Self {
        ValueType(V::from_u128(value), true)
    }
}

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
        module_write_fn: &dyn Fn(usize) -> PathKind,
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
                        Some(delta) => {
                            incarnation_deltas.push((KeyType(key, PathKind::Data), delta))
                        }
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
                        }
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
        module_read_fn: &dyn Fn(usize) -> PathKind,
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
        let is_module_write = |_| -> PathKind {
            if module_access.0 {
                PathKind::Module
            } else {
                PathKind::Data
            }
        };
        let is_module_read = |_| -> PathKind {
            if module_access.1 {
                PathKind::Module
            } else {
                PathKind::Data
            }
        };
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
        limit: u128,
    ) -> Transaction<KeyType<K>, ValueType<V>> {
        let is_module_write = |_| -> PathKind { PathKind::Data };
        let is_module_read = |_| -> PathKind { PathKind::Data };
        let is_delta = |i, v: &V| -> Option<DeltaOp> {
            if i >= delta_threshold {
                let val = AggregatorValue::from_write(&ValueType(v.clone(), true))
                    .unwrap()
                    .into();
                if val % 10 == 0 {
                    None
                } else if val % 10 < 5 {
                    Some(delta_sub(val % 100, limit))
                } else {
                    Some(delta_add(val % 100, limit))
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

        let is_module_write = |i| -> PathKind {
            if i >= write_threshold {
                PathKind::Module
            } else {
                PathKind::Data
            }
        };
        let is_module_read = |i| -> PathKind {
            if i >= read_threshold && i < write_threshold {
                PathKind::Module
            } else {
                PathKind::Data
            }
        };
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
    K: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + 'static,
    V: Send + Sync + Debug + Clone + TransactionWrite + FromU128 + 'static,
{
    type Key = K;
    type Value = V;
}

///////////////////////////////////////////////////////////////////////////
// Naive transaction executor implementation.
///////////////////////////////////////////////////////////////////////////

pub struct Task<K, V>(PhantomData<(K, V)>, Option<u128>);

impl<K, V> Task<K, V> {
    pub fn new(storage_aggregator_val: Option<u128>) -> Self {
        Self(PhantomData, storage_aggregator_val)
    }
}

impl<K, V> ExecutorTask for Task<K, V>
where
    K: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + 'static,
    V: Send + Sync + Debug + Clone + TransactionWrite + FromU128 + 'static,
{
    type T = Transaction<K, V>;
    type Output = Output<K, V>;
    type Error = usize;
    type Argument = Option<u128>;

    fn init(argument: Self::Argument) -> Self {
        Self::new(argument)
    }

    fn execute_transaction_btree_view(
        &self,
        view: &BTreeMap<K, V>,
        txn: &Self::T,
        txn_idx: usize,
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
                    reads_result.push(match view.get(k) {
                        Some(val) => ReadResult::Value(Arc::new(val.clone())),
                        None => ReadResult::None,
                    });
                }

                // Sequential execution via btree view must materialize deltas.
                let mut writes = writes_and_deltas[write_idx].0.clone();
                for (k, delta) in writes_and_deltas[write_idx].1.iter() {
                    let base = match view.get(k) {
                        Some(val) => AggregatorValue::from_write(val)
                            .map(|aggregator_value| aggregator_value.into()),
                        None => Some(self.1.unwrap()),
                    };
                    match base {
                        Some(base) => {
                            // Do not allow delta failures in materialization, consistent with
                            // the current sequential execution behavior.
                            // TODO: write special 'ERROR' value and validate against baseline.
                            let val = delta.apply_to(base).expect("delta application error");
                            writes.push((k.clone(), V::from_u128(val)));
                        }
                        None => {} // Do not overwrite deletion.
                    }
                }
                ExecutionStatus::Success(Output(writes, vec![], reads_result))
            }
            Transaction::SkipRest => ExecutionStatus::SkipRest(Output(vec![], vec![], vec![])),
            Transaction::Abort => ExecutionStatus::Abort(txn_idx),
        }
    }

    fn execute_transaction_mvhashmap_view(
        &self,
        view: &MVHashMapView<K, V>,
        txn: &Self::T,
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
                    reads_result.push(view.read(k));
                }
                ExecutionStatus::Success(Output(
                    writes_and_deltas[write_idx].0.clone(),
                    writes_and_deltas[write_idx].1.clone(),
                    reads_result,
                ))
            }
            Transaction::SkipRest => ExecutionStatus::SkipRest(Output(vec![], vec![], vec![])),
            Transaction::Abort => ExecutionStatus::Abort(view.txn_idx()),
        }
    }
}

#[derive(Debug)]
pub struct Output<K, V>(
    pub Vec<(K, V)>,
    pub Vec<(K, DeltaOp)>,
    pub Vec<ReadResult<V>>,
);

impl<K, V> TransactionOutput for Output<K, V>
where
    K: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + 'static,
    V: Send + Sync + Debug + Clone + TransactionWrite + FromU128 + 'static,
{
    type T = Transaction<K, V>;

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
