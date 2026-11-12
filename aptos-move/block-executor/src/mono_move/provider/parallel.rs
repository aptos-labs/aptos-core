// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The resource provider backing MonoMove parallel Block-STM execution.
//! Created once per incarnation, on top of the multi-version map and the
//! pre-block state view.

use crate::{
    mono_move::{
        group_chain::{member_tag, GroupView},
        provider::{
            decode_group_view, invariant_violation, materialized_value, ParallelReader, StorageBase,
        },
        MonoValue,
    },
    scheduler_wrapper::SchedulerWrapper,
};
use aptos_mvhashmap::{
    types::{Incarnation, TxnIndex},
    MVHashMap,
};
use aptos_types::state_store::{state_key::StateKey, TStateView};
use bytes::Bytes;
use mono_move_core::{
    storage::resource_provider::{
        InMemoryStorageKey, ResourceProvider, ResourceProviderError, StorageRead,
    },
    types::InternedType,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::SegmentedArena;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;

/// Provides access to resources and resource groups during parallel execution.
pub(crate) struct BlockSTMParallelProvider<'a, 'ctx, S> {
    guard: &'a ExecutionGuard<'ctx>,
    /// Base storage layer, pre-block state.
    base_view: &'a S,
    reader: ParallelReader<'a, InMemoryStorageKey>,
    /// Where values read from the base storage layer are materialized.
    arena: &'a SegmentedArena,
}

impl<'a, 'ctx, S: TStateView<Key = StateKey>> BlockSTMParallelProvider<'a, 'ctx, S> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        guard: &'a ExecutionGuard<'ctx>,
        base_view: &'a S,
        versioned_map: &'a MVHashMap<InMemoryStorageKey, (), MonoValue, DelayedFieldID>,
        scheduler: SchedulerWrapper<'a>,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        arena: &'a SegmentedArena,
    ) -> Self {
        Self {
            guard,
            base_view,
            reader: ParallelReader::new(versioned_map, scheduler, txn_idx, incarnation),
            arena,
        }
    }

    /// Whether any read failed speculatively. Nothing this transaction produced
    /// may commit once this is set.
    pub(crate) fn speculative_failure(&self) -> bool {
        self.reader.speculative_failure()
    }

    /// The bytes stored at `key` before the block, or [`None`] for an empty
    /// slot.
    fn base_bytes(&self, key: &InMemoryStorageKey) -> Result<Option<Bytes>, ResourceProviderError> {
        let state_key = key
            .as_state_key()
            .map_err(|e| invariant_violation(format!("{e:#}")))?;
        Ok(self
            .base_view
            .get_state_value(&state_key)
            .map_err(|e| invariant_violation(format!("Storage read failed: {e}")))?
            .map(|value| value.bytes().clone()))
    }
}

impl<S: TStateView<Key = StateKey>> StorageBase<InMemoryStorageKey>
    for BlockSTMParallelProvider<'_, '_, S>
{
    fn resource(
        &self,
        key: &InMemoryStorageKey,
    ) -> Result<Option<MonoValue>, ResourceProviderError> {
        self.base_bytes(key)?
            .map(|blob| materialized_value(self.guard, self.arena, key.value_ty(), &blob))
            .transpose()
    }

    fn group_view(
        &self,
        group_key: &InMemoryStorageKey,
    ) -> Result<GroupView, ResourceProviderError> {
        match self.base_bytes(group_key)? {
            Some(blob) => decode_group_view(&blob),
            None => Ok(GroupView::new()),
        }
    }

    fn materialize_member(
        &self,
        key: &InMemoryStorageKey,
        blob: &Bytes,
    ) -> Result<MonoValue, ResourceProviderError> {
        materialized_value(self.guard, self.arena, key.value_ty(), blob)
    }
}

impl<S: TStateView<Key = StateKey>> ResourceProvider for BlockSTMParallelProvider<'_, '_, S> {
    fn get_resource(
        &self,
        key: &InMemoryStorageKey,
        group: Option<InternedType>,
    ) -> Result<StorageRead, ResourceProviderError> {
        match group {
            None => self.reader.read_resource(self, key),
            Some(group_ty) => {
                let group_key = InMemoryStorageKey::resource_group(key.address(), group_ty);
                // The tag is only needed to provision the member from the
                // stored group, on the block's first read of it.
                let tag = || member_tag(key.value_ty()).map_err(invariant_violation);
                self.reader.read_group_member(self, key, &group_key, tag)
            },
        }
    }
}
