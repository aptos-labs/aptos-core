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
    captured_reads::TxnInput,
    code_cache_global::GlobalModuleCache,
    combinatorial_tests::{
        mock_executor::{mock_fee_statement, MockEvent, MockOutput},
        types::{KeyType, MockIncarnation, MockTransaction, ValueType, RESERVED_TAG},
    },
    errors::ResourceGroupSerializationError,
    executor_utilities::Materializer,
    mono_move::{
        group_chain::{committed_value, unbuilt_metadata_before, GroupMetadata, GroupView},
        version_matches, MonoValue, ParallelReader,
    },
    single_transaction_executor::{SharedViewArgs, SingleTransactionExecutor, ViewMode},
    task::{ExecutionStatus, TxnOutput},
    types::InputOutputKey,
};
use aptos_aggregator::delayed_change::DelayedChange;
use aptos_mvhashmap::{
    types::{Incarnation, MVDataError, MVDataOutput, TxnIndex},
    versioned_data::VersionedData,
    versioned_delayed_fields::TVersionedDelayedFieldView,
    versioned_group_data::VersionedGroupData,
};
use aptos_types::{
    block_executor::output::CommittedTransactionOutput,
    error::{code_invariant_error, PanicError, PanicOr},
    fee_statement::FeeStatement,
    state_store::{
        state_value::{StateValue, StateValueMetadata},
        TStateView,
    },
    transaction::{AuxiliaryInfo, BlockExecutableTransaction as Transaction},
    vm::modules::AptosModuleExtension,
    write_set::{TransactionWrite, WriteOpKind},
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_types::{resolver::ResourceGroupSize, resource_group_adapter::group_size_as_sum};
use bytes::Bytes;
use mono_move_core::storage::resource_provider::{
    ReadPin, ResourceProviderError, StorageRead, Version,
};
use mono_move_global_context::GlobalContext;
use move_binary_format::CompiledModule;
use move_core_types::language_storage::ModuleId;
use move_vm_runtime::Module;
use move_vm_types::{code::SyncModuleCache, delayed_values::delayed_field_id::DelayedFieldID};
use once_cell::sync::OnceCell;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    ptr::NonNull,
    sync::Arc,
};

/// A mock value's backing allocation: the value's length followed by its bytes.
/// The real VM points into an arena and pins it; the mock pins a plain
/// allocation, which puts the same discipline under test without a VM.
struct MockBlob(Box<[u8]>);

impl ReadPin for MockBlob {}

/// Builds the in-memory value for `bytes`, pinned to a fresh allocation.
fn mock_value(bytes: &[u8], kind: WriteOpKind) -> MonoValue<MockKey> {
    let mut blob = Vec::with_capacity(8 + bytes.len());
    blob.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    blob.extend_from_slice(bytes);

    let blob = Arc::new(MockBlob(blob.into_boxed_slice()));
    let ptr = NonNull::new(blob.0.as_ptr().cast_mut()).expect("A boxed slice is never null");
    MonoValue::Write {
        ptr,
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
fn write_value(value: &ValueType) -> MonoValue<MockKey> {
    match value.bytes() {
        Some(bytes) => mock_value(bytes, value.write_op_kind()),
        None => MonoValue::Deletion,
    }
}

/// The mock write a map value came from. Exact, because the mock never attaches
/// metadata to a value inside a group.
fn value_type(value: &MonoValue<MockKey>) -> Result<ValueType, PanicError> {
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
        MonoValue::GroupMetadata { .. } => {
            return Err(code_invariant_error("A group metadata is not a mock write"))
        },
    })
}

/// The mock's Block-STM key: a key of the generated specs, or one member of the
/// group stored at such a key. Members are versioned under their own keys.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) enum MockKey {
    Resource(KeyType<[u8; 32]>),
    GroupMember(KeyType<[u8; 32]>, u32),
}

