// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The resource provider backing MonoMove parallel Block-STM execution.
//! Created once per incarnation, on top of the multi-version map and the
//! pre-block state view.

use crate::{
    mono_move::{
        provider::{invariant_violation, materialized_value, ParallelReader, StorageBase},
        MonoValue,
    },
    scheduler_wrapper::SchedulerWrapper,
};
use anyhow::{anyhow, Result};
use aptos_mvhashmap::{
    types::{Incarnation, TxnIndex},
    MVHashMap,
};
use aptos_types::state_store::{state_key::StateKey, TStateView};
use bytes::Bytes;
use mono_move_aptos_transaction_executor::{decode_group_members, AptosDataProvider, GroupMembers};
use mono_move_core::{
    nominal_tag,
    storage::resource_provider::{
        InMemoryStorageKey, ResourceProvider, ResourceProviderError, StorageRead,
    },
    types::InternedType,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{serialize, SegmentedArena};
use move_core_types::language_storage::StructTag;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;

/// Provides access to resources and resource groups during parallel execution.
pub(crate) struct BlockSTMParallelProvider<'a, 'ctx, S> {
    guard: &'a ExecutionGuard<'ctx>,
    /// Base storage layer, pre-block state.
    base_view: &'a S,
    reader: ParallelReader<'a, InMemoryStorageKey, StructTag>,
    /// Where values read from the base storage layer are materialized.
    arena: &'a SegmentedArena,
}

impl<'a, 'ctx, S: TStateView<Key = StateKey>> BlockSTMParallelProvider<'a, 'ctx, S> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        guard: &'a ExecutionGuard<'ctx>,
        base_view: &'a S,
        versioned_map: &'a MVHashMap<InMemoryStorageKey, StructTag, MonoValue, DelayedFieldID>,
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

    /// The stored bytes of a group member the multi-version map holds.
    fn member_bytes(&self, value: &MonoValue) -> Result<Bytes> {
        match value {
            MonoValue::RawFromStorage(bytes) => Ok(bytes.clone()),
            MonoValue::Write { ptr, ty, .. } => {
                // SAFETY: the entry holding this pointer also pins the frozen
                // heap it points into, and that heap is never mutated again.
                let blob = unsafe { serialize(self.guard, ptr.as_ptr(), *ty) }
                    .map_err(|e| anyhow!("failed to serialize a group member: {e}"))?;
                Ok(Bytes::from(blob))
            },
            // Deleted members are dropped when the group is assembled, and a
            // group's own slot never appears among its members.
            MonoValue::Deletion => Err(anyhow!("a deleted group member was assembled")),
            MonoValue::GroupMetadata => Err(anyhow!("group metadata was assembled as a member")),
        }
    }
}

impl<S: TStateView<Key = StateKey>> StorageBase<InMemoryStorageKey, StructTag>
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

    fn group_members(
        &self,
        group_key: &InMemoryStorageKey,
    ) -> Result<Vec<(StructTag, Bytes)>, ResourceProviderError> {
        let Some(blob) = self.base_bytes(group_key)? else {
            return Ok(vec![]);
        };
        Ok(decode_group_members(&blob)
            .map_err(|e| invariant_violation(format!("Stored group failed to decode: {e:#}")))?
            .into_iter()
            .collect())
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
                let member_tag = nominal_tag(key.value_ty())
                    .map_err(|e| invariant_violation(format!("{e:#}")))?;
                self.reader
                    .read_group_member(self, key, &group_key, &member_tag)
            },
        }
    }
}

impl<S: TStateView<Key = StateKey>> AptosDataProvider for BlockSTMParallelProvider<'_, '_, S> {
    /// The group as the transactions committed before this one left it. An
    /// empty group is never stored, so it reads back as no group at all.
    fn group_members(
        &self,
        group_key: &InMemoryStorageKey,
    ) -> Result<Option<GroupMembers>, ResourceProviderError> {
        let members = self
            .reader
            .versioned_map()
            .group_data()
            .group_members_at(group_key, self.reader.txn_idx())
            .map_err(|e| {
                invariant_violation(format!("failed to read the members of {group_key:?}: {e:?}"))
            })?;
        if members.is_empty() {
            return Ok(None);
        }
        members
            .iter()
            .map(|(tag, value)| Ok((tag.clone(), self.member_bytes(value)?)))
            .collect::<Result<GroupMembers>>()
            .map(Some)
            .map_err(|e| invariant_violation(format!("{e:#}")))
    }
}
