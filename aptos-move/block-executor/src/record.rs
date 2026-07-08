// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    captured_reads::{CapturedReads, SnapshotModuleView, UnsyncReadSet},
    code_cache_global::GlobalModuleCache,
    counters,
    errors::ResourceGroupSerializationError,
    executor_utilities::{map_finalized_group, map_id_to_values_in_group_writes, serialize_groups},
    limit_processor::BlockGasLimitProcessor,
    task::{
        BeforeMaterializationOutput, ExecutionStatus, ExecutorTask, Materializer,
        TransactionExecutor, TransactionOutput,
    },
    types::InputOutputKey,
    view::{LatestView, ViewArgs, ViewStateArgs},
};
use aptos_aggregator::delayed_change::DelayedChange;
use aptos_logger::error;
use aptos_mvhashmap::{
    types::{Incarnation, MVDelayedFieldsError, TxnIndex, ValueWithLayout},
    unsync_map::UnsyncMap,
    versioned_data::VersionedData,
    versioned_delayed_fields::TVersionedDelayedFieldView,
    versioned_group_data::VersionedGroupData,
};
use aptos_types::{
    error::{code_invariant_error, PanicError, PanicOr},
    fee_statement::FeeStatement,
    state_store::{
        state_value::{StateValue, StateValueMetadata},
        TStateView,
    },
    transaction::{BlockExecutableTransaction as Transaction, CommittedTransactionOutput},
    vm::modules::AptosModuleExtension,
    write_set::TransactionWrite,
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::{alert, prelude::*};
use aptos_vm_types::{
    change_set::randomly_check_layout_matches, module_write_set::ModuleWrite,
    resolver::ResourceGroupSize,
};
use bytes::Bytes;
use fail::fail_point;
use move_binary_format::CompiledModule;
use move_core_types::{language_storage::ModuleId, value::MoveTypeLayout};
use move_vm_runtime::{execution_tracing::Trace, Module, TypeChecker, WithRuntimeEnvironment};
use move_vm_types::{code::SyncModuleCache, delayed_values::delayed_field_id::DelayedFieldID};
use parking_lot::Mutex;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fmt::Debug,
};
use triomphe::Arc as TriompheArc;

/// The read set captured while executing a transaction with the legacy VM.
pub(crate) type TxnInput<T> =
    CapturedReads<T, ModuleId, CompiledModule, Module, AptosModuleExtension>;

