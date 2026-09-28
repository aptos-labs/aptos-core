// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! A mock MonoMove VM for the combinatorial Block-STM tests.
//!
//! It replays the same [`MockIncarnation`] read/write specs the legacy mock
//! runs, but through MonoMove's speculative read loops, read set and output
//! contract. The baseline then checks the production Block-STM glue rather than
//! a second copy of it.
//!
//! Deltas, modules, delayed fields and group size queries are out of scope for
//! MonoMove, so a spec carrying any of them is rejected.

use crate::{
    combinatorial_tests::{
        mock_executor::{mock_fee_statement, MockEvent, MockOutput},
        types::{KeyType, MockIncarnation, MockTransaction, ValueType, RESERVED_TAG},
    },
    errors::ResourceGroupSerializationError,
    executor_utilities::Materializer,
    mono_move::{MonoReads, MonoValue, ParallelReader, StorageBase},
    single_transaction_executor::{SharedViewArgs, SingleTransactionExecutor, ViewMode},
    task::{ExecutionStatus, TxnOutput},
    types::InputOutputKey,
};
use aptos_aggregator::delayed_change::DelayedChange;
use aptos_mvhashmap::types::{Incarnation, TxnIndex};
use aptos_types::{
    block_executor::output::CommittedTransactionOutput,
    error::{code_invariant_error, PanicError, PanicOr},
    fee_statement::FeeStatement,
    state_store::{
        state_value::{StateValue, StateValueMetadata},
        TStateView,
    },
    transaction::{AuxiliaryInfo, BlockExecutableTransaction as Transaction},
    write_set::{TransactionWrite, WriteOpKind},
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_types::{resolver::ResourceGroupSize, resource_group_adapter::group_size_as_sum};
use bytes::Bytes;
use mono_move_core::{
    storage::resource_provider::{ReadPin, ResourceProviderError, StorageRead},
    types::U64_TY,
};
use mono_move_global_context::GlobalContext;
use move_core_types::language_storage::ModuleId;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use once_cell::sync::OnceCell;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ptr::NonNull,
    sync::Arc,
};

pub(crate) type MonoMockKey = KeyType<[u8; 32]>;
type MonoMockTxn = MockTransaction<MonoMockKey, MockEvent>;
type MonoMockCommitted = MockOutput<MonoMockKey, MockEvent>;

/// A mock value's backing allocation: the value's length followed by its bytes.
/// The real VM points into an arena and pins it; the mock pins a plain
/// allocation, which puts the same discipline under test without a VM.
struct MockBlob(Box<[u8]>);

impl ReadPin for MockBlob {}

/// Builds the in-memory value for `bytes`, pinned to a fresh allocation.
fn mock_value(bytes: &[u8], kind: WriteOpKind) -> MonoValue {
    let mut blob = Vec::with_capacity(8 + bytes.len());
    blob.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    blob.extend_from_slice(bytes);

    let blob = Arc::new(MockBlob(blob.into_boxed_slice()));
    let ptr = NonNull::new(blob.0.as_ptr().cast_mut()).expect("A boxed slice is never null");
    MonoValue::Write {
        ptr,
        // Nothing in the mock inspects the type: values never go through the
        // real serializer.
        ty: U64_TY,
        kind,
        pin: blob,
    }
}

/// Reads back a value written by [`mock_value`].
///
/// # Safety
///
/// `ptr` must come from a [`MonoValue::Write`] whose pin is still held.
unsafe fn blob_bytes(ptr: NonNull<u8>) -> Vec<u8> {
    unsafe {
        let len = u64::from_le_bytes(
            std::slice::from_raw_parts(ptr.as_ptr(), 8)
                .try_into()
                .expect("A mock blob always starts with its length"),
        ) as usize;
        std::slice::from_raw_parts(ptr.as_ptr().add(8), len).to_vec()
    }
}

/// The bytes a read served, or [`None`] for a slot that does not exist.
fn read_bytes(read: &StorageRead) -> Option<Vec<u8>> {
    match read {
        // SAFETY: the read holds the pin for the allocation it points into.
        StorageRead::ExternalHeap { ptr, .. } => Some(unsafe { blob_bytes(*ptr) }),
        StorageRead::DoesNotExist { .. } => None,
    }
}

