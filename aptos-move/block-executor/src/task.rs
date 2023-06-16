// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::delta_change_set::DeltaOp;
use aptos_mvhashmap::types::TxnIndex;
use aptos_state_view::TStateView;
use aptos_types::{
    executable::ModulePath,
    fee_statement::FeeStatement,
    write_set::{TransactionWrite, WriteOp},
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

/// Trait that defines a transaction type that can be executed by the block executor. A transaction
/// transaction will write to a key value storage as their side effect.
pub trait Transaction: Sync + Send + Clone + 'static {
    type Key: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + Debug;
    type Value: Send + Sync + Clone + TransactionWrite;
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
    type Error: Debug + Clone + Send + Sync + Eq + 'static;

    /// Type to initialize the single thread transaction executor. Copy and Sync are required because
    /// we will create an instance of executor on each individual thread.
    type Argument: Sync + Copy;

    /// Create an instance of the transaction executor.
    fn init(args: Self::Argument) -> Self;

    /// Execute a single transaction given the view of the current state.
    fn execute_transaction(
        &self,
        view: &impl TStateView<Key = <Self::Txn as Transaction>::Key>,
        txn: &Self::Txn,
        txn_idx: TxnIndex,
        materialize_deltas: bool,
    ) -> ExecutionStatus<Self::Output, Self::Error>;
}

/// Trait for execution result of a single transaction.
pub trait TransactionOutput: Send + Sync + Debug {
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

    /// In parallel execution, will be called once per transaction when the output is
    /// ready to be committed. In sequential execution, won't be called (deltas are
    /// materialized and incorporated during execution).
    fn incorporate_delta_writes(
        &self,
        delta_writes: Vec<(<Self::Txn as Transaction>::Key, WriteOp)>,
    );

    /// Return the amount of gas consumed by the transaction.
    fn gas_used(&self) -> u64;

    /// Return the fee statement of the transaction.
    fn fee_statement(&self) -> FeeStatement;
}