/// A group member's key in its group's view.
fn tag_bytes(tag: u32) -> Bytes {
    bcs::to_bytes(&tag).expect("A u32 always serializes").into()
}

/// What committed transaction `writer_idx` wrote to the group member at `key`:
/// the member's key in the view, and its new bytes or [`None`] if it was
/// deleted.
fn member_change(
    data: &VersionedData<MockKey, MonoValue<MockKey>>,
    key: &MockKey,
    writer_idx: TxnIndex,
) -> Result<(Bytes, Option<Bytes>), PanicError> {
    let tag = match key {
        MockKey::GroupMember(_, tag) => tag_bytes(*tag),
        MockKey::Resource(_) => {
            return Err(code_invariant_error("A group metadata names a resource"))
        },
    };
    let bytes = match committed_value(data, key, writer_idx)? {
        // SAFETY: the value holds the pin for the allocation it points into.
        MonoValue::Write { ptr, .. } => Some(unsafe { blob_bytes(ptr) }.into()),
        MonoValue::Deletion => None,
        MonoValue::GroupMetadata { .. } => {
            return Err(code_invariant_error("A group member holds group metadata"))
        },
    };
    Ok((tag, bytes))
}

/// The view of `metadata`, which committed transaction `writer_idx` wrote,
/// built on top of `prev` unless it is built already.
fn build_committed_view(
    data: &VersionedData<MockKey, MonoValue<MockKey>>,
    metadata: &GroupMetadata<MockKey>,
    prev: &GroupView,
    writer_idx: TxnIndex,
) -> Result<Arc<GroupView>, PanicError> {
    if let Some(view) = metadata.current() {
        return Ok(view.clone());
    }
    let changes = metadata
        .changes()
        .iter()
        .map(|key| member_change(data, key, writer_idx))
        .collect::<Result<Vec<_>, PanicError>>()?;
    Ok(metadata.build_view(prev, changes))
}

/// One transaction's writes to a group.
struct MockGroupWrite {
    metadata: StateValueMetadata,
    members: BTreeMap<u32, MonoValue<MockKey>>,
    group_metadata: Arc<GroupMetadata<MockKey>>,
}

/// One incarnation's reads and writes, before materialization.
pub(crate) struct ExecutedTxn {
    /// Resource reads first, then group reads, which is the order the baseline
    /// splits them in.
    read_results: Vec<Option<Vec<u8>>>,
    resource_writes: HashMap<KeyType<[u8; 32]>, MonoValue<MockKey>>,
    group_writes: HashMap<KeyType<[u8; 32]>, MockGroupWrite>,
    /// What Block-STM versions: the resource writes, each group member under
    /// its own key, and one metadata entry per written group.
    versioned_writes: HashMap<MockKey, MonoValue<MockKey>>,
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
    type CommittedOutput = MockOutput<KeyType<[u8; 32]>, MockEvent>;
    type Key = MockKey;
    type Tag = ();
    type Txn = MockTransaction<KeyType<[u8; 32]>, MockEvent>;
    type Value = MonoValue<MockKey>;

    fn skip_output() -> Self {
        MonoMockOutput::Retry { gas: 0 }
    }

    fn resource_write_set(&self) -> HashMap<Self::Key, Self::Value> {
        self.executed()
            .map(|txn| txn.versioned_writes.clone())
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
        // Group members are versioned as plain resources.
        HashMap::new()
    }

    fn for_each_resource_key(
        &self,
        callback: &mut dyn FnMut(&Self::Key) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        for key in self
            .executed()
            .into_iter()
            .flat_map(|txn| txn.versioned_writes.keys())
        {
            callback(key)?;
        }
        Ok(())
    }

