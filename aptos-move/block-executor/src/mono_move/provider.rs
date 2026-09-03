// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The resource provider backing MonoMove sequential Block-STM execution.
//! Created once per transaction on top of existing storage and caching
//! layers.

use crate::mono_move::MonoValue;
use anyhow::{anyhow, Result};
use aptos_mvhashmap::unsync_map::UnsyncMap;
use aptos_types::{
    state_store::{state_key::StateKey, TStateView},
    write_set::WriteOpKind,
};
use mono_move_aptos_transaction_executor::{decode_group_members, AptosDataProvider, GroupMembers};
use mono_move_core::{
    nominal_tag,
    storage::resource_provider::{
        InMemoryStorageKey, ResourceProvider, ResourceProviderError, StorageRead,
    },
    types::InternedType,
    LayoutProvider, ReadPin, OBJECT_HEADER_SIZE,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{deserialize_into, Heap, SharedArena};
use move_core_types::language_storage::StructTag;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{ptr::NonNull, sync::Arc};

type MonoUnsyncMap = UnsyncMap<InMemoryStorageKey, StructTag, MonoValue, DelayedFieldID>;

/// Provides access to resources and resource groups.
pub(crate) struct BlockSTMSequentialProvider<'a, 'ctx, S> {
    guard: &'a ExecutionGuard<'ctx>,
    /// Base storage layer, pre-block state.
    base_view: &'a S,
    /// Caches writes performed by previous transactions.
    unsync_map: &'a MonoUnsyncMap,
}

impl<'a, 'ctx, S: TStateView<Key = StateKey>> BlockSTMSequentialProvider<'a, 'ctx, S> {
    pub(crate) fn new(
        guard: &'a ExecutionGuard<'ctx>,
        base_view: &'a S,
        unsync_map: &'a MonoUnsyncMap,
    ) -> Self {
        Self {
            guard,
            base_view,
            unsync_map,
        }
    }

    /// Converts BCS bytes into MonoMove memory representation and stores the
    /// data into arena.
    fn deserialize_into_arena(
        &self,
        key: &InMemoryStorageKey,
        blob: &[u8],
    ) -> Result<(NonNull<u8>, Arc<SharedArena>), ResourceProviderError> {
        let value_ty = key.value_ty();

        let layout = self
            .guard
            .layout_by_ty(value_ty)
            .ok_or_else(|| invariant_violation("No layout found when deserializing base value"))?;
        let descriptor = self.guard.struct_descriptor(value_ty).ok_or_else(|| {
            invariant_violation("No GC descriptor found when deserializing base value")
        })?;

        let arena = self
            .unsync_map
            .resource_arena()
            .expect("Arena always exists for MonoMove execution");

        let obj = arena.with_heap_mut(|heap: &mut Heap| -> Result<NonNull<u8>, _> {
            // TODO(completeness): ensure this never errors! Our arena should grow.
            let obj = heap
                .alloc_object(OBJECT_HEADER_SIZE + layout.size as usize, descriptor)
                .ok_or_else(|| invariant_violation("Base value arena is full"))?;

            // SAFETY: `obj` is a freshly reserved object sized for the value's
            // layout; `deserialize_into` writes the flat value there.
            unsafe { deserialize_into(self.guard, heap, value_ty, blob, obj.as_ptr()) }.map_err(
                |e| invariant_violation(format!("Storage value failed to deserialize: {e}")),
            )?;
            Ok(obj)
        })?;
        Ok((obj, arena))
    }

    /// Converts BCS value into MonoMove memory representation and records it
    /// in the cache.
    fn cache_existing_value(
        &self,
        key: &InMemoryStorageKey,
        blob: &[u8],
    ) -> Result<StorageRead, ResourceProviderError> {
        let (ptr, arena) = self.deserialize_into_arena(key, blob)?;
        self.unsync_map
            .set_base_value(key.clone(), MonoValue::Write {
                ptr,
                // Base values that exist are cached as modifications. For
                // non-existing values we use deletions.
                kind: WriteOpKind::Modification,
                pin: arena.clone(),
            });
        Ok(read(ptr, arena))
    }

    /// Records non existing entry in the cache.
    fn cache_not_existing_value(&self, key: &InMemoryStorageKey) -> StorageRead {
        self.unsync_map
            .set_base_value(key.clone(), MonoValue::Deletion);
        StorageRead::DoesNotExist
    }

    /// Returns the group's current members, read from the map (cached or written
    /// by some previously executed transaction) or the base view.
    fn fetch_resource_group(&self, group_key: &StateKey) -> Result<Option<GroupMembers>> {
        if let Some(rg) = self.unsync_map.get_group(group_key) {
            return Ok(rg);
        }

        let members = match self
            .base_view
            .get_state_value(group_key)
            .map_err(|e| anyhow!("group read failed: {e}"))?
        {
            Some(value) => Some(decode_group_members(value.bytes())?),
            None => None,
        };
        self.unsync_map
            .insert_group(group_key.clone(), members.clone());
        Ok(members)
    }
}

impl<S: TStateView<Key = StateKey>> ResourceProvider for BlockSTMSequentialProvider<'_, '_, S> {
    fn get_resource(
        &self,
        key: &InMemoryStorageKey,
        group: Option<InternedType>,
    ) -> Result<StorageRead, ResourceProviderError> {
        // If there is a value already in the map, it is most-up-to-date
        // modification, and we return it.
        if let Some(value) = self.unsync_map.fetch_data(key) {
            return Ok(match value {
                MonoValue::Write { ptr, pin, .. } => read(ptr, pin),
                MonoValue::Deletion => StorageRead::DoesNotExist,
            });
        }

        // Otherwise, need to fetch from base view.
        match group {
            None => {
                let state_key = key
                    .as_state_key()
                    .map_err(|e| invariant_violation(format!("{e:#}")))?;
                match self.base_view.get_state_value(&state_key).map_err(|_| {
                    // TODO(completeness): this is not an invariant violation?
                    invariant_violation("Storage error")
                })? {
                    Some(value) => self.cache_existing_value(key, value.bytes()),
                    None => Ok(self.cache_not_existing_value(key)),
                }
            },
            Some(group_ty) => {
                let tag =
                    nominal_tag(group_ty).map_err(|e| invariant_violation(format!("{e:#}")))?;
                let group_state_key = StateKey::resource_group(&key.address(), &tag);

                let members = match self
                    .fetch_resource_group(&group_state_key)
                    .map_err(|e| invariant_violation(format!("{e:#}")))?
                {
                    Some(members) => members,
                    None => return Ok(StorageRead::DoesNotExist),
                };

                let member_tag = nominal_tag(key.value_ty())
                    .map_err(|e| invariant_violation(format!("{e:#}")))?;
                Ok(match members.get(&member_tag) {
                    Some(bytes) => self.cache_existing_value(key, bytes)?,
                    None => self.cache_not_existing_value(key),
                })
            },
        }
    }
}

impl<S: TStateView<Key = StateKey>> AptosDataProvider for BlockSTMSequentialProvider<'_, '_, S> {
    fn group_members(&self, group_key: &StateKey) -> Result<Option<GroupMembers>> {
        self.fetch_resource_group(group_key)
    }
}

fn read(ptr: NonNull<u8>, pin: Arc<dyn ReadPin>) -> StorageRead {
    StorageRead::ExternalHeap {
        ptr,
        // Version is irrelevant for sequential execution.
        version: 0,
        pin,
    }
}

fn invariant_violation(msg: impl ToString) -> ResourceProviderError {
    ResourceProviderError::InvariantViolation(msg.to_string())
}
