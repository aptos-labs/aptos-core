// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::types::InputOutputKey;
use aptos_aggregator::delayed_change::DelayedChange;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    block_executor::{output::CommittedTransactionOutput, value::ValueWithLayout},
    error::PanicError,
    fee_statement::FeeStatement,
    state_store::{state_value::StateValueMetadata, TStateView},
    transaction::{
        AuxiliaryInfoTrait, BlockExecutableTransaction as Transaction, BlockExecutableTransaction,
    },
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_types::{
    module_and_script_storage::code_storage::AptosCodeStorage,
    module_write_set::ModuleWrite,
    resolver::{
        BlockSynchronizationKillSwitch, ResourceGroupSize, TExecutorView, TResourceGroupView,
    },
};
use move_core_types::value::MoveTypeLayout;
use move_vm_runtime::execution_tracing::Trace;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Debug,
};
use triomphe::Arc as TriompheArc;

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
}

/// Trait for single threaded transaction executor.
pub trait ExecutorTask {
    /// Type of transaction and its associated key and value.
    type Txn: Transaction;

    /// Type of auxiliary info.
    type AuxiliaryInfo: AuxiliaryInfoTrait;

    /// The output of a transaction. This should contain the side effect of this transaction.
    type Output: TransactionOutput<Txn = Self::Txn> + 'static;

    /// Type of error when the executor failed to process a transaction and needs to abort.
    type Error: Debug + Clone + Send + Sync + Eq + 'static;

    /// Create an instance of the transaction executor.
    fn init(
        environment: &AptosEnvironment,
        state_view: &impl TStateView<Key = <Self::Txn as Transaction>::Key>,
        // If true, runtime checks for user payloads may not be performed during execution time.
        // Instead, the execution trace will be collected, for later async replay with extra
        // checks during Block-STM's post-commit parallel processing.
        //
        // Note: for each transaction, based on entrypoint, VM may decide not to delay the checks.
        async_runtime_checks_enabled: bool,
    ) -> Self;

    /// Execute a single transaction given the view of the current state.
    fn execute_transaction(
        &self,
        view: &(impl TExecutorView<
            <Self::Txn as Transaction>::Key,
            <Self::Txn as Transaction>::Tag,
            MoveTypeLayout,
        > + TResourceGroupView<
            GroupKey = <Self::Txn as Transaction>::Key,
            ResourceTag = <Self::Txn as Transaction>::Tag,
            Layout = MoveTypeLayout,
        > + AptosCodeStorage
              + BlockSynchronizationKillSwitch),
        txn: &Self::Txn,
        auxiliary_info: &Self::AuxiliaryInfo,
        txn_idx: TxnIndex,
    ) -> ExecutionStatus<Self::Output, Self::Error>;

    fn pre_write_values(
        _txn: &Self::Txn,
    ) -> Vec<(
        <Self::Txn as BlockExecutableTransaction>::Key,
        ValueWithLayout<<Self::Txn as BlockExecutableTransaction>::Value>,
    )> {
        vec![]
    }
}

/// Traits for execution result of a single transaction.
pub trait BeforeMaterializationOutput<Txn: Transaction> {
    /// Get the writes of a transaction from its output, separately for resources
    /// and modules. Aggregator V1 writes are ordinary entries of the resource write set.
    fn resource_write_set(&self) -> HashMap<Txn::Key, ValueWithLayout<Txn::Value>>;

    fn module_write_set(&self) -> &BTreeMap<Txn::Key, ModuleWrite<Txn::Value>>;

