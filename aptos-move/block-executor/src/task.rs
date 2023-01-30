// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::delta_change_set::DeltaOp;
use aptos_state_view::TStateView;
use aptos_types::{
    access_path::AccessPath, state_store::state_key::StateKey, write_set::TransactionWrite,
};
use std::{fmt::Debug, hash::Hash};

/// The execution result of a transaction
#[derive(Debug)]
pub enum ExecutionStatus<T, E> {
    /// Transaction was executed successfully.
    Success(T),
    /// Transaction hit a none recoverable error during execution, halt the execution and propagate
    /// the error back to the caller.
    Abort(E),
    /// Transaction was executed successfully, but will skip the execution of the trailing
    /// transactions in the list
    SkipRest(T),
}

pub trait ModulePath {
    fn module_path(&self) -> Option<AccessPath>;
}

impl ModulePath for StateKey {
    fn module_path(&self) -> Option<AccessPath> {
        if let StateKey::AccessPath(ap) = self {
            if ap.is_code() {
                return Some(ap.clone());
            }
        }
        None
    }
}

/// Trait that defines a transaction that could be parallel executed by the scheduler. Each
/// transaction will write to a key value storage as their side effect.
pub trait Transaction: Sync + Send + 'static {
    type Key: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + Debug;
    type Value: Send + Sync + TransactionWrite;
}

/// Inference result of a transaction.
pub struct Accesses<K> {
    pub keys_read: Vec<K>,
    pub keys_written: Vec<K>,
}

/// Trait for single threaded transaction executor.
// TODO: Sync should not be required. Sync is only introduced because this trait occurs as a phantom type of executor struct.
pub trait ExecutorTask: Sync {
    /// Type of transaction and its associated key and value.
    type Txn: Transaction;

    /// The output of a transaction. This should contain the side effect of this transaction.
    type Output: TransactionOutput<Txn = Self::Txn> + 'static;

    /// Type of error when the executor failed to process a transaction and needs to abort.
    type Error: Clone + Send + Sync + Eq + 'static;

    /// Type to intialize the single thread transaction executor. Copy and Sync are required because
    /// we will create an instance of executor on each individual thread.
    type Argument: Sync + Copy;

    /// Create an instance of the transaction executor.
    fn init(args: Self::Argument) -> Self;

    /// Execute a single transaction given the view of the current state.
    fn execute_transaction(
        &self,
        view: &impl TStateView<Key = <Self::Txn as Transaction>::Key>,
        txn: &Self::Txn,
        txn_idx: usize,
        materialize_deltas: bool,
    ) -> ExecutionStatus<Self::Output, Self::Error>;
}

/// Trait for execution result of a single transaction.
pub trait TransactionOutput: Send + Sync {
    /// Type of transaction and its associated key and value.
    type Txn: Transaction;

    /// Get the writes of a transaction from its output.
    fn get_writes(
        &self,
    ) -> Vec<(
        <Self::Txn as Transaction>::Key,
        <Self::Txn as Transaction>::Value,
    )>;

    /// Get the deltas of a transaction from its output.
    fn get_deltas(&self) -> Vec<(<Self::Txn as Transaction>::Key, DeltaOp)>;

    /// Execution output for transactions that comes after SkipRest signal.
    fn skip_output() -> Self;
}
