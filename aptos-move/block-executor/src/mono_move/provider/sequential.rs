// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The resource provider backing MonoMove sequential Block-STM execution.
//! Created once per transaction on top of existing storage and caching
//! layers.

use crate::mono_move::{
    provider::{invariant_violation, materialized_value, storage_read},
    MonoValue,
};
use anyhow::{anyhow, Result};
use aptos_mvhashmap::{types::UnsyncGroupError, unsync_map::UnsyncMap};
use aptos_types::state_store::{state_key::StateKey, TStateView};
use mono_move_aptos_transaction_executor::{
    check_group_key, decode_group_members, AptosDataProvider, GroupAssembly, GroupMembers,
};
use mono_move_core::{
    intern_struct_tag, nominal_tag,
    storage::resource_provider::{
        InMemoryStorageKey, ResourceProvider, ResourceProviderError, StorageRead,
    },
    types::InternedType,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::SegmentedArena;
use move_core_types::language_storage::StructTag;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;

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

    /// Returns the group's stored members, decoded from the base view.
    fn read_group_from_storage(
        &self,
        group_key: &InMemoryStorageKey,
    ) -> Result<Option<GroupMembers>> {
        Ok(
            match self
                .base_view
                .get_state_value(&group_key.as_state_key()?)
                .map_err(|e| anyhow!("group read failed: {e}"))?
            {
                Some(value) => Some(decode_group_members(value.bytes())?),
                None => None,
            },
        )
    }

    /// Returns the group's current members, read from the map (cached or written
    /// by some previously executed transaction) or the base view.
    fn fetch_resource_group(&self, group_key: &InMemoryStorageKey) -> Result<Option<GroupMembers>> {
        if let Some(rg) = self.unsync_map.get_group(group_key) {
            return Ok(rg);
        }

        let members = self.read_group_from_storage(group_key)?;
        self.unsync_map
            .insert_group(group_key.clone(), members.clone());
        Ok(members)
    }

    /// Reads one member of a resource group. `key` names the member's own slot,
    /// which is what gives its type; `group_key` and `tag` locate it in the map.
    fn read_group_member(
        &self,
        key: &InMemoryStorageKey,
        group_key: &InMemoryStorageKey,
        tag: &StructTag,
    ) -> Result<StorageRead, ResourceProviderError> {
        loop {
            match self.unsync_map.fetch_group_tagged_data(group_key, tag) {
                Ok(MonoValue::ResourceGroupMemberBytes(blob)) => {
                    // A stored group does not name its members' types, so the
                    // first reader that knows one replaces the bytes with the
                    // flat value.
                    let value = self.materialized_value(key, &blob)?;
                    let read = storage_read(&value, None)?;
                    self.unsync_map.update_tagged_base_value_with_layout(
                        group_key.clone(),
                        tag.clone(),
                        value,
                    );
                    return Ok(read);
                },
                Ok(value) => return storage_read(&value, None),
                Err(UnsyncGroupError::Uninitialized) => {
                    let members = self
                        .read_group_from_storage(group_key)
                        .map_err(|e| invariant_violation(format!("{e:#}")))?
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(tag, blob)| (tag, MonoValue::ResourceGroupMemberBytes(blob)));
                    self.unsync_map
                        .set_group_base_values(group_key.clone(), members)
                        .map_err(|e| {
                            invariant_violation(format!("Failed to set group base values: {e:#}"))
                        })?;
                },
                // The group has no such member. Like the legacy VM, sequential
                // execution records nothing: a later creation must find the
                // tag vacant.
                Err(UnsyncGroupError::TagNotFound) => {
                    return Ok(StorageRead::DoesNotExist { version: None })
                },
            }
        }
    }

    /// Reads a resource-group member that the map holds under its own key.
    ///
    /// The first miss on any member of a group provisions all of its stored
    /// members as encoded pre-block values, then puts the group sentinel at
    /// the group's own slot. With the sentinel present, a miss means the
    /// member is not in the group.
    fn read_flat_group_member(
        &self,
        key: &InMemoryStorageKey,
        group_key: &InMemoryStorageKey,
        tag: &StructTag,
    ) -> Result<StorageRead, ResourceProviderError> {
        match self.unsync_map.fetch_data(key) {
            Some(MonoValue::ResourceGroupMemberBytes(blob)) => {
                return self.cache_existing_value(key, &blob)
            },
            Some(value) => return storage_read(&value, None),
            None => {},
        }
        if let Some(MonoValue::GroupMetadata) = self.unsync_map.fetch_data(group_key) {
            return Ok(self.cache_not_existing_value(key));
        }

        let members = self
            .read_group_from_storage(group_key)
            .map_err(|e| invariant_violation(format!("{e:#}")))?
            .unwrap_or_default();
        let mut blob = None;
        for (member_tag, member_blob) in members {
            // TODO(metering): every stored member is interned, including
            // members no transaction reads.
            let member_ty = intern_struct_tag(&member_tag, self.guard)
                .map_err(|e| invariant_violation(format!("{e:#}")))?;
            if &member_tag == tag {
                blob = Some(member_blob.clone());
            }
            // The map also holds this block's writes, which a pre-block value
            // must not replace.
            let member_key = InMemoryStorageKey::resource(group_key.address(), member_ty);
            if self.unsync_map.fetch_data(&member_key).is_none() {
                self.unsync_map
                    .set_base_value(member_key, MonoValue::ResourceGroupMemberBytes(member_blob));
            }
        }
        self.unsync_map
            .set_base_value(group_key.clone(), MonoValue::GroupMetadata);
        match blob {
            Some(blob) => self.cache_existing_value(key, &blob),
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
                let member_tag = nominal_tag(key.value_ty())
                    .map_err(|e| invariant_violation(format!("{e:#}")))?;
                if GroupAssembly::get().flat_members() {
                    self.read_flat_group_member(key, &group_key, &member_tag)
                } else {
                    self.read_group_member(key, &group_key, &member_tag)
                }
            },
        }
    }
}

impl<S: TStateView<Key = StateKey>> AptosDataProvider for BlockSTMSequentialProvider<'_, '_, S> {
    fn group_members(
        &self,
        group_key: &InMemoryStorageKey,
    ) -> Result<Option<GroupMembers>, ResourceProviderError> {
        check_group_key(group_key)?;
        self.fetch_resource_group(group_key)
            .map_err(|e| invariant_violation(format!("{e:#}")))
    }
}
