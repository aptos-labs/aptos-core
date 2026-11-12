// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! MonoMove's Block-STM read set.
//!
//! MonoMove records its reads inside the VM, in the transaction's
//! [`ResourceReadWriteSet`], so this type is built once at the end of execution
//! rather than being filled in as the reads happen.

use crate::{
    captured_reads::TxnInput, code_cache_global::GlobalModuleCache, mono_move::MonoValue,
    types::InputOutputKey,
};
use aptos_mvhashmap::{
    types::{
        Incarnation, MVDataError, MVDataOutput, StorageVersion, TxnIndex, Version as MVVersion,
    },
    versioned_data::VersionedData,
    versioned_delayed_fields::TVersionedDelayedFieldView,
    versioned_group_data::VersionedGroupData,
};
use aptos_types::{error::PanicError, vm::modules::AptosModuleExtension};
use mono_move_core::storage::resource_provider::{InMemoryStorageKey, Version};
use mono_move_runtime::ResourceReadWriteSet;
use move_binary_format::CompiledModule;
use move_core_types::language_storage::{ModuleId, StructTag};
use move_vm_runtime::Module;
use move_vm_types::{code::SyncModuleCache, delayed_values::delayed_field_id::DelayedFieldID};
use serde::Serialize;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fmt::Debug,
    hash::Hash,
    marker::PhantomData,
};

/// The versions a MonoMove transaction observed. Resource-group members are
/// read under their own keys, like any other resource, so `T` only names the
/// tag type Block-STM's group interface expects.
pub struct MonoReads<K, T> {
    data_reads: HashMap<K, Version>,
    /// The incarnation these reads were captured at, for BlockSTMv2. [`None`]
    /// only under sequential execution, which does not validate.
    incarnation: Option<Incarnation>,
    /// Set when the transaction is known not to be committable regardless of
    /// what it read. Fails validation unconditionally.
    speculative_failure: bool,
    _tag: PhantomData<T>,
}

impl<K, T> Default for MonoReads<K, T> {
    fn default() -> Self {
        Self {
            data_reads: HashMap::new(),
            incarnation: None,
            speculative_failure: false,
            _tag: PhantomData,
        }
    }
}

impl<K, T> MonoReads<K, T> {
    /// A read set that validates trivially, for a transaction whose reads were
    /// never collected: sequential execution, or a parallel execution that has
    /// nothing left to commit.
    pub(crate) fn empty(incarnation: Option<Incarnation>) -> Self {
        Self {
            incarnation,
            ..Default::default()
        }
    }
}

impl<K: Eq + Hash, T> MonoReads<K, T> {
    /// Records a read of a value in its own storage slot.
    pub(crate) fn record_data_read(&mut self, key: K, version: Version) {
        self.data_reads.insert(key, version);
    }
}

impl MonoReads<InMemoryStorageKey, StructTag> {
    /// Collects the versions a finished transaction observed.
    pub(crate) fn from_read_write_set(
        rws: &ResourceReadWriteSet,
        incarnation: Incarnation,
    ) -> Self {
        let mut reads = Self {
            incarnation: Some(incarnation),
            ..Default::default()
        };
        for (key, version, _group) in rws.reads_unordered() {
            reads.record_data_read(key.clone(), version);
        }
        reads
    }
}

/// Whether a version recorded by the VM still names the entry Block-STM serves
/// now. A read of pre-block storage is recorded as [`None`].
fn version_matches(recorded: Version, observed: MVVersion) -> bool {
    match (recorded, observed) {
        (None, Err(StorageVersion)) => true,
        (Some(recorded), Ok(observed)) => recorded == observed,
        (None, Ok(_)) | (Some(_), Err(StorageVersion)) => false,
    }
}

impl<K, T> TxnInput for MonoReads<K, T>
where
    K: Send + Sync + Clone + Hash + Eq + Debug + 'static,
    T: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + Debug + Serialize + 'static,
{
    type Key = K;
    type Tag = T;
    type Value = MonoValue;

    fn validate_data_reads(
        &self,
        data_map: &VersionedData<Self::Key, Self::Value>,
        idx_to_validate: TxnIndex,
    ) -> bool {
        if self.speculative_failure {
            return false;
        }
        self.data_reads.iter().all(|(key, recorded)| {
            match data_map.fetch_data_no_record(key, idx_to_validate) {
                Ok(MVDataOutput::Versioned(observed, _)) => version_matches(*recorded, observed),
                // A base value this transaction provisioned cannot disappear, so
                // this only happens if the entry was never there. Either way the
                // recorded version is no longer backed by the map.
                Err(MVDataError::Uninitialized) => false,
                Err(MVDataError::Dependency(_)) => false,
            }
        })
    }

    fn validate_group_reads(
        &self,
        _group_map: &VersionedGroupData<Self::Key, Self::Tag, Self::Value>,
        _idx_to_validate: TxnIndex,
    ) -> bool {
        // Group members are validated as data reads.
        !self.speculative_failure
    }

    fn validate_delayed_field_reads(
        &self,
        _delayed_fields: &dyn TVersionedDelayedFieldView<DelayedFieldID>,
        _idx_to_validate: TxnIndex,
    ) -> Result<bool, PanicError> {
        // TODO(completeness): support delayed fields.
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
        // Module publishing in the middle of the block is not supported by
        // MonoMove.
        true
    }

    fn record_delayed_field_application_failure(&mut self) {
        // BlockSTMv1 has no channel of its own for a speculative failure and
        // reuses this one, so the flag must make validation fail even though
        // MonoMove has no delayed fields yet.
        self.speculative_failure = true;
    }

    fn incarnation(&self) -> Option<Incarnation> {
        self.incarnation
    }

    fn is_incorrect_use(&self) -> bool {
        // A read that cannot be served aborts the transaction speculatively, so
        // a read set that reaches validation is always well formed.
        false
    }

    fn get_read_summary(&self) -> HashSet<InputOutputKey<Self::Key, Self::Tag>> {
        // TODO(perf): this disables the block-limit conflict heuristics.
        HashSet::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_mvhashmap::MVHashMap;

    #[test]
    fn speculative_failure_fails_validation() {
        // BlockSTMv1 marks a doomed transaction this way and then relies on
        // validation to fail so the transaction re-executes.
        let map = MVHashMap::<u32, u32, MonoValue, DelayedFieldID>::new();
        let mut reads = MonoReads::<u32, u32>::empty(Some(0));
        assert!(reads.validate_data_reads(map.data(), 0));
        assert!(reads.validate_group_reads(map.group_data(), 0));

        reads.record_delayed_field_application_failure();
        assert!(!reads.validate_data_reads(map.data(), 0));
        assert!(!reads.validate_group_reads(map.group_data(), 0));
    }
}