    fn for_each_resource_group_key_and_tags(
        &self,
        _callback: &mut dyn FnMut(&Self::Key, HashSet<&Self::Tag>) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
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

/// The versions one incarnation observed, recorded as the mock replays a
/// spec. The real VM keeps them in its read-write set instead.
pub(crate) struct MockReads {
    reads: HashMap<MockKey, Version>,
    incarnation: Incarnation,
    speculative_failure: bool,
}

impl MockReads {
    fn new(incarnation: Incarnation) -> Self {
        Self {
            reads: HashMap::new(),
            incarnation,
            speculative_failure: false,
        }
    }
}

impl TxnInput for MockReads {
    type Key = MockKey;
    type Tag = ();
    type Value = MonoValue<MockKey>;

    fn validate_data_reads(
        &self,
        data_map: &VersionedData<MockKey, MonoValue<MockKey>>,
        idx_to_validate: TxnIndex,
    ) -> bool {
        !self.speculative_failure
            && self.reads.iter().all(|(key, recorded)| {
                match data_map.fetch_data_no_record(key, idx_to_validate) {
                    Ok(MVDataOutput::Versioned(observed, _)) => {
                        version_matches(*recorded, observed)
                    },
                    Err(MVDataError::Uninitialized | MVDataError::Dependency(_)) => false,
                }
            })
    }

    fn validate_group_reads(
        &self,
        _group_map: &VersionedGroupData<MockKey, (), MonoValue<MockKey>>,
        _idx_to_validate: TxnIndex,
    ) -> bool {
        !self.speculative_failure
    }

    fn validate_delayed_field_reads(
        &self,
        _delayed_fields: &dyn TVersionedDelayedFieldView<DelayedFieldID>,
        _idx_to_validate: TxnIndex,
    ) -> Result<bool, PanicError> {
        Ok(true)
    }

    fn legacy_validate_module_reads(
        &self,
        _global_module_cache: &GlobalModuleCache<
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
        >,
        _per_block_module_cache: &SyncModuleCache<
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
            Option<TxnIndex>,
        >,
        _maybe_updated_module_keys: Option<&BTreeSet<ModuleId>>,
    ) -> bool {
        true
    }

    fn record_delayed_field_application_failure(&mut self) {
        self.speculative_failure = true;
    }

    fn incarnation(&self) -> Option<Incarnation> {
        Some(self.incarnation)
    }

    fn is_incorrect_use(&self) -> bool {
        false
    }

    fn get_read_summary(&self) -> HashSet<InputOutputKey<MockKey, ()>> {
        HashSet::new()
    }
}

/// Replays one incarnation, caching reads the way a VM caches them so that a key
/// read twice observes one value and pins one version.
struct MockSession<'a, S> {
    reader: &'a ParallelReader<'a, MockKey>,
    /// Pre-block state.
    base_view: &'a S,
    reads: MockReads,
    cache: HashMap<MockKey, Option<Vec<u8>>>,
}

impl<'a, S: TStateView<Key = KeyType<[u8; 32]>>> MockSession<'a, S> {
    fn new(
        reader: &'a ParallelReader<'a, MockKey>,
        base_view: &'a S,
        incarnation: Incarnation,
    ) -> Self {
        Self {
            reader,
            base_view,
            reads: MockReads::new(incarnation),
            cache: HashMap::new(),
        }
    }

    fn get_pre_block_value(
        &self,
        key: &KeyType<[u8; 32]>,
    ) -> Result<Option<StateValue>, ResourceProviderError> {
        self.base_view
            .get_state_value(key)
            .map_err(|e| ResourceProviderError::InvariantViolation(format!("{e:?}")))
    }

    fn read_resource(
        &mut self,
        key: &KeyType<[u8; 32]>,
    ) -> Result<Option<Vec<u8>>, ResourceProviderError> {
        self.read(&MockKey::Resource(*key), |session| {
            session
                .reader
                .read_or_initialize(&MockKey::Resource(*key), || {
                    Ok(match session.get_pre_block_value(key)? {
                        Some(value) => mock_value(value.bytes(), WriteOpKind::Modification),
                        None => MonoValue::Deletion,
                    })
                })
        })
    }

