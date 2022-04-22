// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    errors::{Error, Result},
    executor::MVHashMapView,
    task::{ExecutionStatus, ExecutorTask, Transaction as TransactionType, TransactionOutput},
};
use proptest::{arbitrary::Arbitrary, collection::vec, prelude::*, proptest, sample::Index};
use proptest_derive::Arbitrary;
use std::{
    collections::{BTreeSet, HashMap},
    fmt::Debug,
    hash::Hash,
    marker::PhantomData,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

///////////////////////////////////////////////////////////////////////////
// Generation of transactions
///////////////////////////////////////////////////////////////////////////

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
pub struct TransactionGen<V: Arbitrary + Debug + 'static + Clone> {
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

impl<V: Arbitrary + Debug + Clone> TransactionGen<V> {
    pub fn materialize<K: Clone + Eq + Ord>(self, universe: &[K]) -> Transaction<K, V> {
        let mut keys_modified = BTreeSet::new();
        let mut writes = vec![];

        for modified in self.keys_modified.into_iter() {
            let mut incarnation_writes: Vec<(K, V)> = vec![];
            for (idx, value) in modified.into_iter() {
                let key = universe[idx.index(universe.len())].clone();
                if !keys_modified.contains(&key) {
                    keys_modified.insert(key.clone());
                    incarnation_writes.push((key, value.clone()));
                }
            }
            writes.push(incarnation_writes);
        }

        Transaction::Write {
            incarnation: Arc::new(AtomicUsize::new(0)),
            writes,
            reads: self
                .keys_read
                .into_iter()
                .map(|keys_read| {
                    keys_read
                        .into_iter()
                        .map(|k| universe[k.index(universe.len())].clone())
                        .collect()
                })
                .collect(),
        }
    }
}

impl<K, V> TransactionType for Transaction<K, V>
where
    K: PartialOrd + Send + Sync + Clone + Hash + Eq + 'static,
    V: Send + Sync + Debug + Clone + 'static,
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
    K: PartialOrd + Send + Sync + Clone + Hash + Eq + 'static,
    V: Send + Sync + Debug + Clone + 'static,
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

pub struct Output<K, V>(Vec<(K, V)>, Vec<Option<V>>);

impl<K, V> TransactionOutput for Output<K, V>
where
    K: PartialOrd + Send + Sync + Clone + Hash + Eq + 'static,
    V: Send + Sync + Debug + Clone + 'static,
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
