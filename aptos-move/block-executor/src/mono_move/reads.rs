// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! MonoMove's Block-STM read set.
//!
//! MonoMove records its reads inside the VM, in the session's read-write set.
//! The read set validates straight from it, so the reads keep the values they
//! observed.

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
use mono_move_aptos_transaction_executor::TxnOutcome;
use mono_move_core::storage::resource_provider::{InMemoryStorageKey, Version};
use move_binary_format::CompiledModule;
use move_core_types::language_storage::ModuleId;
use move_vm_runtime::Module;
use move_vm_types::{code::SyncModuleCache, delayed_values::delayed_field_id::DelayedFieldID};
use std::{
    collections::{BTreeSet, HashSet},
    sync::Arc,
};

/// A MonoMove transaction's read set, as Block-STM validates it.
pub struct MonoReads {
    /// The outcome whose read-write set holds the reads, shared with the
    /// transaction's output. [`None`] for a transaction whose reads were never
    /// collected: sequential execution, or a parallel execution that has
    /// nothing left to commit.
    outcome: Option<Arc<TxnOutcome>>,
    /// The incarnation these reads were captured at, for BlockSTMv2. [`None`]
    /// only under sequential execution, which does not validate.
    incarnation: Option<Incarnation>,
    /// Set when the transaction is known not to be committable regardless of
    /// what it read. Fails validation unconditionally.
    speculative_failure: bool,
}

// SAFETY: the outcome's effects only point into frozen heaps, which are never
// mutated again and which the effects and their reads pin. Their keys hold
// interned types from the global arena, which outlives the block.
unsafe impl Send for MonoReads {}
unsafe impl Sync for MonoReads {}

impl MonoReads {
    pub(crate) fn new(outcome: Arc<TxnOutcome>, incarnation: Incarnation) -> Self {
        Self {
            outcome: Some(outcome),
            incarnation: Some(incarnation),
            speculative_failure: false,
        }
    }

    /// A read set that validates trivially.
    pub(crate) fn empty(incarnation: Option<Incarnation>) -> Self {
        Self {
            outcome: None,
            incarnation,
            speculative_failure: false,
        }
    }
}

/// Whether a version recorded by the VM still names the entry Block-STM serves
/// now. A read of pre-block storage is recorded as [`None`].
pub(crate) fn version_matches(recorded: Version, observed: MVVersion) -> bool {
    match (recorded, observed) {
        (None, Err(StorageVersion)) => true,
        (Some(recorded), Ok(observed)) => recorded == observed,
        (None, Ok(_)) | (Some(_), Err(StorageVersion)) => false,
    }
}

impl TxnInput for MonoReads {
    type Key = InMemoryStorageKey;
    // MonoMove versions group members as plain resources, so Block-STM's group
    // tags are never used.
    type Tag = ();
    type Value = MonoValue;

    fn validate_data_reads(
        &self,
        data_map: &VersionedData<Self::Key, Self::Value>,
        idx_to_validate: TxnIndex,
    ) -> bool {
        if self.speculative_failure {
            return false;
        }
        let Some(effects) = self.outcome.as_ref().and_then(|outcome| outcome.effects()) else {
            return true;
        };
        effects
            .read_write_set()
            .reads_unordered()
            .all(|(key, recorded, _group)| {
                match data_map.fetch_data_no_record(key, idx_to_validate) {
                    Ok(MVDataOutput::Versioned(observed, _)) => version_matches(recorded, observed),
                    // A base value this transaction provisioned cannot
                    // disappear, so this only happens if the entry was never
                    // there. Either way the recorded version is no longer
                    // backed by the map.
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
        let map = MVHashMap::<InMemoryStorageKey, (), MonoValue, DelayedFieldID>::new();
        let mut reads = MonoReads::empty(Some(0));
        assert!(reads.validate_data_reads(map.data(), 0));
        assert!(reads.validate_group_reads(map.group_data(), 0));

        reads.record_delayed_field_application_failure();
        assert!(!reads.validate_data_reads(map.data(), 0));
        assert!(!reads.validate_group_reads(map.group_data(), 0));
    }
}