/// The in-memory value for a mock write.
fn write_value(value: &ValueType) -> MonoValue {
    match value.bytes() {
        Some(bytes) => mock_value(bytes, value.write_op_kind()),
        None => MonoValue::Deletion,
    }
}

/// The mock write a map value came from. Exact, because the mock never attaches
/// metadata to a value inside a group.
fn value_type(value: &MonoValue) -> Result<ValueType, PanicError> {
    Ok(match value {
        MonoValue::Write { ptr, kind, .. } => ValueType::new(
            // SAFETY: the entry holding this pointer also holds its pin.
            Some(unsafe { blob_bytes(*ptr) }.into()),
            StateValueMetadata::none(),
            kind.clone(),
        ),
        MonoValue::Deletion => {
            ValueType::new(None, StateValueMetadata::none(), WriteOpKind::Deletion)
        },
        MonoValue::RawFromStorage(bytes) => ValueType::new(
            Some(bytes.clone()),
            StateValueMetadata::none(),
            WriteOpKind::Modification,
        ),
        MonoValue::GroupMetadata => {
            return Err(code_invariant_error("Group metadata is not a mock write"))
        },
    })
}

/// The pre-block state, as MonoMove's read loops ask for it.
struct MockStorageBase<'a, S> {
    base_view: &'a S,
}

impl<S: TStateView<Key = MonoMockKey>> StorageBase<MonoMockKey, u32> for MockStorageBase<'_, S> {
    fn resource(&self, key: &MonoMockKey) -> Result<Option<MonoValue>, ResourceProviderError> {
        Ok(self
            .base_view
            .get_state_value(key)
            .map_err(|e| ResourceProviderError::InvariantViolation(format!("{e:?}")))?
            .map(|value| mock_value(value.bytes(), WriteOpKind::Modification)))
    }

    fn group_members(
        &self,
        group_key: &MonoMockKey,
    ) -> Result<Vec<(u32, Bytes)>, ResourceProviderError> {
        let Some(value) = self
            .base_view
            .get_state_value(group_key)
            .map_err(|e| ResourceProviderError::InvariantViolation(format!("{e:?}")))?
        else {
            return Ok(vec![]);
        };
        let members: BTreeMap<u32, Bytes> = bcs::from_bytes(value.bytes()).map_err(|e| {
            ResourceProviderError::InvariantViolation(format!("Stored group failed to decode: {e}"))
        })?;
        Ok(members.into_iter().collect())
    }

    fn materialize_member(
        &self,
        _key: &MonoMockKey,
        blob: &Bytes,
    ) -> Result<MonoValue, ResourceProviderError> {
        Ok(mock_value(blob, WriteOpKind::Modification))
    }
}

/// One incarnation's reads and writes, before materialization.
pub(crate) struct ExecutedTxn {
    /// Resource reads first, then group reads, which is the order the baseline
    /// splits them in.
    read_results: Vec<Option<Vec<u8>>>,
    resource_writes: HashMap<MonoMockKey, MonoValue>,
    group_writes: HashMap<MonoMockKey, (StateValueMetadata, BTreeMap<u32, MonoValue>)>,
    events: Vec<MockEvent>,
    gas: u64,
}

/// What the mock VM hands back to Block-STM.
pub(crate) enum MonoMockOutput {
    Executed(Box<ExecutedTxn>),
    /// An empty output that commits nothing: a transaction after `SkipRest`, or
    /// the `SkipRest` transaction itself.
    Retry {
        gas: u64,
    },
    /// An empty but successful transaction, i.e. a state checkpoint.
    Empty,
}

impl std::fmt::Debug for MonoMockOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MonoMockOutput::Executed(_) => f.write_str("MonoMockOutput::Executed"),
            MonoMockOutput::Retry { .. } => f.write_str("MonoMockOutput::Retry"),
            MonoMockOutput::Empty => f.write_str("MonoMockOutput::Empty"),
        }
    }
}

impl MonoMockOutput {
    fn executed(&self) -> Option<&ExecutedTxn> {
        match self {
            MonoMockOutput::Executed(txn) => Some(txn),
            MonoMockOutput::Retry { .. } | MonoMockOutput::Empty => None,
        }
    }
}

