// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::types::InputOutputKey;
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
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Debug,
};
use triomphe::Arc as TriompheArc;

/// The outcome of executing a transaction, produced by the VM and stored by the
/// parallel executor. The output is present exactly for committable transactions;
/// a fatal abort keeps only its (stringified) error; a speculative failure keeps
/// nothing (its read set drives re-execution). A code-invariant bug is reported as
/// `Err(PanicError)` from `execute_transaction` and never stored.
#[derive(Debug)]
pub enum RecordedOutput<O> {
    /// Committable output. `skips_rest` is true for SkipRest (and is also set at
    /// commit time when the block gas limit ends the block early).
    Committed { output: O, skips_rest: bool },
    /// Fatal VM error. Surfaces as FatalVMError at commit.
    Aborted(String),
    /// Parallel speculative failure; the transaction must re-execute.
    SpeculativeFailure,
}

impl<O> RecordedOutput<O> {
    /// Reference to the output, present only for a committable transaction.
    pub(crate) fn committed_output(&self) -> Option<&O> {
        match self {
            RecordedOutput::Committed { output, .. } => Some(output),
            RecordedOutput::Aborted(_) | RecordedOutput::SpeculativeFailure => None,
        }
    }

    /// Whether the recorded outcome is a speculative failure.
    pub(crate) fn is_speculative_failure(&self) -> bool {
        matches!(self, RecordedOutput::SpeculativeFailure)
    }
}

/// Trait for single threaded transaction executor.
pub trait ExecutorTask {
    /// Type of transaction and its associated key and value.
    type Txn: Transaction;

    /// Type of auxiliary info.
    type AuxiliaryInfo: AuxiliaryInfoTrait;

    /// The output of a transaction. This should contain the side effect of this transaction.
    /// The map key/tag/value are pinned to the legacy Move VM's types (bridge; relaxed once
    /// a VM supplies its own).
    type Output: TransactionOutput<
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
    ) -> Result<RecordedOutput<Self::Output>, PanicError>;

    fn pre_write_values(
        _txn: &Self::Txn,
    ) -> Vec<(
        <Self::Txn as BlockExecutableTransaction>::Key,
        ValueWithLayout<<Self::Txn as BlockExecutableTransaction>::Value>,
    )> {
        vec![]
    }
}

/// The result of executing a single transaction.
///
/// The associated `Key`/`Tag`/`Value` are the multi-version map's types (the
/// write set applied to and validated against the map). Reads reported for hot
/// state and the block epilogue are storage keys (`Txn::Key`); events are
/// `Txn::Event`, and the raw committed write op is `Txn::Value`. For the legacy
/// Move VM the map key/value coincide with the storage key and a wrapped write
/// op (see the `Output` bounds on the executor and store).
///
/// The read accessors below must not be called after `incorporate_materialized_txn_output`
/// has produced the committed output; the block executor calls it exactly once,
/// as the terminal operation on the output.
pub trait TransactionOutput: Send + Debug {
    /// Type of transaction and its associated key, value, and event.
    type Txn: Transaction;
    /// Multi-version map key (the key resources are written under).
    type Key;
    /// Resource group tag.
    type Tag;
    /// Multi-version map value (a speculative write).
    type Value: SpeculativeValue;
    /// The materialized output produced from this (speculative) output.
    type CommittedOutput: CommittedTransactionOutput;

    /// Execution output for transactions that comes after SkipRest signal.
    fn skip_output() -> Self;

    /// Get the writes of a transaction from its output, separately for resources
    /// and modules. Aggregator V1 writes are ordinary entries of the resource write set.
    fn resource_write_set(&self) -> HashMap<Self::Key, Self::Value>;

    /// Get the delayed field changes of a transaction from its output.
    fn delayed_field_change_set(&self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>;

    fn reads_needing_delayed_field_exchange(
        &self,
    ) -> Vec<(
        <Self::Txn as Transaction>::Key,
        StateValueMetadata,
        TriompheArc<MoveTypeLayout>,
    )>;

    fn group_reads_needing_delayed_field_exchange(
        &self,
    ) -> Vec<(<Self::Txn as Transaction>::Key, StateValueMetadata)>;

    /// Get the events of a transaction from its output.
    fn get_events(&self) -> Vec<(<Self::Txn as Transaction>::Event, Option<MoveTypeLayout>)>;

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

    // The below interfaces for keys and metadata and keys and tags are provided
    // to avoid unnecessarily cloning the whole resource group write set. The raw
    // write op type is the transaction's value.
    // TODO: get rid of these interfaces when we can have zero-copy access to the output.
    fn resource_group_metadata_ops(&self) -> Vec<(Self::Key, <Self::Txn as Transaction>::Value)>;

    fn legacy_v1_resource_group_tags(&self) -> Vec<(Self::Key, HashSet<Self::Tag>)>;

    /// Return the fee statement of the transaction.
    fn fee_statement(&self) -> FeeStatement;

    fn has_new_epoch_event(&self) -> bool;

    /// Deterministic, but approximate size of the output, as
    /// before creating actual TransactionOutput, we don't know the exact size of it.
    ///
    /// Sum of all sizes of writes (keys + write_ops) and events.
    fn output_approx_size(&self) -> u64;

    fn get_write_summary(&self) -> HashSet<InputOutputKey<Self::Key, Self::Tag>>;

    /// State keys read by the VM during the execution that produced this output.
    fn storage_keys_read(&self) -> impl Iterator<Item = &<Self::Txn as Transaction>::Key>;

    /// Keys written when this output commits.
    fn storage_keys_written(&self) -> impl Iterator<Item = &<Self::Txn as Transaction>::Key>;

    /// Will be called once per transaction when the output is ready to be committed.
    /// Ensures that any writes corresponding to materialized delayed fields and group
    /// updates (recorded in output separately) are incorporated into the transaction output.
    /// !!! [CAUTION] !!!: This method must be called in quiescence, i.e. may not be
    /// concurrent with any other method that accesses the output. After it returns, the
    /// read accessors above must not be called again.
    fn incorporate_materialized_txn_output(
        &mut self,
        patched_resource_write_set: Vec<(
            <Self::Txn as Transaction>::Key,
            <Self::Txn as Transaction>::Value,
        )>,
        patched_events: Vec<<Self::Txn as Transaction>::Event>,
    ) -> Result<(Self::CommittedOutput, Trace), PanicError>;
}
