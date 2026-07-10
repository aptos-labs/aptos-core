// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The record of one mono execution: the reads it observed (with versions)
//! and the writes it produced, both extracted from the interpreter's
//! read-write set. Validation compares versions only; materialization is a
//! stub (the P0 goal is executing blocks end-to-end, not producing outputs).

use crate::value::{to_mono_version, MonoValue};
use aptos_aggregator::delayed_change::DelayedChange;
use aptos_block_executor::{
    code_cache_global::GlobalModuleCache,
    errors::ResourceGroupSerializationError,
    limit_processor::BlockGasLimitProcessor,
    record::{Record, RecordStatus},
    types::InputOutputKey,
    view::ViewArgs,
};
use aptos_mvhashmap::{
    types::{Incarnation, MVDataError, MVDataOutput, MVDelayedFieldsError, TxnIndex},
    unsync_map::UnsyncMap,
    versioned_data::VersionedData,
    versioned_delayed_fields::TVersionedDelayedFieldView,
    versioned_group_data::VersionedGroupData,
};
use aptos_types::{
    contract_event::ContractEvent,
    error::{PanicError, PanicOr},
    fee_statement::FeeStatement,
    state_store::{state_value::StateValue, TStateView},
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction,
        BlockExecutableTransaction as Transaction, ExecutionStatus, TransactionAuxiliaryData,
        TransactionOutput, TransactionStatus,
    },
    vm::modules::AptosModuleExtension,
    write_set::WriteSet,
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_types::resolver::ResourceGroupSize;
use mono_move_core::storage::resource_provider::InMemoryStorageKey;
use mono_move_runtime::Version;
use move_binary_format::CompiledModule;
use move_core_types::{
    language_storage::{ModuleId, StructTag},
    vm_status::VMStatus,
};
use move_vm_runtime::Module;
use move_vm_types::{code::SyncModuleCache, delayed_values::delayed_field_id::DelayedFieldID};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// The status a mono record was produced with. The mono path never emits
/// skip-rest or fatal aborts: failed executions become success records with
/// empty writes (outputs are stubs in P0).
enum MonoRecordStatus {
    Success,
    /// The execution observed an inconsistent speculative state (e.g. read a
    /// value marked as an estimate) and must be re-executed.
    SpeculativeFailure(String),
}

/// The artifact of one mono execution.
pub struct MonoRecord {
    status: MonoRecordStatus,
    /// Every storage read the execution made, with the version it observed.
    reads: HashMap<InMemoryStorageKey, Version>,
    /// The produced write set. Empty for failed and speculative-failure runs.
    writes: HashMap<InMemoryStorageKey, MonoValue>,
    /// The events the execution emitted, already serialized (an aborted
    /// incarnation never gets here, so only committed-or-superseded runs pay
    /// the serialization). Included in the committed output for comparison
    /// against the legacy VM.
    events: Vec<ContractEvent>,
    /// The incarnation the record was produced by, when run under BlockSTMv2.
    incarnation: Option<Incarnation>,
}

impl MonoRecord {
    /// A record of a completed execution (successful or failed; failed runs
    /// pass empty writes and events but keep their reads so validation still
    /// catches stale speculative reads).
    pub fn from_execution(
        reads: HashMap<InMemoryStorageKey, Version>,
        writes: HashMap<InMemoryStorageKey, MonoValue>,
        events: Vec<ContractEvent>,
        incarnation: Option<Incarnation>,
    ) -> Self {
        Self {
            status: MonoRecordStatus::Success,
            reads,
            writes,
            events,
            incarnation,
        }
    }

    /// A record for a transaction the mono path does not execute (non-entry
    /// payloads, system transactions): no reads, no writes, trivially valid.
    pub fn empty_success(incarnation: Option<Incarnation>) -> Self {
        Self::from_execution(HashMap::new(), HashMap::new(), vec![], incarnation)
    }

    /// A record of an incarnation that observed an inconsistent speculative
    /// state. Its own validation always fails, guaranteeing re-execution.
    pub fn speculative_failure(msg: String, incarnation: Option<Incarnation>) -> Self {
        Self {
            status: MonoRecordStatus::SpeculativeFailure(msg),
            reads: HashMap::new(),
            writes: HashMap::new(),
            events: vec![],
            incarnation,
        }
    }
}

impl Record for MonoRecord {
    type CommittedOutput = TransactionOutput;
    type Error = VMStatus;
    type Key = InMemoryStorageKey;
    type Tag = StructTag;
    type Txn = SignatureVerifiedTransaction;
    type Value = MonoValue;

