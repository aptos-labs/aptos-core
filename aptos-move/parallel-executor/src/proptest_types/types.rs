// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    errors::{Error, Result},
    executor::MVHashMapView,
    task::{
        ExecutionStatus, ExecutorTask, ModulePath, Transaction as TransactionType,
        TransactionOutput,
    },
};
use aptos_types::{
    access_path::AccessPath, account_address::AccountAddress, write_set::TransactionWrite,
};
use proptest::{arbitrary::Arbitrary, collection::vec, prelude::*, proptest, sample::Index};
use proptest_derive::Arbitrary;
use std::collections::hash_map::DefaultHasher;
use std::{
    collections::{BTreeSet, HashMap},
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

#[derive(Clone, Copy, Hash, Debug, PartialEq, PartialOrd, Eq)]
pub struct KeyType<K: Hash + Clone + Debug + PartialOrd + Eq>(
    /// Wrapping the types used for testing to add ModulePath trait implementation (below).
    pub K,
    /// The bool field determines for testing purposes, whether the key will be interpreted
    /// as a module access path. In this case, if a module path is both read and written
    /// during parallel execution, Error::ModulePathReadWrite must be returned and the
    /// block execution must fall back to the sequential execution.
    pub bool,
);

impl<K: Hash + Clone + Debug + Eq + PartialOrd> ModulePath for KeyType<K> {
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
);

impl<V: Into<Vec<u8>> + Debug + Clone + Eq + Send + Sync + Arbitrary> TransactionWrite
    for ValueType<V>
{
    fn extract_raw_bytes(&self) -> Option<Vec<u8>> {
        Some(self.0.clone().into())
    }
}

impl<V: Into<Vec<u8>> + Debug + Clone + Eq + Send + Sync + Arbitrary> ValueType<V> {
    fn _extract_u128(&self) -> Option<u128> {
        let v = self.extract_raw_bytes().unwrap();
        if v.is_empty() {
            None
        } else {
            Some(v[0] as u128)
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
        /// Vector of all possible write-sets of transaction execution (chosen round-robin depending
        /// on the incarnation counter value). Each write set is a vector describing writes, each
        /// to a key with a provided value.
        writes: Vec<Vec<(K, V)>>,
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

impl<V: Into<Vec<u8>> + Arbitrary + Clone + Debug + Eq> TransactionGen<V> {
    fn writes_from_gen<K: Clone + Hash + Debug + Eq + Ord>(
        universe: &[K],
        gen: Vec<Vec<(Index, V)>>,
        module_write_fn: &dyn Fn(usize) -> bool,
    ) -> Vec<Vec<(KeyType<K>, ValueType<V>)>> {
        let mut ret = vec![];
        for write_gen in gen.into_iter() {
            let mut keys_modified = BTreeSet::new();
            let mut incarnation_writes: Vec<(KeyType<K>, ValueType<V>)> = vec![];
            for (idx, value) in write_gen.into_iter() {
                let i = idx.index(universe.len());
                let key = universe[i].clone();
                if !keys_modified.contains(&key) {
                    keys_modified.insert(key.clone());
                    incarnation_writes
                        .push((KeyType(key, module_write_fn(i)), ValueType(value.clone())));
                }
            }
            ret.push(incarnation_writes);
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
        Transaction::Write {
            incarnation: Arc::new(AtomicUsize::new(0)),
            writes: Self::writes_from_gen(universe, self.keys_modified, &|_| -> bool {
                module_access.0
            }),
            reads: Self::reads_from_gen(universe, self.keys_read, &|_| -> bool { module_access.1 }),
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

        Transaction::Write {
            incarnation: Arc::new(AtomicUsize::new(0)),
            writes: Self::writes_from_gen(universe, self.keys_modified, &is_module_write),
            reads: Self::reads_from_gen(universe, self.keys_read, &is_module_read),
        }
    }
}

impl<K, V> TransactionType for Transaction<K, V>
where
    K: PartialOrd + Send + Sync + Clone + Hash + Eq + ModulePath + 'static,
    V: Send + Sync + Debug + Clone + TransactionWrite + 'static,
{
    type Key = K;
    type Value = V;
}

///////////////////////////////////////////////////////////////////////////
// Naive transaction executor implementation.
///////////////////////////////////////////////////////////////////////////

pub struct Task<K, V>(PhantomData<(K, V)>);

impl<K, V> Task<K, V> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<K, V> ExecutorTask for Task<K, V>
where
    K: PartialOrd + Send + Sync + Clone + Hash + Eq + ModulePath + 'static,
    V: Send + Sync + Debug + Clone + TransactionWrite + 'static,
{
    type T = Transaction<K, V>;
    type Output = Output<K, V>;
    type Error = usize;
    type Argument = ();

    fn init(_argument: Self::Argument) -> Self {
        Self::new()
    }

    fn execute_transaction(
        &self,
        view: &MVHashMapView<K, V>,
        txn: &Self::T,
    ) -> ExecutionStatus<Self::Output, Self::Error> {
        match txn {
            Transaction::Write {
                incarnation,
                reads,
                writes,
            } => {
                // Use incarnation counter value as an index to determine the read-
                // and write-sets of the execution. Increment incarnation counter to
                // simulate dynamic behavior when there are multiple possible read-
                // and write-sets (i.e. each are selected round-robin).
                let idx = incarnation.fetch_add(1, Ordering::SeqCst);
                let read_idx = idx % reads.len();
                let write_idx = idx % writes.len();

                // Reads
                let mut reads_result = vec![];
                for k in reads[read_idx].iter() {
                    reads_result.push(view.read(k).map(|v| (*v).clone()));
                }
                ExecutionStatus::Success(Output(writes[write_idx].clone(), reads_result))
            }
            Transaction::SkipRest => ExecutionStatus::SkipRest(Output(vec![], vec![])),
            Transaction::Abort => ExecutionStatus::Abort(view.txn_idx()),
        }
    }
}

#[derive(Debug)]
pub struct Output<K, V>(Vec<(K, V)>, Vec<Option<V>>);

impl<K, V> TransactionOutput for Output<K, V>
where
    K: PartialOrd + Send + Sync + Clone + Hash + Eq + ModulePath + 'static,
    V: Send + Sync + Debug + Clone + TransactionWrite + 'static,
{
    type T = Transaction<K, V>;

    fn get_writes(&self) -> Vec<(K, V)> {
        self.0.clone()
    }

    fn skip_output() -> Self {
        Self(vec![], vec![])
    }
}

///////////////////////////////////////////////////////////////////////////
// Sequential Baseline implementation.
///////////////////////////////////////////////////////////////////////////

/// Sequential baseline of execution result for dummy transaction.
pub enum ExpectedOutput<V> {
    Aborted(usize),
    SkipRest(usize, Vec<Vec<Option<V>>>),
    Success(Vec<Vec<Option<V>>>),
}

impl<V: Clone + Eq> ExpectedOutput<V> {
    /// Must be invoked after parallel execution to work with dynamic read/writes.
    pub fn generate_baseline<K: Hash + Clone + Eq>(txns: &[Transaction<K, V>]) -> Self {
        let mut current_world = HashMap::new();
        let mut result_vec = vec![];
        for (idx, txn) in txns.iter().enumerate() {
            match txn {
                Transaction::Abort => return Self::Aborted(idx),
                Transaction::Write {
                    incarnation,
                    reads,
                    writes,
                } => {
                    // Determine the read and write sets of the latest incarnation
                    // of the transaction. The index for choosing the read and
                    // write sets is based on the value of the incarnation counter
                    // prior to the fetch_add during the last execution.
                    let incarnation = incarnation.load(Ordering::SeqCst);
                    // Determine the read- and write-sets of the latest incarnation
                    // during parallel execution to use for the baseline.
                    let read_set = if reads.len() == 1 {
                        // Static read-set.
                        &reads[0]
                    } else {
                        assert!(incarnation > 0, "must run after parallel execution");
                        &reads[(incarnation - 1) as usize % reads.len()]
                    };
                    let write_set = if writes.len() == 1 {
                        // Static write-set.
                        &writes[0]
                    } else {
                        assert!(incarnation > 0, "must run after parallel execution");
                        &writes[(incarnation - 1) as usize % writes.len()]
                    };

                    let mut result = vec![];
                    for k in read_set.iter() {
                        result.push(current_world.get(k).cloned());
                    }
                    for (k, v) in write_set.iter() {
                        current_world.insert(k.clone(), v.clone());
                    }
                    result_vec.push(result)
                }
                Transaction::SkipRest => return Self::SkipRest(idx, result_vec),
            }
        }
        Self::Success(result_vec)
    }

    pub fn check_output<K>(&self, results: &Result<Vec<Output<K, V>>, usize>) -> bool {
        match (self, results) {
            (Self::Aborted(i), Err(Error::UserError(idx))) => i == idx,
            (Self::SkipRest(skip_at, expected_results), Ok(results)) => {
                results
                    .iter()
                    .take(*skip_at)
                    .zip(expected_results.iter())
                    .all(|(Output(_, result), expected_results)| expected_results == result)
                    && results
                        .iter()
                        .skip(*skip_at)
                        .all(|Output(_, result)| result.is_empty())
            }
            (Self::Success(expected_results), Ok(results)) => expected_results
                .iter()
                .zip(results.iter())
                .all(|(expected_result, Output(_, result))| expected_result == result),
            _ => false,
        }
    }
}
