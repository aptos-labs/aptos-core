// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The resource provider that backs MonoMove execution with the sequential
//! Block-STM multi-version map.
//!
//! A read is served from the `UnsyncMap` first (a write by an earlier
//! transaction, or a base value an earlier read cached), then from the base
//! state view. Base bytes are materialized (BCS -> flat) into the map's
//! append-only resource arena and cached back into the map, so each distinct
//! key is materialized at most once per block.
//!
//! Resource group members are first-class map entries under their own
//! `Resource` keys, just like own-slot resources: a member read materializes
//! only that one member (its bytes taken from the group's decoded base blob),
//! and a member write lands in the map under its own key. The group blob is
//! reassembled only at materialization, by
//! [`group_members`](BlockSTMProvider::group_members): it starts from the
//! stored blob and overlays every member the block has written (recorded in the
//! map's reverse index). Thanks to the sequential loop's per-transaction
//! interleaving, that overlay is exactly the writes through the transaction
//! being materialized.

use crate::mono_move::MonoValue;
use anyhow::{anyhow, bail, Result};
use aptos_mvhashmap::unsync_map::UnsyncMap;
use aptos_types::{
    state_store::{state_key::StateKey, table::TableHandle, TStateView},
    write_set::WriteOpKind,
};
use bytes::Bytes;
use mono_move_aptos_transaction_executor::{decode_group_members, AptosDataProvider, GroupMembers};
use mono_move_core::{
    intern_struct_tag,
    storage::resource_provider::{
        InMemoryStorageKey, ResourceProvider, ResourceProviderError, StorageRead,
    },
    struct_tag_of,
    types::InternedType,
    LayoutProvider, OBJECT_HEADER_SIZE,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{deserialize_into, serialize_value};
use move_core_types::language_storage::StructTag;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{cell::RefCell, collections::HashMap, ptr::NonNull};
use triomphe::Arc;

/// The sequential Block-STM map keyed/valued for MonoMove.
type MonoUnsyncMap = UnsyncMap<InMemoryStorageKey, StructTag, MonoValue, DelayedFieldID>;

/// Resource provider reading through the sequential multi-version map, then the
/// base state view.
pub(crate) struct BlockSTMProvider<'a, 'ctx, S> {
    guard: &'a ExecutionGuard<'ctx>,
    base_view: &'a S,
    unsync_map: &'a MonoUnsyncMap,
    inner: RefCell<ProviderState>,
}

/// The provider's interior-mutable caches. The materialized values and the
/// per-member writes live in the shared map, not here.
struct ProviderState {
    /// The stored (pre-block) members of each resource group read so far, keyed
    /// by the group's state key, caching absence as well. Reused to serve member
    /// reads and to seed group reassembly, so a group's blob is decoded once.
    group_base: HashMap<StateKey, Option<Arc<GroupMembers>>>,
}

impl<'a, 'ctx, S: TStateView<Key = StateKey>> BlockSTMProvider<'a, 'ctx, S> {
    pub(crate) fn new(
        guard: &'a ExecutionGuard<'ctx>,
        base_view: &'a S,
        unsync_map: &'a MonoUnsyncMap,
    ) -> Self {
        Self {
            guard,
            base_view,
            unsync_map,
            inner: RefCell::new(ProviderState {
                group_base: HashMap::new(),
            }),
        }
    }

    /// Materializes `blob` (BCS -> flat) into the map's append-only resource
    /// arena as the value of `key`, returning a pointer into the arena. The
    /// arena is never garbage-collected, so the pointer stays valid for the
    /// lifetime of the map.
    fn materialize_into_arena(
        &self,
        key: &InMemoryStorageKey,
        blob: &[u8],
    ) -> Result<NonNull<u8>, ResourceProviderError> {
        let internal = ResourceProviderError::InvariantViolation;
        let ty = key.value_ty();
        let layout = self
            .guard
            .layout_by_ty(ty)
            .ok_or_else(|| internal(format!("no layout for the value at {:?}", key.address())))?;
        // Lowering publishes a descriptor for every resource a global-storage
        // instruction reaches, so the read that got here has one already.
        let descriptor = self
            .guard
            .struct_descriptor(ty)
            .ok_or_else(|| internal(format!("no GC descriptor for {:?}", key.address())))?;

        self.unsync_map.with_resource_arena(|arena| {
            let obj = arena
                .alloc_object(OBJECT_HEADER_SIZE + layout.size as usize, descriptor)
                .ok_or_else(|| internal("resource arena is full".to_string()))?;
            // SAFETY: `obj` is a freshly reserved object sized for the value's
            // layout; `deserialize_into` writes the flat value there and boxes
            // any nested vectors in the same arena.
            unsafe { deserialize_into(self.guard, arena, ty, blob, obj.as_ptr()) }
                .map_err(|e| internal(format!("stored value failed to deserialize: {e}")))?;
            Ok(obj)
        })
    }

    /// Materializes `blob` as the value of `key`, caching it in the map so
    /// re-reads (this or a later transaction) hit the map instead of
    /// re-materializing. The cached value is a base value, not a write, so it
    /// never enters a write set.
    fn cache_base(
        &self,
        key: &InMemoryStorageKey,
        blob: &[u8],
    ) -> Result<StorageRead, ResourceProviderError> {
        let ptr = self.materialize_into_arena(key, blob)?;
        self.unsync_map.set_base_value(key.clone(), base_value(ptr));
        Ok(StorageRead::ExternalHeap { ptr, version: 0 })
    }

    /// The stored (pre-block) members of a resource group, decoded from the base
    /// view and cached (absence included), keyed by the group's state key.
    fn base_group(&self, group_state_key: &StateKey) -> Result<Option<Arc<GroupMembers>>> {
        if let Some(members) = self.inner.borrow().group_base.get(group_state_key) {
            return Ok(members.clone());
        }
        let members = match self
            .base_view
            .get_state_value(group_state_key)
            .map_err(|e| anyhow!("group read failed: {e}"))?
        {
            Some(value) => Some(Arc::new(decode_group_members(value.bytes(), self.guard)?)),
            None => None,
        };
        self.inner
            .borrow_mut()
            .group_base
            .insert(group_state_key.clone(), members.clone());
        Ok(members)
    }

    /// Lowers an in-memory group slot key to its resource-group state key.
    fn group_state_key(&self, group_key: &InMemoryStorageKey) -> Result<StateKey> {
        match group_key {
            InMemoryStorageKey::Group { address, group_ty } => {
                let tag =
                    struct_tag_of(*group_ty).ok_or_else(|| anyhow!("group type is not nominal"))?;
                Ok(StateKey::resource_group(address, &tag))
            },
            InMemoryStorageKey::Resource { .. } | InMemoryStorageKey::TableItem { .. } => {
                bail!("group_members expects a group slot key")
            },
        }
    }

    /// The state key of an own-slot value: a resource under its struct tag, or
    /// a table item under its handle. A group slot has no own storage slot.
    fn own_slot_state_key(&self, key: &InMemoryStorageKey) -> Result<StateKey> {
        match key {
            InMemoryStorageKey::Resource { address, ty } => {
                let tag =
                    struct_tag_of(*ty).ok_or_else(|| anyhow!("resource type is not nominal"))?;
                StateKey::resource(address, &tag).map_err(|e| anyhow!("bad state key: {e}"))
            },
            InMemoryStorageKey::TableItem { handle, key, .. } => {
                Ok(StateKey::table_item(&TableHandle(handle.address()), key))
            },
            InMemoryStorageKey::Group { .. } => bail!("a group slot has no own storage slot"),
        }
    }
}

/// A base value in the map: a pointer into the map's arena, needing no pin.
/// `Modification` is inert here (base values never enter a write set).
fn base_value(ptr: NonNull<u8>) -> MonoValue {
    MonoValue::Write {
        ptr,
        kind: WriteOpKind::Modification,
        pin: None,
    }
}

impl<S: TStateView<Key = StateKey>> ResourceProvider for BlockSTMProvider<'_, '_, S> {
    // No read capture this milestone: reads are not validated because the mono
    // path runs only sequentially, so `version` stays 0.
    // TODO(completeness): capture reads and their versions for parallel
    // validation.
    fn get_resource(
        &self,
        key: &InMemoryStorageKey,
        group: Option<InternedType>,
    ) -> Result<StorageRead, ResourceProviderError> {
        let internal = ResourceProviderError::InvariantViolation;
        // A value already in the map wins over the base view: an earlier
        // transaction's write, or a base value an earlier read cached.
        if let Some(value) = self.unsync_map.fetch_data(key) {
            return Ok(match value {
                MonoValue::Write { ptr, .. } => StorageRead::ExternalHeap { ptr, version: 0 },
                MonoValue::Deletion => StorageRead::DoesNotExist,
            });
        }

        match group {
            None => {
                let state_key = self
                    .own_slot_state_key(key)
                    .map_err(|e| internal(format!("{e:#}")))?;
                match self
                    .base_view
                    .get_state_value(&state_key)
                    .map_err(|e| internal(format!("state read failed: {e}")))?
                {
                    Some(value) => self.cache_base(key, value.bytes()),
                    None => Ok(StorageRead::DoesNotExist),
                }
            },
            Some(group_ty) => {
                // Serve just this member from the group's stored blob and cache
                // it in the map. A member created by an earlier transaction is
                // already in the map and served above; one absent from the base
                // blob does not exist.
                let group_key = InMemoryStorageKey::group(key.address(), group_ty);
                let group_state_key = self
                    .group_state_key(&group_key)
                    .map_err(|e| internal(format!("{e:#}")))?;
                let member = key.value_ty();
                let base = self
                    .base_group(&group_state_key)
                    .map_err(|e| internal(format!("{e:#}")))?;
                match base
                    .as_ref()
                    .and_then(|members| members.get(&member).cloned())
                {
                    Some(bytes) => self.cache_base(key, &bytes),
                    None => Ok(StorageRead::DoesNotExist),
                }
            },
        }
    }
}

impl<S: TStateView<Key = StateKey>> AptosDataProvider for BlockSTMProvider<'_, '_, S> {
    /// Reassembles a group's members: the stored blob overlaid with every member
    /// the block has written so far (a write replaces, a deletion removes).
    ///
    /// This reads the map, so it depends on the sequential loop's
    /// per-transaction interleaving (execute -> apply -> materialize): at
    /// materialization the map holds this transaction's writes but not any later
    /// transaction's, so the overlay is exactly the writes through this
    /// transaction. Batched materialization would break this.
    fn group_members(&self, group_key: &InMemoryStorageKey) -> Result<Option<Arc<GroupMembers>>> {
        let group_state_key = self.group_state_key(group_key)?;
        let mut members: GroupMembers = self
            .base_group(&group_state_key)?
            .as_ref()
            .map(|base| (**base).clone())
            .unwrap_or_default();

        let address = group_key.address();
        for tag in self.unsync_map.group_member_tags(group_key) {
            let member_ty = intern_struct_tag(&tag, self.guard)?;
            let member_key = InMemoryStorageKey::resource(address, member_ty);
            match self.unsync_map.fetch_data(&member_key) {
                Some(MonoValue::Write { ptr, .. }) => {
                    // SAFETY: the pointer addresses a live value in the map's
                    // arena or a session heap pinned by the map entry; no GC
                    // runs during materialization.
                    let bytes = unsafe { serialize_value(self.guard, ptr, member_ty) }
                        .map_err(|e| anyhow!("failed to serialize a group member: {e}"))?;
                    members.insert(member_ty, Bytes::from(bytes));
                },
                Some(MonoValue::Deletion) => {
                    members.remove(&member_ty);
                },
                // Recorded as written but absent from the map: cannot happen,
                // but if it did, leaving the base member is the safe choice.
                None => {},
            }
        }

        if members.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Arc::new(members)))
        }
    }

    /// Records a member type in the map's reverse index so a later
    /// [`group_members`](Self::group_members) reassembly overlays it.
    fn note_group_member(
        &self,
        group_key: &InMemoryStorageKey,
        member: InternedType,
    ) -> Result<()> {
        let tag =
            struct_tag_of(member).ok_or_else(|| anyhow!("group member type is not nominal"))?;
        self.unsync_map.record_group_member(group_key.clone(), tag);
        Ok(())
    }
}