    fn status(&self) -> RecordStatus<'_, VMStatus> {
        match &self.status {
            MonoRecordStatus::Success => RecordStatus::Success,
            MonoRecordStatus::SpeculativeFailure(msg) => RecordStatus::SpeculativeFailure(msg),
        }
    }

    fn capture_delayed_field_read_error(&mut self, _e: &PanicOr<MVDelayedFieldsError>) {
        // Delayed fields are disabled on the mono path; speculative failures
        // are guaranteed to fail validation via the status check instead.
    }

    fn resource_write_set(&self) -> Result<HashMap<Self::Key, Self::Value>, PanicError> {
        Ok(self.writes.clone())
    }

    #[allow(clippy::type_complexity)]
    fn resource_group_write_set(
        &self,
    ) -> Result<
        HashMap<
            Self::Key,
            (
                Self::Value,
                ResourceGroupSize,
                BTreeMap<Self::Tag, Self::Value>,
            ),
        >,
        PanicError,
    > {
        Ok(HashMap::new())
    }

    fn resource_group_tags(&self) -> Result<Vec<(Self::Key, HashSet<Self::Tag>)>, PanicError> {
        Ok(vec![])
    }

    fn delayed_field_change_set(
        &self,
    ) -> Result<BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>, PanicError> {
        Ok(BTreeMap::new())
    }

    // Iterates exactly the keys of `resource_write_set`: the executor marks
    // these as estimates on abort (V1) and diffs them against the next
    // incarnation's writes (V2), so the two key sets must coincide.
    fn for_each_resource_key(
        &self,
        callback: &mut dyn FnMut(&Self::Key) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        for key in self.writes.keys() {
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
        // Module publishing is unsupported on the mono path.
        Ok(())
    }

    fn validate_data_reads(
        &self,
        data_map: &VersionedData<Self::Key, Self::Value>,
        idx_to_validate: TxnIndex,
    ) -> bool {
        // A speculative failure must never validate: it is stored like any
        // other record, and only failed validation triggers re-execution.
        if self.is_speculative_failure() {
            return false;
        }
        self.reads.iter().all(|(key, version)| {
            match data_map.fetch_data_no_record(key, idx_to_validate) {
                Ok(MVDataOutput::Versioned(fetched, _)) => to_mono_version(fetched) == *version,
                Err(MVDataError::Uninitialized) => *version == Version::Storage,
                Err(MVDataError::Dependency(_)) => false,
            }
        })
    }

    fn validate_group_reads(
        &self,
        _group_map: &VersionedGroupData<Self::Key, Self::Tag, Self::Value>,
        _idx_to_validate: TxnIndex,
    ) -> bool {
        true
    }

    fn validate_module_reads(
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
        // Mid-block module publishing is unsupported, so code never changes
        // under an executing block.
        true
    }

    fn validate_delayed_field_reads(
        &self,
        _delayed_fields: &dyn TVersionedDelayedFieldView<DelayedFieldID>,
        _idx_to_validate: TxnIndex,
    ) -> Result<bool, PanicError> {
        // The commit path re-executes a transaction only when this check (or
        // wave validation) fails, so a speculative failure must fail here too.
        Ok(!self.is_speculative_failure())
    }

    fn blockstm_v2_incarnation(&self) -> Option<Incarnation> {
        self.incarnation
    }

    fn read_summary(&self) -> HashSet<InputOutputKey<Self::Key, Self::Tag>> {
        self.reads
            .keys()
            .map(|key| InputOutputKey::Resource(key.clone()))
            .collect()
    }

    fn write_summary(&self) -> Result<HashSet<InputOutputKey<Self::Key, Self::Tag>>, PanicError> {
        Ok(self
            .writes
            .keys()
            .map(|key| InputOutputKey::Resource(key.clone()))
            .collect())
    }

    fn fee_statement(&self) -> Result<FeeStatement, PanicError> {
        // No gas metering on the mono path.
        Ok(FeeStatement::zero())
    }

    fn has_new_epoch_event(&self) -> Result<bool, PanicError> {
        Ok(self.events.iter().any(ContractEvent::is_new_epoch_event))
    }

    fn output_approx_size(&self) -> Result<u64, PanicError> {
        // Stub outputs are empty; effectively disables block output limits.
        Ok(0)
    }

    fn accumulate_hot_state(
        &self,
        _block_limit_processor: &mut BlockGasLimitProcessor<
            <Self::Txn as Transaction>::Key,
            Self::Key,
            Self::Tag,
        >,
    ) -> Result<(), PanicError> {
        Ok(())
    }

    fn sequential_group_serialization_error(
        &self,
        _unsync_map: &UnsyncMap<Self::Key, Self::Tag, Self::Value, DelayedFieldID>,
    ) -> Result<bool, PanicError> {
        Ok(false)
    }

    fn sequential_incorrect_use(&self) -> bool {
        false
    }

    fn materialize<S: TStateView<Key = <Self::Txn as Transaction>::Key> + Sync>(
        &self,
        _args: &ViewArgs<'_, Self, S>,
        _txn_idx: TxnIndex,
        _environment: &AptosEnvironment,
    ) -> Result<Self::CommittedOutput, PanicOr<ResourceGroupSerializationError>> {
        // P0 stub with real events: the write set stays empty (commit-time
        // serialization comes later), but the emitted events are included so
        // they can be compared against the legacy VM.
        Ok(TransactionOutput::new(
            WriteSet::default(),
            self.events.clone(),
            0,
            TransactionStatus::Keep(ExecutionStatus::Success),
            TransactionAuxiliaryData::None,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_mvhashmap::MVHashMap;
    use aptos_types::write_set::WriteOpKind;
    use mono_move_alloc::GlobalArenaPtr;
    use mono_move_core::types::Type;
    use mono_move_runtime::Heap;
    use move_core_types::account_address::AccountAddress;
    use std::{ptr::NonNull, sync::Arc};

    // An `InternedType` is just an arena pointer; a `'static` node gives a
    // stable one without standing up an interner.
    static TY: Type = Type::U64;

    fn key(n: u8) -> InMemoryStorageKey {
        let mut bytes = [0u8; AccountAddress::LENGTH];
        bytes[AccountAddress::LENGTH - 1] = n;
        InMemoryStorageKey::Resource {
            address: AccountAddress::new(bytes),
            ty: GlobalArenaPtr::from_static(&TY),
        }
    }

    fn value() -> MonoValue {
        MonoValue::Write {
            // The map never dereferences stored values, so any non-null
            // pointer is a valid stand-in.
            ptr: NonNull::new(0x10 as *mut u8).expect("non-null"),
            kind: WriteOpKind::Creation,
            heap: Arc::new(Heap::new(64)),
        }
    }

    fn map() -> MVHashMap<InMemoryStorageKey, StructTag, MonoValue, DelayedFieldID> {
        MVHashMap::new()
    }

    fn record_with_read(key: InMemoryStorageKey, version: Version) -> MonoRecord {
        MonoRecord::from_execution(
            std::iter::once((key, version)).collect(),
            HashMap::new(),
            vec![],
            None,
        )
    }

    #[test]
    fn storage_read_validates_against_empty_map() {
        let map = map();
        let record = record_with_read(key(1), Version::Storage);
        assert!(record.validate_data_reads(map.data(), 5));
    }

    #[test]
    fn storage_read_invalidated_by_lower_write() {
        let map = map();
        map.data().write(key(1), 2, 0, value()).unwrap();
        let record = record_with_read(key(1), Version::Storage);
        assert!(!record.validate_data_reads(map.data(), 5));
    }

    #[test]
    fn higher_writes_are_invisible() {
        let map = map();
        map.data().write(key(1), 5, 0, value()).unwrap();
        let record = record_with_read(key(1), Version::Storage);
        // Validating index 3: the write at 5 is not visible below it.
        assert!(record.validate_data_reads(map.data(), 3));
    }

    #[test]
    fn versioned_read_validates_same_incarnation_only() {
        let map = map();
        map.data().write(key(1), 2, 0, value()).unwrap();
        let read = Version::Write {
            txn_idx: 2,
            incarnation: 0,
        };
        assert!(record_with_read(key(1), read).validate_data_reads(map.data(), 5));

        // A re-execution of txn 2 bumps the incarnation: the read is stale.
        map.data().write(key(1), 2, 1, value()).unwrap();
        assert!(!record_with_read(key(1), read).validate_data_reads(map.data(), 5));
    }

    #[test]
    fn estimate_fails_validation() {
        let map = map();
        map.data().write(key(1), 2, 0, value()).unwrap();
        map.data().mark_estimate(&key(1), 2);
        let read = Version::Write {
            txn_idx: 2,
            incarnation: 0,
        };
        assert!(!record_with_read(key(1), read).validate_data_reads(map.data(), 5));
    }

    #[test]
    fn speculative_failure_never_validates() {
        let map = map();
        let record = MonoRecord::speculative_failure("dependency".to_string(), None);
        // No reads to mismatch, yet validation must fail to force
        // re-execution.
        assert!(!record.validate_data_reads(map.data(), 5));
        assert!(!record
            .validate_delayed_field_reads(map.delayed_fields(), 5)
            .unwrap());
    }
}