/// The status a record was produced with, as exposed to the executor. Mirrors
/// [`ExecutionStatus`], borrowing the error / message from the record.
#[derive(Clone, Copy, Debug)]
pub enum RecordStatus<'a, E> {
    Success,
    SkipRest,
    /// A fatal VM error.
    Abort(&'a E),
    /// The incarnation observed an inconsistent speculative state and must be
    /// re-executed. Recordable only in parallel execution; the sequential loop
    /// converts it to a fatal error.
    SpeculativeFailure(&'a str),
    /// A delayed field code invariant violation. Only representable for
    /// sequential records: the parallel flow converts this status into an
    /// error when the record is created, and never stores it.
    DelayedFieldsCodeInvariantError(&'a str),
}

impl<E> RecordStatus<'_, E> {
    pub fn is_success_or_skip_rest(&self) -> bool {
        matches!(self, RecordStatus::Success | RecordStatus::SkipRest)
    }
}

/// The artifact of executing one transaction (one incarnation): everything the
/// VM produced, i.e. the observed reads together with the produced writes and
/// outputs. Block-STM's pipeline is generic over this: the apply loops iterate
/// the write sets (typed by the transaction's speculative value), validation
/// asks the record to validate its own reads against the shared multi-version
/// structures, commit sequencing reads the facts, and materialization turns
/// the record into the committed output.
pub trait Record: Send + Sync {
    type Txn: Transaction;
    type Error: Debug + Clone + Send + Sync + Eq + 'static;
    type CommittedOutput: CommittedTransactionOutput;

    // ---------------------------------------------------------------------
    // Status.
    // ---------------------------------------------------------------------

    fn status(&self) -> RecordStatus<'_, Self::Error>;

    fn is_speculative_failure(&self) -> bool {
        matches!(self.status(), RecordStatus::SpeculativeFailure(_))
    }

    /// Marks a delayed-field read failure observed while applying this
    /// record's delayed field changes, guaranteeing that the delayed field
    /// validation of this incarnation fails.
    fn capture_delayed_field_read_error(&mut self, e: &PanicOr<MVDelayedFieldsError>);

    // ---------------------------------------------------------------------
    // Writes, typed by the speculative value: drive the apply loops,
    // incarnation diffing, estimate marking on abort, pre-write verification
    // and module publishing. Empty when the status carries no output.
    // ---------------------------------------------------------------------

    fn resource_write_set(
        &self,
    ) -> Result<
        HashMap<<Self::Txn as Transaction>::Key, <Self::Txn as Transaction>::SpeculativeValue>,
        PanicError,
    >;

    #[allow(clippy::type_complexity)]
    fn resource_group_write_set(
        &self,
    ) -> Result<
        HashMap<
            <Self::Txn as Transaction>::Key,
            (
                <Self::Txn as Transaction>::SpeculativeValue,
                ResourceGroupSize,
                BTreeMap<
                    <Self::Txn as Transaction>::Tag,
                    <Self::Txn as Transaction>::SpeculativeValue,
                >,
            ),
        >,
        PanicError,
    >;

    /// The written group keys with the sets of tags written in each group.
    #[allow(clippy::type_complexity)]
    fn resource_group_tags(
        &self,
    ) -> Result<
        Vec<(
            <Self::Txn as Transaction>::Key,
            HashSet<<Self::Txn as Transaction>::Tag>,
        )>,
        PanicError,
    >;

    fn delayed_field_change_set(
        &self,
    ) -> Result<BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>, PanicError>;

    fn for_each_resource_key(
        &self,
        callback: &mut dyn FnMut(&<Self::Txn as Transaction>::Key) -> Result<(), PanicError>,
    ) -> Result<(), PanicError>;

    #[allow(clippy::type_complexity)]
    fn for_each_resource_group_key_and_tags(
        &self,
        callback: &mut dyn FnMut(
            &<Self::Txn as Transaction>::Key,
            HashSet<&<Self::Txn as Transaction>::Tag>,
        ) -> Result<(), PanicError>,
    ) -> Result<(), PanicError>;

    /// Iterates the module writes of the output. May only be called for
    /// success / skip-rest records (module publishing happens at commit).
    fn for_each_module_write(
        &self,
        callback: &mut dyn FnMut(
            &ModuleWrite<<Self::Txn as Transaction>::Value>,
        ) -> Result<(), PanicError>,
    ) -> Result<(), PanicError>;

    // ---------------------------------------------------------------------
    // Reads: validation of the record's reads against the shared
    // multi-version structures.
    // ---------------------------------------------------------------------

    fn validate_data_reads(
        &self,
        data_map: &VersionedData<
            <Self::Txn as Transaction>::Key,
            <Self::Txn as Transaction>::SpeculativeValue,
        >,
        idx_to_validate: TxnIndex,
    ) -> bool;

    fn validate_group_reads(
        &self,
        group_map: &VersionedGroupData<
            <Self::Txn as Transaction>::Key,
            <Self::Txn as Transaction>::Tag,
            <Self::Txn as Transaction>::SpeculativeValue,
        >,
        idx_to_validate: TxnIndex,
    ) -> bool;

    fn validate_module_reads(
        &self,
        global_module_cache: &GlobalModuleCache<
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
        >,
        per_block_module_cache: &SyncModuleCache<
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
            Option<TxnIndex>,
        >,
        maybe_updated_module_keys: Option<&BTreeSet<ModuleId>>,
    ) -> bool;

    fn validate_delayed_field_reads(
        &self,
        delayed_fields: &dyn TVersionedDelayedFieldView<DelayedFieldID>,
        idx_to_validate: TxnIndex,
    ) -> Result<bool, PanicError>;

    /// The incarnation this record's reads were captured at, when executed by
    /// BlockSTMv2 (used to guard module validation).
    fn blockstm_v2_incarnation(&self) -> Option<Incarnation>;

    // ---------------------------------------------------------------------
    // Commit facts: drive block limit accounting, the epilogue decision and
    // hot state accumulation.
    // ---------------------------------------------------------------------

    #[allow(clippy::type_complexity)]
    fn read_summary(
        &self,
    ) -> HashSet<InputOutputKey<<Self::Txn as Transaction>::Key, <Self::Txn as Transaction>::Tag>>;

    #[allow(clippy::type_complexity)]
    fn write_summary(
        &self,
    ) -> Result<
        HashSet<InputOutputKey<<Self::Txn as Transaction>::Key, <Self::Txn as Transaction>::Tag>>,
        PanicError,
    >;

    fn fee_statement(&self) -> Result<FeeStatement, PanicError>;

    fn has_new_epoch_event(&self) -> Result<bool, PanicError>;

    /// Deterministic, but approximate size of the output, as before creating
    /// the committed output we don't know its exact size.
    fn output_approx_size(&self) -> Result<u64, PanicError>;

    /// Feeds the hot state accumulator with the keys this record read and wrote.
    fn accumulate_hot_state(
        &self,
        block_limit_processor: &mut BlockGasLimitProcessor<Self::Txn>,
    ) -> Result<(), PanicError>;

    // ---------------------------------------------------------------------
    // Sequential-only hooks.
    // ---------------------------------------------------------------------

    /// Checks whether any resource group this record read (needing exchange)
    /// or wrote would fail bcs serialization or produce a size mismatch. Used
    /// by the sequential bcs fallback re-run to discard such transactions.
    fn sequential_group_serialization_error(
        &self,
        unsync_map: &UnsyncMap<
            <Self::Txn as Transaction>::Key,
            <Self::Txn as Transaction>::Tag,
            <Self::Txn as Transaction>::SpeculativeValue,
            DelayedFieldID,
        >,
    ) -> Result<bool, PanicError>;

    /// Whether the reads were used incorrectly during (sequential) execution.
    /// The parallel flow rejects such records at construction instead.
    fn sequential_incorrect_use(&self) -> bool;

    // ---------------------------------------------------------------------
    // Materialization.
    // ---------------------------------------------------------------------

    /// Materializes this record into its committed output. A group
    /// serialization failure is surfaced so the sequential caller can trigger
    /// the bcs fallback re-run of the block; parallel materialization converts
    /// it to an invariant error internally.
    ///
    /// May only be called once per record, after the transaction commits, and
    /// may not be concurrent with any other access to the record's output.
    fn materialize<S: TStateView<Key = <Self::Txn as Transaction>::Key> + Sync>(
        &self,
        args: &ViewArgs<'_, Self::Txn, S>,
        txn_idx: TxnIndex,
        environment: &AptosEnvironment,
    ) -> Result<Self::CommittedOutput, PanicOr<ResourceGroupSerializationError>>;
}

/// The reads captured by one execution, in the representation of the mode the
/// transaction was executed in (mirroring `ViewState`).
enum LegacyReads<T: Transaction> {
    Parallel(TxnInput<T>),
    Sequential(UnsyncReadSet<T, ModuleId>),
}

impl<T: Transaction> LegacyReads<T> {
    fn parallel(&self) -> &TxnInput<T> {
        match self {
            LegacyReads::Parallel(reads) => reads,
            LegacyReads::Sequential(_) => {
                unreachable!("Parallel reads requested from a sequential record")
            },
        }
    }
}

/// The status a legacy record was produced with (the owned counterpart of
/// [`RecordStatus`]).
enum LegacyStatus<E> {
    Success,
    SkipRest,
    Abort(E),
    SpeculativeFailure(String),
    /// Only constructed for sequential records; see [`RecordStatus`].
    DelayedFieldsCodeInvariantError(String),
}

/// The record of the legacy VM: the captured reads together with the produced
/// [`TransactionOutput`]. The output lives behind a mutex only because
/// materialization consumes it (in place) while the record is shared behind an
/// `Arc`; all accessors take the lock briefly. Materialization may not be
/// concurrent with any other output access.
pub struct LegacyRecord<T, O, E>
where
    T: Transaction,
    O: TransactionOutput<Txn = T>,
{
    reads: LegacyReads<T>,
    status: LegacyStatus<E>,
    output: Mutex<Option<O>>,
}

impl<T, O, E> LegacyRecord<T, O, E>
where
    T: Transaction<SpeculativeValue = ValueWithLayout<<T as Transaction>::Value>>,
    O: TransactionOutput<Txn = T>,
    E: Debug + Clone + Send + Sync + Eq + 'static,
{
    /// Bundles the reads captured by a parallel execution with the execution
    /// result, converting code invariant statuses into errors and capturing
    /// the delayed-field read error for speculative aborts (so that failed
    /// validation is guaranteed).
    pub(crate) fn from_execution_status(
        mut reads: TxnInput<T>,
        result: ExecutionStatus<O, E>,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
    ) -> Result<Self, PanicError> {
        if reads.is_incorrect_use() {
            return Err(code_invariant_error(format!(
                "Incorrect use detected in CapturedReads after executing txn = {} incarnation = {}",
                txn_idx, incarnation
            )));
        }

        let (status, output) = match result {
            ExecutionStatus::Success(output) => (LegacyStatus::Success, Some(output)),
            ExecutionStatus::SkipRest(output) => (LegacyStatus::SkipRest, Some(output)),
            ExecutionStatus::Abort(err) => (LegacyStatus::Abort(err), None),
            ExecutionStatus::SpeculativeExecutionAbortError(msg) => {
                // TODO(BlockSTMv2): cleaner to rename or distinguish V2 early abort
                // from DeltaApplicationFailure.
                reads.capture_delayed_field_read_error(&PanicOr::Or(
                    MVDelayedFieldsError::DeltaApplicationFailure,
                ));
                (LegacyStatus::SpeculativeFailure(msg), None)
            },
            ExecutionStatus::DelayedFieldsCodeInvariantError(msg) => {
                return Err(code_invariant_error(format!(
                    "[Execution] At txn {}, failed with DelayedFieldsCodeInvariantError: {:?}",
                    txn_idx, msg
                )));
            },
        };

        Ok(Self {
            reads: LegacyReads::Parallel(reads),
            status,
            output: Mutex::new(output),
        })
    }

    /// Bundles the reads and result of a sequential execution. All statuses
    /// are representable: the sequential loop converts failure statuses into
    /// fatal block errors itself. The incorrect-use check is deferred until
    /// after materialization (mirroring the sequential flow).
    pub(crate) fn from_execution_status_sequential(
        reads: UnsyncReadSet<T, ModuleId>,
        result: ExecutionStatus<O, E>,
    ) -> Self {
        let (status, output) = match result {
            ExecutionStatus::Success(output) => (LegacyStatus::Success, Some(output)),
            ExecutionStatus::SkipRest(output) => (LegacyStatus::SkipRest, Some(output)),
            ExecutionStatus::Abort(err) => (LegacyStatus::Abort(err), None),
            ExecutionStatus::SpeculativeExecutionAbortError(msg) => {
                (LegacyStatus::SpeculativeFailure(msg), None)
            },
            ExecutionStatus::DelayedFieldsCodeInvariantError(msg) => {
                (LegacyStatus::DelayedFieldsCodeInvariantError(msg), None)
            },
        };

        Self {
            reads: LegacyReads::Sequential(reads),
            status,
            output: Mutex::new(output),
        }
    }

    // Runs the given function on the output guard if the output is present
    // (success / skip-rest and not yet consumed by materialization), otherwise
    // returns the fallback.
    fn with_output_or<R>(
        &self,
        f: impl FnOnce(&O::BeforeMaterializationGuard<'_>) -> R,
        fallback: impl FnOnce() -> R,
    ) -> Result<R, PanicError> {
        let output = self.output.lock();
        match output.as_ref() {
            Some(output) => Ok(f(&output.before_materialization()?)),
            None => Ok(fallback()),
        }
    }

    // ---------------------------------------------------------------------
    // Legacy materialization internals.
    // ---------------------------------------------------------------------

    fn resource_group_metadata_ops(
        &self,
    ) -> Result<Vec<(T::Key, T::SpeculativeValue)>, PanicError> {
        self.with_output_or(|guard| guard.resource_group_metadata_ops(), Vec::new)
    }

    fn reads_needing_delayed_field_exchange(
        &self,
    ) -> Result<Vec<(T::Key, StateValueMetadata, TriompheArc<MoveTypeLayout>)>, PanicError> {
        self.with_output_or(
            |guard| guard.reads_needing_delayed_field_exchange(),
            Vec::new,
        )
    }

    fn group_reads_needing_delayed_field_exchange(
        &self,
    ) -> Result<Vec<(T::Key, StateValueMetadata)>, PanicError> {
        self.with_output_or(
            |guard| guard.group_reads_needing_delayed_field_exchange(),
            Vec::new,
        )
    }

    /// Fetches the value and layout a read needing delayed-field exchange
    /// observed during execution. The value must have been captured in the
    /// exchanged format.
    fn fetch_exchanged_data(
        &self,
        key: &T::Key,
    ) -> Result<(TriompheArc<T::Value>, TriompheArc<MoveTypeLayout>), PanicError> {
        use crate::captured_reads::{DataRead, ReadKind};
        let data_read = self
            .reads
            .parallel()
            .get_by_kind(key, None, ReadKind::Value);
        if let Some(DataRead::Versioned(_, value, Some(layout))) = data_read {
            Ok((value, layout))
        } else {
            Err(code_invariant_error(format!(
                "Read value needing exchange {:?} not in Exchanged format",
                data_read
            )))
        }
    }

    /// Finalizes the stored output into the committed output, materializing
    /// the output in place (patching the writes and events it owns via the
    /// materializer), consuming the stored (speculative) output.
    /// !!! [CAUTION] !!!: This method must be called in quiescence, i.e. may
    /// not be concurrent with any other method that accesses the output.
    fn incorporate_materialized_txn_output(
        &self,
        materializer: &impl Materializer<Key = T::Key>,
    ) -> Result<(O::CommittedOutput, Trace), PanicError> {
        let mut output = self.output.lock();
        let output = output.as_mut().ok_or_else(|| {
            // Only committed (success / skip-rest) outputs are materialized, so a
            // non-committed status here is a code invariant violation.
            code_invariant_error("Only committed (success / skip-rest) outputs can be materialized")
        })?;
        output.incorporate_materialized_txn_output(materializer)
    }
}

/// The materializer handed to the legacy output at incorporation: resolves
/// value-level patches via the view, and serves the group and exchanged-read
/// bytes precomputed by the record (both derived from the multi-version state
/// the output cannot access itself).
struct LegacyMaterializer<'a, 'b, T: Transaction, S: TStateView<Key = T::Key>> {
    view: &'b LatestView<'a, T, S>,
    serialized_groups: HashMap<T::Key, Bytes>,
    exchanged_reads: HashMap<T::Key, Bytes>,
}

impl<T: Transaction, S: TStateView<Key = T::Key>> Materializer
    for LegacyMaterializer<'_, '_, T, S>
{
    type Key = T::Key;

    fn replace_identifiers_with_values(
        &self,
        bytes: &[u8],
        layout: &MoveTypeLayout,
    ) -> Result<Bytes, PanicError> {
        let (bytes, _) = self
            .view
            .replace_identifiers_with_values(bytes, layout)
            .map_err(|_| {
                code_invariant_error(format!(
                    "Failed to replace identifiers with values in a value with layout {:?}",
                    layout
                ))
            })?;
        Ok(bytes)
    }

    fn serialized_group_bytes(&self, key: &T::Key) -> Result<Bytes, PanicError> {
        self.serialized_groups.get(key).cloned().ok_or_else(|| {
            code_invariant_error(format!("No serialized group bytes for key {:?}", key))
        })
    }

    fn exchanged_read_bytes(&self, key: &T::Key) -> Result<Bytes, PanicError> {
        self.exchanged_reads.get(key).cloned().ok_or_else(|| {
            code_invariant_error(format!("No exchanged read bytes for key {:?}", key))
        })
    }
}

impl<T, O, E> Record for LegacyRecord<T, O, E>
where
    // The value-shape bound reflects that the legacy record's reads, writes and
    // views are built on `ValueWithLayout`.
    T: Transaction<SpeculativeValue = ValueWithLayout<<T as Transaction>::Value>>,
    O: TransactionOutput<Txn = T> + 'static,
    E: Debug + Clone + Send + Sync + Eq + 'static,
{
    type CommittedOutput = O::CommittedOutput;
    type Error = E;
    type Txn = T;

    fn status(&self) -> RecordStatus<'_, E> {
        match &self.status {
            LegacyStatus::Success => RecordStatus::Success,
            LegacyStatus::SkipRest => RecordStatus::SkipRest,
            LegacyStatus::Abort(err) => RecordStatus::Abort(err),
            LegacyStatus::SpeculativeFailure(msg) => RecordStatus::SpeculativeFailure(msg),
            LegacyStatus::DelayedFieldsCodeInvariantError(msg) => {
                RecordStatus::DelayedFieldsCodeInvariantError(msg)
            },
        }
    }

    fn capture_delayed_field_read_error(&mut self, e: &PanicOr<MVDelayedFieldsError>) {
        match &mut self.reads {
            LegacyReads::Parallel(reads) => reads.capture_delayed_field_read_error(e),
            LegacyReads::Sequential(_) => {
                unreachable!("Delayed field read errors are only captured in parallel execution")
            },
        }
    }

    fn resource_write_set(&self) -> Result<HashMap<T::Key, T::SpeculativeValue>, PanicError> {
        self.with_output_or(|guard| guard.resource_write_set(), HashMap::new)
    }

    fn resource_group_write_set(
        &self,
    ) -> Result<
        HashMap<
            T::Key,
            (
                T::SpeculativeValue,
                ResourceGroupSize,
                BTreeMap<T::Tag, T::SpeculativeValue>,
            ),
        >,
        PanicError,
    > {
        self.with_output_or(|guard| guard.resource_group_write_set(), HashMap::new)
    }

    fn resource_group_tags(&self) -> Result<Vec<(T::Key, HashSet<T::Tag>)>, PanicError> {
        self.with_output_or(|guard| guard.legacy_v1_resource_group_tags(), Vec::new)
    }

    fn delayed_field_change_set(
        &self,
    ) -> Result<BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>, PanicError> {
        self.with_output_or(|guard| guard.delayed_field_change_set(), BTreeMap::new)
    }

    fn for_each_resource_key(
        &self,
        callback: &mut dyn FnMut(&T::Key) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        self.with_output_or(|guard| guard.for_each_resource_key(callback), || Ok(()))?
    }

    fn for_each_resource_group_key_and_tags(
        &self,
        callback: &mut dyn FnMut(&T::Key, HashSet<&T::Tag>) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        self.with_output_or(
            |guard| guard.for_each_resource_group_key_and_tags(callback),
            || Ok(()),
        )?
    }

    fn for_each_module_write(
        &self,
        callback: &mut dyn FnMut(&ModuleWrite<T::Value>) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        let output = self.output.lock();
        let output = output.as_ref().ok_or_else(|| {
            code_invariant_error("Module writes may only be iterated on committable outputs")
        })?;
        for write in output.before_materialization()?.module_write_set().values() {
            callback(write)?;
        }
        Ok(())
    }

    fn validate_data_reads(
        &self,
        data_map: &VersionedData<T::Key, T::SpeculativeValue>,
        idx_to_validate: TxnIndex,
    ) -> bool {
        self.reads
            .parallel()
            .validate_data_reads(data_map, idx_to_validate)
    }

    fn validate_group_reads(
        &self,
        group_map: &VersionedGroupData<T::Key, T::Tag, T::SpeculativeValue>,
        idx_to_validate: TxnIndex,
    ) -> bool {
        self.reads
            .parallel()
            .validate_group_reads(group_map, idx_to_validate)
    }

    fn validate_module_reads(
        &self,
        global_module_cache: &GlobalModuleCache<
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
        >,
        per_block_module_cache: &SyncModuleCache<
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
            Option<TxnIndex>,
        >,
        maybe_updated_module_keys: Option<&BTreeSet<ModuleId>>,
    ) -> bool {
        self.reads.parallel().validate_module_reads(
            global_module_cache,
            per_block_module_cache,
            maybe_updated_module_keys,
        )
    }

    fn validate_delayed_field_reads(
        &self,
        delayed_fields: &dyn TVersionedDelayedFieldView<DelayedFieldID>,
        idx_to_validate: TxnIndex,
    ) -> Result<bool, PanicError> {
        self.reads
            .parallel()
            .validate_delayed_field_reads(delayed_fields, idx_to_validate)
    }

    fn blockstm_v2_incarnation(&self) -> Option<Incarnation> {
        self.reads.parallel().blockstm_v2_incarnation()
    }

    fn read_summary(&self) -> HashSet<InputOutputKey<T::Key, T::Tag>> {
        match &self.reads {
            LegacyReads::Parallel(reads) => reads.get_read_summary(),
            LegacyReads::Sequential(reads) => reads.get_read_summary(),
        }
    }

    fn write_summary(&self) -> Result<HashSet<InputOutputKey<T::Key, T::Tag>>, PanicError> {
        self.with_output_or(|guard| guard.get_write_summary(), HashSet::new)
    }

    fn fee_statement(&self) -> Result<FeeStatement, PanicError> {
        self.with_output_or(|guard| guard.fee_statement(), FeeStatement::zero)
    }

    fn has_new_epoch_event(&self) -> Result<bool, PanicError> {
        self.with_output_or(|guard| guard.has_new_epoch_event(), || false)
    }

    fn output_approx_size(&self) -> Result<u64, PanicError> {
        self.with_output_or(|guard| guard.output_approx_size(), || 0)
    }

    fn accumulate_hot_state(
        &self,
        block_limit_processor: &mut BlockGasLimitProcessor<T>,
    ) -> Result<(), PanicError> {
        self.with_output_or(
            |guard| {
                block_limit_processor.accumulate_hot_state_rw(
                    guard.storage_keys_written(),
                    guard.storage_keys_read(),
                )
            },
            || (),
        )
    }

    fn sequential_group_serialization_error(
        &self,
        unsync_map: &UnsyncMap<T::Key, T::Tag, T::SpeculativeValue, DelayedFieldID>,
    ) -> Result<bool, PanicError> {
        let finalize = |group_key| -> (BTreeMap<_, _>, ResourceGroupSize) {
            let (group, size) = unsync_map.finalize_group(&group_key);

            (
                group
                    .map(|(resource_tag, value_with_layout)| {
                        let value = match value_with_layout {
                            ValueWithLayout::RawFromStorage(value)
                            | ValueWithLayout::Exchanged(value, _) => value,
                        };
                        (
                            resource_tag,
                            value
                                .extract_raw_bytes()
                                .expect("Deletions should already be applied"),
                        )
                    })
                    .collect(),
                size,
            )
        };

        // The IDs are not exchanged but it doesn't change the types (Bytes) or size.
        let serialization_error = self
            .group_reads_needing_delayed_field_exchange()?
            .iter()
            .any(|(group_key, _)| {
                fail_point!("fail-point-resource-group-serialization", |_| { true });

                let (finalized_group, group_size) = finalize(group_key.clone());
                match bcs::to_bytes(&finalized_group) {
                    Ok(group) => {
                        (!finalized_group.is_empty() || group_size.get() != 0)
                            && group.len() as u64 != group_size.get()
                    },
                    Err(_) => true,
                }
            })
            || self.resource_group_write_set()?.into_iter().any(
                |(group_key, (_, output_group_size, group_ops))| {
                    fail_point!("fail-point-resource-group-serialization", |_| { true });

                    let (mut finalized_group, group_size) = finalize(group_key);
                    if output_group_size.get() != group_size.get() {
                        return false;
                    }
                    for (value_tag, value_with_layout) in group_ops {
                        let group_op = value_with_layout.extract_value();
                        if group_op.is_deletion() {
                            finalized_group.remove(&value_tag);
                        } else {
                            finalized_group.insert(
                                value_tag,
                                group_op.extract_raw_bytes().expect("Not a deletion"),
                            );
                        }
                    }
                    match bcs::to_bytes(&finalized_group) {
                        Ok(group) => {
                            (!finalized_group.is_empty() || group_size.get() != 0)
                                && group.len() as u64 != group_size.get()
                        },
                        Err(_) => true,
                    }
                },
            );

        Ok(serialization_error)
    }

    fn sequential_incorrect_use(&self) -> bool {
        match &self.reads {
            LegacyReads::Parallel(_) => {
                unreachable!("Sequential incorrect use queried on a parallel record")
            },
            LegacyReads::Sequential(reads) => reads.incorrect_use,
        }
    }

    fn materialize<S: TStateView<Key = T::Key> + Sync>(
        &self,
        args: &ViewArgs<'_, T, S>,
        txn_idx: TxnIndex,
        environment: &AptosEnvironment,
    ) -> Result<O::CommittedOutput, PanicOr<ResourceGroupSerializationError>> {
        let latest_view = args.build_view(txn_idx);

        // Groups to finalize: the groups this record wrote, and the groups it
        // only read that need delayed field exchange (their metadata op is
        // reconstructed from the captured read).
        let groups_to_finalize = self
            .resource_group_metadata_ops()?
            .into_iter()
            .map(|val| (val, false))
            .chain(
                self.group_reads_needing_delayed_field_exchange()?
                    .into_iter()
                    .map(|(key, metadata)| {
                        let value = TransactionWrite::from_state_value(Some(
                            StateValue::new_with_metadata(Bytes::new(), metadata),
                        ));
                        (
                            (
                                key,
                                ValueWithLayout::Exchanged(TriompheArc::new(value), None),
                            ),
                            true,
                        )
                    }),
            );

        let serialized_groups = match (&args.state, &self.reads) {
            (
                ViewStateArgs::Parallel {
                    versioned_cache, ..
                },
                LegacyReads::Parallel(_),
            ) => {
                let finalized_groups = groups_to_finalize
                    .map(|((group_key, metadata_op), is_read_needing_exchange)| {
                        let (finalized_group, group_size) = versioned_cache
                            .group_data()
                            .finalize_group(&group_key, txn_idx)?;

                        map_finalized_group::<T>(
                            group_key,
                            finalized_group,
                            group_size,
                            metadata_op,
                            is_read_needing_exchange,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let materialized_finalized_groups =
                    map_id_to_values_in_group_writes(finalized_groups, &latest_view)?;
                // A group serialization failure in parallel mode is an invariant
                // violation (the sizes were validated during execution).
                serialize_groups::<T>(materialized_finalized_groups).map_err(|e| {
                    code_invariant_error(format!("Panic error in serializing groups {e:?}"))
                })?
            },
            (ViewStateArgs::Sequential { unsync_map, .. }, LegacyReads::Sequential(_)) => {
                let finalized_groups = groups_to_finalize
                    .map(|((group_key, metadata_op), is_read_needing_exchange)| {
                        let (group_ops_iter, group_size) = unsync_map.finalize_group(&group_key);
                        map_finalized_group::<T>(
                            group_key,
                            group_ops_iter.collect(),
                            group_size,
                            metadata_op,
                            is_read_needing_exchange,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let materialized_finalized_groups =
                    map_id_to_values_in_group_writes(finalized_groups, &latest_view)?;
                // The sequential caller turns this into the bcs fallback re-run.
                serialize_groups::<T>(materialized_finalized_groups).map_err(PanicOr::Or)?
            },
            (ViewStateArgs::Parallel { .. }, LegacyReads::Sequential(_))
            | (ViewStateArgs::Sequential { .. }, LegacyReads::Parallel(_)) => {
                return Err(code_invariant_error(
                    "Materialization mode does not match the record's execution mode",
                )
                .into());
            },
        };

        // The values of reads needing delayed field exchange: observed values
        // (fetched from the captured reads in parallel mode, and from the
        // unsync map in sequential mode) with the delayed-field identifiers
        // replaced by the committed values. Writes carrying a layout hold
        // their own bytes and are patched by the output itself during
        // incorporation.
        let exchanged_reads = self
            .reads_needing_delayed_field_exchange()?
            .into_iter()
            .map(|(key, _metadata, layout)| -> Result<_, PanicError> {
                let (value, existing_layout) = match &args.state {
                    ViewStateArgs::Parallel { .. } => self.fetch_exchanged_data(&key)?,
                    ViewStateArgs::Sequential { unsync_map, .. } => {
                        match unsync_map.fetch_data(&key) {
                            Some(ValueWithLayout::Exchanged(value, Some(layout))) => {
                                (value, layout)
                            },
                            data => {
                                return Err(code_invariant_error(format!(
                                    "Read value needing exchange {:?} does not exist or not in \
                                     Exchanged format",
                                    data
                                )));
                            },
                        }
                    },
                };
                randomly_check_layout_matches(Some(&existing_layout), Some(layout.as_ref()))?;
                let bytes = value.bytes().cloned().unwrap_or_else(Bytes::new);
                let (patched_bytes, _) = latest_view
                    .replace_identifiers_with_values(&bytes, &layout)
                    .map_err(|_| {
                        code_invariant_error(format!(
                            "Failed to replace identifiers with values in a resource {:?}",
                            layout
                        ))
                    })?;
                Ok((key, patched_bytes))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        // This call finalizes the output, materializing it in place (patching
        // the writes and events it owns via the materializer), and may not be
        // concurrent with any other accesses to the output (e.g. querying the
        // write-set, events, etc), as these read accesses are not synchronized
        // and assumed to have terminated.
        let materializer = LegacyMaterializer {
            view: &latest_view,
            serialized_groups,
            exchanged_reads,
        };
        let (committed_output, trace) = self.incorporate_materialized_txn_output(&materializer)?;

        match &self.reads {
            LegacyReads::Parallel(reads) => {
                if environment.async_runtime_checks_enabled() && !trace.is_empty() {
                    // Note that the trace may be empty (if block was small and executor decides
                    // not to collect the trace and replay, or if the VM decides it is not
                    // profitable to do this check for this particular transaction), so we check
                    // it in advance.

                    // Create a module view that resolves modules from the read-set snapshot
                    // (captured at execution time), ensuring that the replay sees the same module
                    // versions as execution. This prevents race conditions where modules are
                    // published between execution and post-commit processing.
                    let snapshot_view =
                        SnapshotModuleView::new(reads, environment.runtime_environment());

                    let result = {
                        counters::update_txn_trace_counters(&trace);
                        let _timer = counters::TRACE_REPLAY_SECONDS.start_timer();
                        // Use snapshot_view instead of latest_view to avoid module cache race.
                        TypeChecker::new(&snapshot_view).replay(&trace)
                    };

                    // In case of runtime type check errors, fallback to sequential execution.
                    // There errors are supposed to be unlikely so this fallback is fine, and is
                    // mostly needed to make sure transaction epilogue runs after failure, etc.
                    if let Err(err) = result {
                        alert!(
                            "Runtime type check failed during replay of transaction {}: {:?}",
                            txn_idx,
                            err
                        );
                        return Err(PanicError::CodeInvariantError(format!(
                            "Sequential fallback on type check failure for transaction {}: {:?}",
                            txn_idx, err
                        ))
                        .into());
                    }
                }
            },
            LegacyReads::Sequential(_) => {
                // Sequential execution never collects any traces.
                if !trace.is_empty() {
                    return Err(code_invariant_error(
                        "Sequential execution should not record any traces",
                    )
                    .into());
                }
            },
        }

        Ok(committed_output)
    }
}

/// Every legacy VM (an [`ExecutorTask`]) is a [`TransactionExecutor`]: the
/// glue builds the V1 view (`LatestView`) from the VM-neutral view
/// ingredients, runs the transaction against it, and bundles the captured
/// reads with the execution result into a [`LegacyRecord`].
impl<E: ExecutorTask> TransactionExecutor for E
where
    E::Txn: Transaction<SpeculativeValue = ValueWithLayout<<E::Txn as Transaction>::Value>>,
{
    type AuxiliaryInfo = E::AuxiliaryInfo;
    type Error = E::Error;
    type Record = LegacyRecord<E::Txn, E::Output, E::Error>;
    type Txn = E::Txn;

    fn init(
        environment: &AptosEnvironment,
        state_view: &impl TStateView<Key = <E::Txn as Transaction>::Key>,
        async_runtime_checks_enabled: bool,
    ) -> Self {
        <E as ExecutorTask>::init(environment, state_view, async_runtime_checks_enabled)
    }

    fn execute_transaction_v2<S: TStateView<Key = <E::Txn as Transaction>::Key> + Sync>(
        &self,
        args: &ViewArgs<'_, E::Txn, S>,
        txn: &E::Txn,
        auxiliary_info: &E::AuxiliaryInfo,
        txn_idx: TxnIndex,
    ) -> Result<Self::Record, PanicError> {
        let view = args.build_view(txn_idx);
        let result =
            <E as ExecutorTask>::execute_transaction(self, &view, txn, auxiliary_info, txn_idx);

        match &args.state {
            ViewStateArgs::Parallel { incarnation, .. } => {
                let reads = view.take_parallel_reads();
                LegacyRecord::from_execution_status(reads, result, txn_idx, *incarnation)
            },
            ViewStateArgs::Sequential { .. } => {
                let reads = view.take_sequential_reads();
                Ok(LegacyRecord::from_execution_status_sequential(
                    reads, result,
                ))
            },
        }
    }
}