    fn read_group_member(
        &mut self,
        group_key: &KeyType<[u8; 32]>,
        tag: u32,
    ) -> Result<Option<Vec<u8>>, ResourceProviderError> {
        let key = MockKey::GroupMember(*group_key, tag);
        self.read(&key, |session| {
            session.reader.read_or_initialize(&key, || {
                let group = session
                    .reader
                    .get_or_initialize_pre_block_group(&MockKey::Resource(*group_key), || {
                        session.get_pre_block_group(group_key)
                    })?;
                Ok(match group.get(&tag_bytes(tag)) {
                    Some(blob) => mock_value(blob, WriteOpKind::Modification),
                    None => MonoValue::Deletion,
                })
            })
        })
    }

    fn get_pre_block_group(
        &self,
        group_key: &KeyType<[u8; 32]>,
    ) -> Result<GroupView, ResourceProviderError> {
        let Some(value) = self.get_pre_block_value(group_key)? else {
            return Ok(GroupView::default());
        };
        let members: BTreeMap<u32, Bytes> = bcs::from_bytes(value.bytes()).map_err(|e| {
            ResourceProviderError::InvariantViolation(format!("Stored group failed to decode: {e}"))
        })?;
        Ok(GroupView::new(
            members
                .into_iter()
                .map(|(tag, bytes)| (tag_bytes(tag), bytes)),
        ))
    }

    /// Serves a read from the cache, or through `read` on the first one.
    fn read(
        &mut self,
        key: &MockKey,
        read: impl FnOnce(&Self) -> Result<StorageRead, ResourceProviderError>,
    ) -> Result<Option<Vec<u8>>, ResourceProviderError> {
        if let Some(cached) = self.cache.get(key) {
            return Ok(cached.clone());
        }
        let read = read(self)?;
        self.reads.reads.insert(*key, read.version());
        let bytes = read_bytes(&read);
        self.cache.insert(*key, bytes.clone());
        Ok(bytes)
    }

    /// Replays a spec: reads first, then the writes derived from them.
    fn run(
        &mut self,
        behavior: &MockIncarnation<KeyType<[u8; 32]>, MockEvent>,
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

        let resource_writes = behavior
            .resource_writes
            .iter()
            .map(|(key, value, _)| (*key, write_value(value)))
            .collect::<HashMap<_, _>>();
        let mut versioned_writes = resource_writes
            .iter()
            .map(|(key, value)| (MockKey::Resource(*key), value.clone()))
            .collect::<HashMap<_, _>>();

        let mut group_writes = HashMap::new();
        for (group_key, metadata, inner_ops) in &behavior.group_writes {
            let mut members = BTreeMap::new();
            for (tag, (inner_op, _)) in inner_ops {
                let Some(op) = self.group_member_op(group_key, *tag, inner_op)? else {
                    continue;
                };
                let value = write_value(&op);
                versioned_writes.insert(MockKey::GroupMember(*group_key, *tag), value.clone());
                members.insert(*tag, value);
            }
            if members.is_empty() {
                continue;
            }
            let group_metadata = Arc::new(GroupMetadata::from_changes(
                members
                    .keys()
                    .map(|tag| MockKey::GroupMember(*group_key, *tag))
                    .collect(),
            ));
            versioned_writes.insert(MockKey::Resource(*group_key), MonoValue::GroupMetadata {
                group: group_metadata.clone(),
            });
            group_writes.insert(*group_key, MockGroupWrite {
                metadata: metadata.clone(),
                members,
                group_metadata,
            });
        }

        Ok(ExecutedTxn {
            read_results,
            resource_writes,
            group_writes,
            versioned_writes,
            events: behavior.events.clone(),
            gas: behavior.gas,
        })
    }

