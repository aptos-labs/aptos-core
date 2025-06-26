// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::types::InputOutputKey;
use aptos_aggregator::{
    delayed_change::DelayedChange, delta_change_set::DeltaOp, resolver::TAggregatorV1View,
};
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    delayed_fields::PanicError,
    fee_statement::FeeStatement,
    state_store::{state_value::StateValueMetadata, TStateView},
    transaction::BlockExecutableTransaction as Transaction,
    write_set::WriteOp,
};
use aptos_vm_types::resolver::{TExecutorView, TResourceGroupView};
use move_core_types::{value::MoveTypeLayout, vm_status::StatusCode};
use std::{
    collections::{BTreeMap, HashSet},
    fmt::Debug,
    sync::Arc,
};

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
    /// Transaction detected that it is in inconsistent state due to speculative
    /// reads it did, and needs to be re-executed.
    SpeculativeExecutionAbortError(String),
    /// Code invariant error was detected during transaction execution, which
    /// can only be caused by the bug in the code.
    DelayedFieldsCodeInvariantError(String),
    /// Transaction can't be executed because it requires a native function that is present
    /// framework, but not in the current VM implementation.
    MissingNativeFunction(String),
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

    /// Type to initialize the single thread transaction executor. Clone and Sync are required because
    /// we will create an instance of executor on each individual thread.
    type Environment: Sync + Clone;

    /// Create an instance of the transaction executor.
    fn init(
        env: Self::Environment,
        state_view: &impl TStateView<Key = <Self::Txn as Transaction>::Key>,
    ) -> Self;

    /// Execute a single transaction given the view of the current state.
    fn execute_transaction(
        &self,
        view: &(impl TExecutorView<
            <Self::Txn as Transaction>::Key,
            <Self::Txn as Transaction>::Tag,
            MoveTypeLayout,
            <Self::Txn as Transaction>::Identifier,
            <Self::Txn as Transaction>::Value,
        > + TResourceGroupView<
            GroupKey = <Self::Txn as Transaction>::Key,
            ResourceTag = <Self::Txn as Transaction>::Tag,
            Layout = MoveTypeLayout,
        >),
        txn: &Self::Txn,
        txn_idx: TxnIndex,
    ) -> ExecutionStatus<Self::Output, Self::Error>;

    fn is_transaction_dynamic_change_set_capable(txn: &Self::Txn) -> bool;
}

/// Trait for execution result of a single transaction.
pub trait TransactionOutput: Send + Sync + Debug {
    /// Type of transaction and its associated key and value.
    type Txn: Transaction;

    /// Get the writes of a transaction from its output, separately for resources, modules and
    /// aggregator_v1.
    fn resource_write_set(
        &self,
    ) -> Vec<(
        <Self::Txn as Transaction>::Key,
        Arc<<Self::Txn as Transaction>::Value>,
        Option<Arc<MoveTypeLayout>>,
    )>;

    fn module_write_set(
        &self,
    ) -> BTreeMap<<Self::Txn as Transaction>::Key, <Self::Txn as Transaction>::Value>;

    fn aggregator_v1_write_set(
        &self,
    ) -> BTreeMap<<Self::Txn as Transaction>::Key, <Self::Txn as Transaction>::Value>;

    /// Get the aggregator V1 deltas of a transaction from its output.
    fn aggregator_v1_delta_set(&self) -> Vec<(<Self::Txn as Transaction>::Key, DeltaOp)>;

    /// Get the delayed field changes of a transaction from its output.
    fn delayed_field_change_set(
        &self,
    ) -> BTreeMap<
        <Self::Txn as Transaction>::Identifier,
        DelayedChange<<Self::Txn as Transaction>::Identifier>,
    >;

    fn reads_needing_delayed_field_exchange(
        &self,
    ) -> Vec<(
        <Self::Txn as Transaction>::Key,
        StateValueMetadata,
        Arc<MoveTypeLayout>,
    )>;

    fn group_reads_needing_delayed_field_exchange(
        &self,
    ) -> Vec<(<Self::Txn as Transaction>::Key, StateValueMetadata)>;

    /// Get the events of a transaction from its output.
    fn get_events(&self) -> Vec<(<Self::Txn as Transaction>::Event, Option<MoveTypeLayout>)>;

    fn resource_group_write_set(
        &self,
    ) -> Vec<(
        <Self::Txn as Transaction>::Key,
        <Self::Txn as Transaction>::Value,
        BTreeMap<
            <Self::Txn as Transaction>::Tag,
            (
                <Self::Txn as Transaction>::Value,
                Option<Arc<MoveTypeLayout>>,
            ),
        >,
    )>;

    fn resource_group_metadata_ops(
        &self,
    ) -> Vec<(
        <Self::Txn as Transaction>::Key,
        <Self::Txn as Transaction>::Value,
    )> {
        self.resource_group_write_set()
            .into_iter()
            .map(|(key, op, _)| (key, op))
            .collect()
    }

    /// Execution output for transactions that comes after SkipRest signal.
    fn skip_output() -> Self;

    /// Execution output for transactions that should be discarded.
    fn discard_output(discard_code: StatusCode) -> Self;

    fn materialize_agg_v1(
        &self,
        view: &impl TAggregatorV1View<Identifier = <Self::Txn as Transaction>::Key>,
    );

    /// Will be called once per transaction when the output is ready to be committed.
    /// Ensures that any writes corresponding to materialized deltas and group updates
    /// (recorded in output separately) are incorporated into the transaction output.
    fn incorporate_materialized_txn_output(
        &self,
        aggregator_v1_writes: Vec<(<Self::Txn as Transaction>::Key, WriteOp)>,
        patched_resource_write_set: Vec<(
            <Self::Txn as Transaction>::Key,
            <Self::Txn as Transaction>::Value,
        )>,
        patched_events: Vec<<Self::Txn as Transaction>::Event>,
    ) -> Result<(), PanicError>;

    fn set_txn_output_for_non_dynamic_change_set(&self);

    /// Return the fee statement of the transaction.
    fn fee_statement(&self) -> FeeStatement;

    /// Deterministic, but approximate size of the output, as
    /// before creating actual TransactionOutput, we don't know the exact size of it.
    ///
    /// Sum of all sizes of writes (keys + write_ops) and events.
    fn output_approx_size(&self) -> u64;

    fn get_write_summary(
        &self,
    ) -> HashSet<
        InputOutputKey<
            <Self::Txn as Transaction>::Key,
            <Self::Txn as Transaction>::Tag,
            <Self::Txn as Transaction>::Identifier,
        >,
    >;
}
