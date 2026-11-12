// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The resource provider backing MonoMove sequential Block-STM execution.
//! Created once per transaction on top of existing storage and caching
//! layers.

use crate::mono_move::{
    group_chain::{member_tag, GroupLink, GroupView},
    provider::{decode_group_view, invariant_violation, materialized_value, storage_read},
    MonoValue,
};
use aptos_mvhashmap::unsync_map::UnsyncMap;
use aptos_types::state_store::{state_key::StateKey, TStateView};
use mono_move_core::{
    storage::resource_provider::{
        InMemoryStorageKey, ResourceProvider, ResourceProviderError, StorageRead,
    },
    types::InternedType,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::SegmentedArena;
use move_core_types::language_storage::StructTag;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::sync::Arc;

type MonoUnsyncMap = UnsyncMap<InMemoryStorageKey, StructTag, MonoValue, DelayedFieldID>;

/// Provides access to resources and resource groups.
pub(crate) struct BlockSTMSequentialProvider<'a, 'ctx, S> {
    guard: &'a ExecutionGuard<'ctx>,
    /// Base storage layer, pre-block state.
    base_view: &'a S,
    /// Caches writes performed by previous transactions.
    unsync_map: &'a MonoUnsyncMap,
    /// Where values read from the base storage layer are materialized.
    arena: &'a SegmentedArena,
}

impl<'a, 'ctx, S: TStateView<Key = StateKey>> BlockSTMSequentialProvider<'a, 'ctx, S> {
    pub(crate) fn new(
        guard: &'a ExecutionGuard<'ctx>,
        base_view: &'a S,
        unsync_map: &'a MonoUnsyncMap,
        arena: &'a SegmentedArena,
    ) -> Self {
        Self {
            guard,
            base_view,
            unsync_map,
            arena,
        }
    }

    /// Converts BCS value into MonoMove memory representation and records it
    /// in the cache.
    fn cache_existing_value(
        &self,
        key: &InMemoryStorageKey,
        blob: &[u8],
    ) -> Result<StorageRead, ResourceProviderError> {
        let value = self.materialized_value(key, blob)?;
        let read = storage_read(&value, None)?;
        self.unsync_map.set_base_value(key.clone(), value);
        Ok(read)
    }

    /// Records non existing entry in the cache.
    fn cache_not_existing_value(&self, key: &InMemoryStorageKey) -> StorageRead {
        self.unsync_map
            .set_base_value(key.clone(), MonoValue::Deletion);
        StorageRead::DoesNotExist { version: None }
    }

    /// The group's stored members, decoded from the base view. Empty if no
    /// group is stored there.
    fn stored_group_view(
        &self,
        group_key: &InMemoryStorageKey,
    ) -> Result<GroupView, ResourceProviderError> {
        let state_key = group_key
            .as_state_key()
            .map_err(|e| invariant_violation(format!("{e:#}")))?;
        match self
            .base_view
            .get_state_value(&state_key)
            .map_err(|e| invariant_violation(format!("Group read failed: {e}")))?
        {
            Some(value) => decode_group_view(value.bytes()),
            None => Ok(GroupView::new()),
        }
    }

    /// Reads one member of a resource group. `key` names the member's own slot,
    /// which is where the map holds it.
    ///
    /// A member no transaction wrote in this block is looked up in the view at
    /// the group's slot. That is the link of the latest materialized writer, or
    /// the stored group's link, so its view is the group as this transaction
    /// sees it.
    fn read_group_member(
        &self,
        key: &InMemoryStorageKey,
        group_key: &InMemoryStorageKey,
    ) -> Result<StorageRead, ResourceProviderError> {
        if let Some(value) = self.unsync_map.fetch_data(key) {
            return storage_read(&value, None);
        }
        let link = match self.unsync_map.fetch_data(group_key) {
            Some(MonoValue::GroupLink(link)) => link,
            Some(MonoValue::Write { .. } | MonoValue::Deletion) => {
                return Err(invariant_violation(
                    "A group slot holds a value that is not a link",
                ))
            },
            None => {
                let link = Arc::new(GroupLink::stored(self.stored_group_view(group_key)?));
                self.unsync_map
                    .set_base_value(group_key.clone(), MonoValue::GroupLink(link.clone()));
                link
            },
        };
        let view = link
            .built_view()
            .ok_or_else(|| invariant_violation("The group slot's link has no view"))?;
        let tag = member_tag(key.value_ty()).map_err(invariant_violation)?;
        match view.get(&tag) {
            Some(blob) => self.cache_existing_value(key, blob),
            None => Ok(self.cache_not_existing_value(key)),
        }
    }

    /// Materializes a stored group member, whose type `key` names.
    fn materialized_value(
        &self,
        key: &InMemoryStorageKey,
        blob: &[u8],
    ) -> Result<MonoValue, ResourceProviderError> {
        materialized_value(self.guard, self.arena, key.value_ty(), blob)
    }
}

impl<S: TStateView<Key = StateKey>> ResourceProvider for BlockSTMSequentialProvider<'_, '_, S> {
    fn get_resource(
        &self,
        key: &InMemoryStorageKey,
        group: Option<InternedType>,
    ) -> Result<StorageRead, ResourceProviderError> {
        match group {
            None => {
                // If there is a value already in the map, it is most-up-to-date
                // modification, and we return it.
                if let Some(value) = self.unsync_map.fetch_data(key) {
                    return storage_read(&value, None);
                }

                // Otherwise, need to fetch from base view.
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
                let group_key = InMemoryStorageKey::resource_group(key.address(), group_ty);
                self.read_group_member(key, &group_key)
            },
        }
    }
}