    /// Turns a generated group member op into one Block-STM accepts, whose kind
    /// agrees with whether the member is currently there. Mirrors the legacy
    /// mock so both executors write the same groups for the same spec.
    fn group_member_op(
        &mut self,
        group_key: &KeyType<[u8; 32]>,
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
        mode: ViewMode<'_, MockReads>,
        txn_idx: TxnIndex,
    ) -> Result<MockOutput<KeyType<[u8; 32]>, MockEvent>, PanicError> {
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
        for (group_key, group_write) in &txn.group_writes {
            let MockGroupWrite {
                metadata,
                members,
                group_metadata,
            } = group_write;
            let data = versioned_map.data();
            let (mut prev, unbuilt) =
                unbuilt_metadata_before(data, &MockKey::Resource(*group_key), txn_idx)?;
            for (writer_idx, unbuilt_metadata) in unbuilt {
                prev = build_committed_view(data, &unbuilt_metadata, &prev, writer_idx)?;
            }
            let view = build_committed_view(data, group_metadata, &prev, txn_idx)?;
            let assembled = view
                .iter()
                .map(|(tag, bytes)| {
                    let tag = bcs::from_bytes::<u32>(tag).map_err(|e| {
                        code_invariant_error(format!("Mock group tag failed to decode: {e}"))
                    })?;
                    Ok((tag, bytes.clone()))
                })
                .collect::<Result<BTreeMap<_, _>, PanicError>>()?;

            let inner_ops = members
                .iter()
                .map(|(tag, value)| Ok((*tag, (value_type(value)?, None))))
                .collect::<Result<BTreeMap<_, _>, PanicError>>()?;

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
                patched.insert(
                    *group_key,
                    ValueType::new(
                        Some(view.encode()),
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

impl SingleTransactionExecutor for MonoMockExecutor {
    type AuxiliaryInfo = AuxiliaryInfo;
    type Input = MockReads;
    type Key = MockKey;
    type Output = MonoMockOutput;
    type Tag = ();
    type Txn = MockTransaction<KeyType<[u8; 32]>, MockEvent>;
    type Value = MonoValue<MockKey>;

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
                    MockReads::new(incarnation),
                ))
            },
            MockTransaction::Abort => {
                return Ok((
                    ExecutionStatus::Aborted(txn_idx.to_string()),
                    MockReads::new(incarnation),
                ))
            },
            MockTransaction::StateCheckpoint => {
                return Ok((
                    ExecutionStatus::Executed {
                        output: MonoMockOutput::Empty,
                        skips_rest: false,
                    },
                    MockReads::new(incarnation),
                ))
            },
            MockTransaction::InterruptRequested => {
                return Err(code_invariant_error(
                    "The MonoMove mock executor does not test interrupts",
                ))
            },
        };

        let reader = ParallelReader::new(versioned_map, scheduler, txn_idx, incarnation);
        let mut session = MockSession::new(&reader, shared.base_view, incarnation);
        let executed = session.run(behavior);

        if reader.speculative_failure() {
            // A read could not be served, so whatever came out of it must not
            // commit.
            return Ok((
                ExecutionStatus::SpeculativeFailure,
                MockReads::new(incarnation),
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
    ) -> Result<MockOutput<KeyType<[u8; 32]>, MockEvent>, PanicOr<ResourceGroupSerializationError>>
    {
        Ok(match output {
            MonoMockOutput::Executed(txn) => Self::committed_output(*txn, mode, txn_idx)?,
            MonoMockOutput::Retry { gas } => {
                let mut retry = MockOutput::<KeyType<[u8; 32]>, MockEvent>::retry();
                retry.total_gas = gas;
                retry
            },
            MonoMockOutput::Empty => {
                MockOutput::<KeyType<[u8; 32]>, MockEvent>::empty_success_output()
            },
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
                .map(|(key, value)| (MockKey::Resource(*key), write_value(value)))
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
        match key {
            MockKey::Resource(key) => Ok(key),
            MockKey::GroupMember(..) => Err(code_invariant_error(
                "A group member has no storage key of its own",
            )),
        }
    }
}
