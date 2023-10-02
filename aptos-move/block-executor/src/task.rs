// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::delta_change_set::DeltaOp;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    fee_statement::FeeStatement, transaction::BlockExecutableTransaction, write_set::WriteOp,
};
use aptos_vm_types::resolver::TExecutorView;
use move_core_types::value::MoveTypeLayout;
use std::{collections::HashMap, fmt::Debug};

/// The execution result of a transaction
#[derive(Debug)]
pub enum ExecutionStatus<O, E> {
    /// Transaction was executed successfully.
    Success(O),
    /// Transaction hit a none recoverable error during execution, halt the execution and propagate
    /// the error back to the caller.
    Abort(E),
    /// Transaction was executed successfully, but will skip the execution of the trailing
    /// transactions in the list
    SkipRest(O),
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
    type Txn: BlockExecutableTransaction;

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
        view: &impl TExecutorView<
            <Self::Txn as BlockExecutableTransaction>::Key,
            MoveTypeLayout,
            <Self::Txn as BlockExecutableTransaction>::Identifier,
        >,
        txn: &Self::Txn,
        txn_idx: TxnIndex,
        materialize_deltas: bool,
    ) -> ExecutionStatus<Self::Output, Self::Error>;
}

/// Trait for execution result of a single transaction.
pub trait TransactionOutput: Send + Sync + Debug {
    /// Type of transaction and its associated key and value.
    type Txn: BlockExecutableTransaction;

    /// Get the writes of a transaction from its output, separately for resources, modules and
    /// aggregator_v1.
    fn resource_write_set(
        &self,
    ) -> HashMap<
        <Self::Txn as BlockExecutableTransaction>::Key,
        <Self::Txn as BlockExecutableTransaction>::Value,
    >;

    fn module_write_set(
        &self,
    ) -> HashMap<
        <Self::Txn as BlockExecutableTransaction>::Key,
        <Self::Txn as BlockExecutableTransaction>::Value,
    >;

    fn aggregator_v1_write_set(
        &self,
    ) -> HashMap<
        <Self::Txn as BlockExecutableTransaction>::Key,
        <Self::Txn as BlockExecutableTransaction>::Value,
    >;

    /// Get the aggregator deltas of a transaction from its output.
    fn aggregator_v1_delta_set(
        &self,
    ) -> HashMap<<Self::Txn as BlockExecutableTransaction>::Key, DeltaOp>;

    /// Get the events of a transaction from its output.
    fn get_events(&self) -> Vec<<Self::Txn as BlockExecutableTransaction>::Event>;

    /// Execution output for transactions that comes after SkipRest signal.
    fn skip_output() -> Self;

    /// In parallel execution, will be called once per transaction when the output is
    /// ready to be committed. In sequential execution, won't be called (deltas are
    /// materialized and incorporated during execution).
    fn incorporate_delta_writes(
        &self,
        delta_writes: Vec<(<Self::Txn as BlockExecutableTransaction>::Key, WriteOp)>,
    );

    /// Return the fee statement of the transaction.
    fn fee_statement(&self) -> FeeStatement;
}