impl TxnOutput for MonoMockOutput {
    type CommittedOutput = MonoMockCommitted;
    type Key = MonoMockKey;
    type Tag = u32;
    type Txn = MonoMockTxn;
    type Value = MonoValue;

    fn skip_output() -> Self {
        MonoMockOutput::Retry { gas: 0 }
    }

    fn resource_write_set(&self) -> HashMap<Self::Key, Self::Value> {
        self.executed()
            .map(|txn| txn.resource_writes.clone())
            .unwrap_or_default()
    }

    fn delayed_field_change_set(&self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        BTreeMap::new()
    }

    fn resource_group_write_set(
        &self,
    ) -> HashMap<
        Self::Key,
        (
            Self::Value,
            ResourceGroupSize,
            BTreeMap<Self::Tag, Self::Value>,
        ),
    > {
        self.executed()
            .map(|txn| {
                txn.group_writes
                    .iter()
                    .map(|(group_key, (_, members))| {
                        (
                            *group_key,
                            (
                                MonoValue::GroupMetadata,
                                // Every `MonoValue` reports no serialized
                                // length, so this is also what the map computes.
                                ResourceGroupSize::zero_combined(),
                                members.clone(),
                            ),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn for_each_resource_key(
        &self,
        callback: &mut dyn FnMut(&Self::Key) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        for key in self
            .executed()
            .into_iter()
            .flat_map(|txn| txn.resource_writes.keys())
        {
            callback(key)?;
        }
        Ok(())
    }

    fn for_each_resource_group_key_and_tags(
        &self,
        callback: &mut dyn FnMut(&Self::Key, HashSet<&Self::Tag>) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        for (group_key, (_, members)) in self
            .executed()
            .into_iter()
            .flat_map(|txn| txn.group_writes.iter())
        {
            callback(group_key, members.keys().collect())?;
        }
        Ok(())
    }

    fn for_each_module_write(
        &self,
        _callback: &mut dyn FnMut(&ModuleId, StateValue) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        Ok(())
    }

    fn fee_statement(&self) -> FeeStatement {
        mock_fee_statement(match self {
            MonoMockOutput::Executed(txn) => txn.gas,
            MonoMockOutput::Retry { gas } => *gas,
            MonoMockOutput::Empty => 0,
        })
    }

    fn has_new_epoch_event(&self) -> bool {
        false
    }

    fn output_approx_size(&self) -> u64 {
        0
    }

    fn get_write_summary(&self) -> HashSet<InputOutputKey<Self::Key, Self::Tag>> {
        HashSet::new()
    }

    fn storage_keys_read(&self) -> impl Iterator<Item = &Self::Key> {
        std::iter::empty()
    }

    fn storage_keys_written(&self) -> impl Iterator<Item = &Self::Key> {
        std::iter::empty()
    }

    fn check_materialization(&self, _materializer: &impl Materializer<Self::Txn>) -> bool {
        true
    }
}

/// Replays one incarnation, caching reads the way a VM caches them so that a key
/// read twice observes one value and pins one version.
struct MockSession<'a> {
    reader: &'a ParallelReader<'a, MonoMockKey, u32>,
    base: &'a dyn StorageBase<MonoMockKey, u32>,
    reads: MonoReads<MonoMockKey, u32>,
    resource_cache: HashMap<MonoMockKey, Option<Vec<u8>>>,
    group_cache: HashMap<(MonoMockKey, u32), Option<Vec<u8>>>,
}

impl<'a> MockSession<'a> {
    fn new(
        reader: &'a ParallelReader<'a, MonoMockKey, u32>,
        base: &'a dyn StorageBase<MonoMockKey, u32>,
        incarnation: Incarnation,
    ) -> Self {
        Self {
            reader,
            base,
            reads: MonoReads::empty(Some(incarnation)),
            resource_cache: HashMap::new(),
            group_cache: HashMap::new(),
        }
    }

    fn read_resource(
        &mut self,
        key: &MonoMockKey,
    ) -> Result<Option<Vec<u8>>, ResourceProviderError> {
        if let Some(cached) = self.resource_cache.get(key) {
            return Ok(cached.clone());
        }
        let read = self.reader.read_resource(self.base, key)?;
        self.reads.record_data_read(*key, read.version());
        let bytes = read_bytes(&read);
        self.resource_cache.insert(*key, bytes.clone());
        Ok(bytes)
    }

    fn read_group_member(
        &mut self,
        group_key: &MonoMockKey,
        tag: u32,
    ) -> Result<Option<Vec<u8>>, ResourceProviderError> {
        if let Some(cached) = self.group_cache.get(&(*group_key, tag)) {
            return Ok(cached.clone());
        }
        // The mock has no per-member slot, so the group's own key stands in for
        // the one that would name the member's type.
        let read = self
            .reader
            .read_group_member(self.base, group_key, group_key, &tag)?;
        self.reads
            .record_group_read(*group_key, tag, read.version());
        let bytes = read_bytes(&read);
        self.group_cache.insert((*group_key, tag), bytes.clone());
        Ok(bytes)
    }

    /// Replays a spec: reads first, then the writes derived from them.
    fn run(
        &mut self,
        behavior: &MockIncarnation<MonoMockKey, MockEvent>,
    ) -> Result<ExecutedTxn, ResourceProviderError> {
        assert!(
            behavior.deltas.is_empty()
                && behavior.module_reads.is_empty()
                && behavior.module_writes.is_empty()
                && behavior.group_queries.is_empty(),
            "MonoMove supports neither deltas, nor modules, nor group size queries"
        );

        let mut read_results =
            Vec::with_capacity(behavior.resource_reads.len() + behavior.group_reads.len());
        for (key, _) in &behavior.resource_reads {
            let bytes = self.read_resource(key)?;
            read_results.push(bytes);
        }
        for (group_key, tag, _) in &behavior.group_reads {
            let bytes = self.read_group_member(group_key, *tag)?;
            read_results.push(bytes);
        }

        let mut group_writes: HashMap<_, (_, BTreeMap<_, _>)> = HashMap::new();
        for (group_key, metadata, inner_ops) in &behavior.group_writes {
            let mut members = BTreeMap::new();
            for (tag, (inner_op, _)) in inner_ops {
                let Some(op) = self.group_member_op(group_key, *tag, inner_op)? else {
                    continue;
                };
                members.insert(*tag, write_value(&op));
            }
            if !members.is_empty() {
                group_writes.insert(*group_key, (metadata.clone(), members));
            }
        }

        let resource_writes = behavior
            .resource_writes
            .iter()
            .map(|(key, value, _)| (*key, write_value(value)))
            .collect();

        Ok(ExecutedTxn {
            read_results,
            resource_writes,
            group_writes,
            events: behavior.events.clone(),
            gas: behavior.gas,
        })
    }

    /// Turns a generated group member op into one Block-STM accepts, whose kind
    /// agrees with whether the member is currently there. Mirrors the legacy
    /// mock so both executors write the same groups for the same spec.
    fn group_member_op(
        &mut self,
        group_key: &MonoMockKey,
        tag: u32,
        inner_op: &ValueType,
    ) -> Result<Option<ValueType>, ResourceProviderError> {
        let exists = self.read_group_member(group_key, tag)?.is_some();
        assert!(
            tag != RESERVED_TAG || exists,
            "RESERVED_TAG must always be present in groups in tests"
        );
        assert!(!inner_op.is_modification());

        if !exists {
            return Ok(inner_op.is_creation().then(|| inner_op.clone()));
        }
        let bytes = inner_op.bytes().cloned();
        Ok(Some(
            if inner_op.is_creation()
                && (bytes.as_ref().expect("A creation carries bytes")[0] % 4 < 3
                    || tag == RESERVED_TAG)
            {
                ValueType::new(bytes, StateValueMetadata::none(), WriteOpKind::Modification)
            } else {
                ValueType::new(None, StateValueMetadata::none(), WriteOpKind::Deletion)
            },
        ))
    }
}

/// A mock MonoMove VM, one per Block-STM worker.
pub(crate) struct MonoMockExecutor;

impl MonoMockExecutor {
    /// Renders a committed transaction the way the baseline reads it: the
    /// recorded reads, plus each written group assembled from the state the
    /// transactions before it left.
    fn committed_output(
        txn: ExecutedTxn,
        mode: ViewMode<'_, MonoReads<MonoMockKey, u32>>,
        txn_idx: TxnIndex,
    ) -> Result<MonoMockCommitted, PanicError> {
        let ViewMode::Parallel { versioned_map, .. } = mode else {
            return Err(code_invariant_error(
                "The MonoMove mock executor only runs in parallel",
            ));
        };

        let mut output = MockOutput::empty_success_output();
        output.read_results = txn.read_results;
        output.total_gas = txn.gas;
        output.events = txn.events;
        output.writes = txn
            .resource_writes
            .iter()
            .map(|(key, value)| Ok((*key, value_type(value)?, None)))
            .collect::<Result<Vec<_>, PanicError>>()?;

        let mut patched = HashMap::new();
        for (group_key, (metadata, members)) in &txn.group_writes {
            let mut assembled = versioned_map
                .group_data()
                .group_members_at(group_key, txn_idx)?
                .into_iter()
                .map(|(tag, value)| Ok((tag, member_bytes(&value)?)))
                .collect::<Result<BTreeMap<u32, Bytes>, PanicError>>()?;

            let mut inner_ops = BTreeMap::new();
            for (tag, value) in members {
                let op = value_type(value)?;
                match op.bytes() {
                    Some(bytes) => assembled.insert(*tag, bytes.clone()),
                    None => assembled.remove(tag),
                };
                inner_ops.insert(*tag, (op, None));
            }

            let (metadata_op, size) = if assembled.is_empty() {
                patched.insert(
                    *group_key,
                    ValueType::new(None, metadata.clone(), WriteOpKind::Deletion),
                );
                (
                    ValueType::new(None, metadata.clone(), WriteOpKind::Deletion),
                    ResourceGroupSize::zero_combined(),
                )
            } else {
                let bytes = bcs::to_bytes(&assembled).map_err(|e| {
                    code_invariant_error(format!("Assembled group failed to serialize: {e}"))
                })?;
                patched.insert(
                    *group_key,
                    ValueType::new(
                        Some(bytes.into()),
                        metadata.clone(),
                        WriteOpKind::Modification,
                    ),
                );
                (
                    // The mock never tests the metadata op itself.
                    ValueType::new(
                        Some(Bytes::new()),
                        metadata.clone(),
                        WriteOpKind::Modification,
                    ),
                    group_size_as_sum(assembled.iter().map(|(tag, bytes)| (tag, bytes.len())))
                        .map_err(|e| {
                            code_invariant_error(format!("Group size failed to compute: {e}"))
                        })?,
                )
            };
            output
                .group_writes
                .push((*group_key, metadata_op, size, inner_ops));
        }
        output.patched_resource_write_set = OnceCell::with_value(patched);

        Ok(output)
    }
}

/// The stored bytes of a group member the multi-version map holds.
fn member_bytes(value: &MonoValue) -> Result<Bytes, PanicError> {
    match value {
        MonoValue::RawFromStorage(bytes) => Ok(bytes.clone()),
        // SAFETY: the entry holding this pointer also holds its pin.
        MonoValue::Write { ptr, .. } => Ok(unsafe { blob_bytes(*ptr) }.into()),
        MonoValue::Deletion => Err(code_invariant_error("A deleted group member was assembled")),
        MonoValue::GroupMetadata => Err(code_invariant_error(
            "Group metadata was assembled as a member",
        )),
    }
}

impl SingleTransactionExecutor for MonoMockExecutor {
    type AuxiliaryInfo = AuxiliaryInfo;
    type Input = MonoReads<MonoMockKey, u32>;
    type Key = MonoMockKey;
    type Output = MonoMockOutput;
    type Tag = u32;
    type Txn = MonoMockTxn;
    type Value = MonoValue;

    fn init(
        _environment: &AptosEnvironment,
        _ctx: Arc<GlobalContext>,
        _state_view: &impl TStateView<Key = <Self::Txn as Transaction>::Key>,
        _worker_id: u32,
        _async_runtime_checks_enabled: bool,
    ) -> Self {
        Self
    }

    fn execute<S: TStateView<Key = <Self::Txn as Transaction>::Key> + Sync>(
        &self,
        shared: SharedViewArgs<'_, S>,
        mode: ViewMode<'_, Self::Input>,
        txn: &Self::Txn,
        _auxiliary_info: &Self::AuxiliaryInfo,
        txn_idx: TxnIndex,
    ) -> Result<(ExecutionStatus<Self::Output>, Self::Input), PanicError> {
        let ViewMode::Parallel {
            versioned_map,
            scheduler,
            incarnation,
            ..
        } = mode
        else {
            return Err(code_invariant_error(
                "The MonoMove mock executor only runs in parallel",
            ));
        };

        let behavior = match txn {
            MockTransaction::Write {
                incarnation_counter,
                incarnation_behaviors,
                ..
            } => {
                let idx = incarnation_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                &incarnation_behaviors[idx % incarnation_behaviors.len()]
            },
            MockTransaction::SkipRest(gas) => {
                return Ok((
                    ExecutionStatus::Executed {
                        output: MonoMockOutput::Retry { gas: *gas },
                        skips_rest: true,
                    },
                    MonoReads::empty(Some(incarnation)),
                ))
            },
            MockTransaction::Abort => {
                return Ok((
                    ExecutionStatus::Aborted(txn_idx.to_string()),
                    MonoReads::empty(Some(incarnation)),
                ))
            },
            MockTransaction::StateCheckpoint => {
                return Ok((
                    ExecutionStatus::Executed {
                        output: MonoMockOutput::Empty,
                        skips_rest: false,
                    },
                    MonoReads::empty(Some(incarnation)),
                ))
            },
            MockTransaction::InterruptRequested => {
                return Err(code_invariant_error(
                    "The MonoMove mock executor does not test interrupts",
                ))
            },
        };

        let reader = ParallelReader::new(versioned_map, scheduler, txn_idx, incarnation);
        let base = MockStorageBase {
            base_view: shared.base_view,
        };
        let mut session = MockSession::new(&reader, &base, incarnation);
        let executed = session.run(behavior);

        if reader.speculative_failure() {
            // A read could not be served, so whatever came out of it must not
            // commit.
            return Ok((
                ExecutionStatus::SpeculativeFailure,
                MonoReads::empty(Some(incarnation)),
            ));
        }
        let executed = executed.map_err(|e| code_invariant_error(format!("{e}")))?;

        Ok((
            ExecutionStatus::Executed {
                output: MonoMockOutput::Executed(Box::new(executed)),
                skips_rest: false,
            },
            session.reads,
        ))
    }

    fn materialize<S: TStateView<Key = <Self::Txn as Transaction>::Key> + Sync>(
        &self,
        output: Self::Output,
        _input: &Self::Input,
        _shared: SharedViewArgs<'_, S>,
        mode: ViewMode<'_, Self::Input>,
        txn_idx: TxnIndex,
    ) -> Result<MonoMockCommitted, PanicOr<ResourceGroupSerializationError>> {
        Ok(match output {
            MonoMockOutput::Executed(txn) => Self::committed_output(*txn, mode, txn_idx)?,
            MonoMockOutput::Retry { gas } => {
                let mut retry = MonoMockCommitted::retry();
                retry.total_gas = gas;
                retry
            },
            MonoMockOutput::Empty => MonoMockCommitted::empty_success_output(),
        })
    }

    fn check_materialization<S: TStateView<Key = <Self::Txn as Transaction>::Key> + Sync>(
        &self,
        _output: &Self::Output,
        _shared: SharedViewArgs<'_, S>,
        _mode: ViewMode<'_, Self::Input>,
        _txn_idx: TxnIndex,
    ) -> bool {
        true
    }

    fn pre_write_values(txn: &Self::Txn) -> Vec<(Self::Key, Self::Value)> {
        match txn {
            MockTransaction::Write { pre_writes, .. } => pre_writes
                .iter()
                .map(|(key, value)| (*key, write_value(value)))
                .collect(),
            MockTransaction::InterruptRequested
            | MockTransaction::SkipRest(_)
            | MockTransaction::Abort
            | MockTransaction::StateCheckpoint => vec![],
        }
    }

    fn materialize_storage_key(
        &self,
        key: Self::Key,
    ) -> Result<<Self::Txn as Transaction>::Key, PanicError> {
        Ok(key)
    }
}
