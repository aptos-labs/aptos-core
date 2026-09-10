// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{executor_utilities::Materializer, types::InputOutputKey};
use aptos_aggregator::delayed_change::DelayedChange;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    block_executor::{
        output::CommittedTransactionOutput,
        value::{SpeculativeValue, ValueWithLayout},
    },
    error::PanicError,
    fee_statement::FeeStatement,
    state_store::{
        state_value::{StateValue, StateValueMetadata},
        TStateView,
    },
    transaction::{
        AuxiliaryInfoTrait, BlockExecutableTransaction as Transaction, BlockExecutableTransaction,
    },
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_types::{
    module_and_script_storage::code_storage::AptosCodeStorage,
    resolver::{
        BlockSynchronizationKillSwitch, ResourceGroupSize, TExecutorView, TResourceGroupView,
    },
};
use move_core_types::{language_storage::ModuleId, value::MoveTypeLayout};
use move_vm_runtime::execution_tracing::Trace;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Debug,
    hash::Hash,
};
use triomphe::Arc as TriompheArc;

/// The outcome of executing a transaction.
#[derive(Debug)]
pub enum ExecutionStatus<O> {
    /// Transaction was executed successfully.
    Executed { output: O, skips_rest: bool },
    /// Transaction hit a non-recoverable error during execution. Halts execution
    /// and returns the error message to the caller.
    Aborted(String),
    /// Speculative failure during parallel execution; the transaction
    /// re-executes.
    SpeculativeFailure,
}

impl<O> ExecutionStatus<O> {
    /// The output, present only for an executed transaction.
    pub(crate) fn get_output(&self) -> Option<&O> {
        match self {
            ExecutionStatus::Executed { output, .. } => Some(output),
            ExecutionStatus::Aborted(_) | ExecutionStatus::SpeculativeFailure => None,
        }
    }

    pub(crate) fn is_speculative_failure(&self) -> bool {
        matches!(self, ExecutionStatus::SpeculativeFailure)
    }
}

/// Trait for single threaded transaction executor.
pub trait ExecutorTask {
    /// Type of transaction and its associated key and value.
    type Txn: Transaction;

    /// Type of auxiliary info.
    type AuxiliaryInfo: AuxiliaryInfoTrait;

    /// The output of a transaction. This should contain the side effect of this transaction.
    type Output: LegacyTxnOutput<
            Txn = Self::Txn,
            Key = <Self::Txn as Transaction>::Key,
            Tag = <Self::Txn as Transaction>::Tag,
            Value = ValueWithLayout<<Self::Txn as Transaction>::Value>,
        > + 'static;

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
    ) -> Result<ExecutionStatus<Self::Output>, PanicError>;

    fn pre_write_values(
        _txn: &Self::Txn,
    ) -> Vec<(
        <Self::Txn as BlockExecutableTransaction>::Key,
        ValueWithLayout<<Self::Txn as BlockExecutableTransaction>::Value>,
    )> {
        vec![]
    }
}

/// The output of executing a single transaction. The associated `Key`, `Tag`, and
/// `Value` are the multi-version map's types (the writes applied to and validated
/// against the map); `Txn` provides the storage-side key/value/event types.
pub trait TxnOutput: Send + Debug {
    type Txn: Transaction;
    type Key: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + Debug + 'static;
    type Tag: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + Debug + Serialize + 'static;
    type Value: SpeculativeValue + 'static;
    /// The materialized output produced from this (speculative) output.
    type CommittedOutput: CommittedTransactionOutput;

    /// Execution output for transactions that comes after SkipRest signal.
    fn skip_output() -> Self;

    /// Get the writes of a transaction from its output, separately for resources
    /// and modules.
    fn resource_write_set(&self) -> HashMap<Self::Key, Self::Value>;

    /// Get the delayed field changes of a transaction from its output.
    fn delayed_field_change_set(&self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>;

    fn resource_group_write_set(
        &self,
    ) -> HashMap<
        Self::Key,
        (
            Self::Value,
            ResourceGroupSize,
            BTreeMap<Self::Tag, Self::Value>,
        ),
    >;

    fn for_each_resource_key(
        &self,
        callback: &mut dyn FnMut(&Self::Key) -> Result<(), PanicError>,
    ) -> Result<(), PanicError>;

    fn for_each_resource_group_key_and_tags(
        &self,
        // This is &mut dyn and not Impl to sidestep an internal compiler error:
        // https://github.com/rust-lang/rust/issues/145188.
        callback: &mut dyn FnMut(&Self::Key, HashSet<&Self::Tag>) -> Result<(), PanicError>,
    ) -> Result<(), PanicError>;

    /// Invokes the callback for each module published by this transaction.
    /// Modules cannot be deleted, so all writes are concrete state values.
    fn for_each_module_write(
        &self,
        callback: &mut dyn FnMut(&ModuleId, StateValue) -> Result<(), PanicError>,
    ) -> Result<(), PanicError>;

    /// Returns the key and member tags for each resource group modified. Only
    /// used by Block-STM v1.
    fn legacy_v1_resource_group_tags(&self) -> Vec<(Self::Key, HashSet<Self::Tag>)> {
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

    /// Keys written by transaction.
    fn get_write_summary(&self) -> HashSet<InputOutputKey<Self::Key, Self::Tag>>;

    /// State keys read by the VM during the execution that produced this output.
    fn storage_keys_read(&self) -> impl Iterator<Item = &Self::Key>;

    /// Keys written when this output commits.
    fn storage_keys_written(&self) -> impl Iterator<Item = &Self::Key>;

    /// Verifies that this transaction's output can be materialized (e.g., outputs
    /// BCS-serialized into storage representation).
    ///
    /// Only invoked during the resource-group serialization fallback.
    fn check_materialization(&self, materializer: &impl Materializer<Self::Txn>) -> bool;
}

/// Output accessors used only by the legacy Move VM's materialization; a VM that
/// materializes its own output does not implement this.
pub trait LegacyTxnOutput: TxnOutput {
    fn reads_needing_delayed_field_exchange(
        &self,
    ) -> Vec<(Self::Key, StateValueMetadata, TriompheArc<MoveTypeLayout>)>;

    fn group_reads_needing_delayed_field_exchange(&self) -> Vec<(Self::Key, StateValueMetadata)>;

    /// Get the events of a transaction from its output.
    fn get_events(&self) -> Vec<(<Self::Txn as Transaction>::Event, Option<MoveTypeLayout>)>;

    /// Group metadata write ops, kept separate to avoid cloning the whole resource
    /// group write set.
    fn resource_group_metadata_ops(&self) -> Vec<(Self::Key, <Self::Txn as Transaction>::Value)>;

    /// Will be called once per transaction when the output is ready to be committed.
    /// Ensures that any writes corresponding to materialized delayed fields and group
    /// updates (recorded in output separately) are incorporated into the transaction
    /// output. Consumes the output, so the accessors above cannot be called afterwards.
    fn incorporate_materialized_txn_output(
        self,
        patched_resource_write_set: Vec<(
            <Self::Txn as Transaction>::Key,
            <Self::Txn as Transaction>::Value,
        )>,
        patched_events: Vec<<Self::Txn as Transaction>::Event>,
    ) -> Result<(Self::CommittedOutput, Trace), PanicError>;
}