    /// Get the delayed field changes of a transaction from its output.
    fn delayed_field_change_set(&self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>;

    fn reads_needing_delayed_field_exchange(
        &self,
    ) -> Vec<(Txn::Key, StateValueMetadata, TriompheArc<MoveTypeLayout>)>;

    fn group_reads_needing_delayed_field_exchange(&self) -> Vec<(Txn::Key, StateValueMetadata)>;

    /// Get the events of a transaction from its output.
    fn get_events(&self) -> Vec<(Txn::Event, Option<MoveTypeLayout>)>;

    fn resource_group_write_set(
        &self,
    ) -> HashMap<
        Txn::Key,
        (
            ValueWithLayout<Txn::Value>,
            ResourceGroupSize,
            BTreeMap<Txn::Tag, ValueWithLayout<Txn::Value>>,
        ),
    >;

    fn for_each_resource_key(
        &self,
        callback: &mut dyn FnMut(&Txn::Key) -> Result<(), PanicError>,
    ) -> Result<(), PanicError>;

    fn for_each_resource_group_key_and_tags(
        &self,
        // This is &mut dyn and not Impl to sidestep an internal compiler error:
        // https://github.com/rust-lang/rust/issues/145188.
        callback: &mut dyn FnMut(&Txn::Key, HashSet<&Txn::Tag>) -> Result<(), PanicError>,
    ) -> Result<(), PanicError>;

    // For now, the below interfaces for keys and metada and keys and tags are provided
    // to avoid unnecessarily cloning the whole resource group write set.
    // TODO: get rid of these interfaces when we can have zero-copy access to the output.
    fn resource_group_metadata_ops(&self) -> Vec<(Txn::Key, Txn::Value)> {
        self.resource_group_write_set()
            .into_iter()
            .map(|(key, (op, _, _))| (key, op.extract_value().clone()))
            .collect()
    }

    fn legacy_v1_resource_group_tags(&self) -> Vec<(Txn::Key, HashSet<Txn::Tag>)> {
        self.resource_group_write_set()
            .into_iter()
            .map(|(key, (_, _, group_ops))| (key, group_ops.keys().cloned().collect()))
            .collect()
    }

    /// Return the fee statement of the transaction.
    fn fee_statement(&self) -> FeeStatement;

    fn has_new_epoch_event(&self) -> bool;

    /// Deterministic, but approximate size of the output, as
    /// before creating actual TransactionOutput, we don't know the exact size of it.
    ///
    /// Sum of all sizes of writes (keys + write_ops) and events.
    fn output_approx_size(&self) -> u64;

    fn get_write_summary(&self) -> HashSet<InputOutputKey<Txn::Key, Txn::Tag>>;

    /// State keys read by the VM during the execution that produced this output.
    fn storage_keys_read(&self) -> impl Iterator<Item = &Txn::Key>;

    /// Keys written when this output commits.
    fn storage_keys_written(&self) -> impl Iterator<Item = &Txn::Key>;
}

pub trait TransactionOutput: Send + Debug {
    /// Type of transaction and its associated key and value.
    type Txn: Transaction;
    /// The materialized output produced from this (speculative) output.
    type CommittedOutput: CommittedTransactionOutput;

    type BeforeMaterializationGuard<'a>: BeforeMaterializationOutput<Self::Txn> + 'a
    where
        Self: 'a;

    /// Execution output for transactions that comes after SkipRest signal.
    fn skip_output() -> Self;

    // Materialization transforms the stored txn output, and may require the
    // TransactionOutput implementation to have different processing for
    // extracting data from the output. Hence, it is important to make the
    // caller aware via the guard types and the chaining pattern in order to
    // ensure the appropriate methods are called.
    fn before_materialization<'a>(
        &'a self,
    ) -> Result<Self::BeforeMaterializationGuard<'a>, PanicError>;

    // Below methods perform various types of materialization. These may modify
    // the stored output representation and hence must be carefully implemented
    // to avoid data races with the accessor methods.

    /// Will be called once per transaction when the output is ready to be committed.
    /// Ensures that any writes corresponding to materialized delayed fields and group
    /// updates (recorded in output separately) are incorporated into the transaction output.
    /// !!! [CAUTION] !!!: This method must be called in quiescence, i.e. may not be
    /// concurrent with any other method that accesses the output.
    fn incorporate_materialized_txn_output(
        &mut self,
        patched_resource_write_set: Vec<(
            <Self::Txn as Transaction>::Key,
            <Self::Txn as Transaction>::Value,
        )>,
        patched_events: Vec<<Self::Txn as Transaction>::Event>,
    ) -> Result<(Self::CommittedOutput, Trace), PanicError>;
}
