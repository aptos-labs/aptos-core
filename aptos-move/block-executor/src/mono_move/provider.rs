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
//! and a member write lands in the map under its own key. The group blob itself
//! is not reassembled here. At materialization the transaction executor's
//! `drain_write_set` overlays the transaction's own member ops onto the stored
//! members returned by [`group_members`](BlockSTMProvider::group_members), which
//! reads the base view only.

use crate::mono_move::MonoValue;
use anyhow::{anyhow, Result};
use aptos_mvhashmap::unsync_map::{RunningGroup, UnsyncMap};
use aptos_types::{
    state_store::{state_key::StateKey, TStateView},
    write_set::{WriteOp, WriteOpKind},
};
use bytes::Bytes;
use mono_move_aptos_transaction_executor::{
    decode_group_members, finalize_group, AptosDataProvider, GroupMembers,
};
use mono_move_core::{
    storage::resource_provider::{
        nominal_tag, InMemoryStorageKey, ResourceProvider, ResourceProviderError, StorageRead,
    },
    types::InternedType,
    LayoutProvider, OBJECT_HEADER_SIZE,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::deserialize_into;
use move_core_types::language_storage::StructTag;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{
    collections::{BTreeMap, HashMap},
    ptr::NonNull,
};
use triomphe::Arc;

/// The sequential Block-STM map keyed/valued for MonoMove.
type MonoUnsyncMap = UnsyncMap<InMemoryStorageKey, StructTag, MonoValue, DelayedFieldID>;

/// Resource provider reading through the sequential multi-version map, then the
/// base state view.
pub(crate) struct BlockSTMProvider<'a, 'ctx, S> {
    guard: &'a ExecutionGuard<'ctx>,
    base_view: &'a S,
    unsync_map: &'a MonoUnsyncMap,
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
        }
    }

    /// Materializes `blob` (BCS -> flat) into the map's append-only resource
    /// arena as the value of `key`, returning a pointer into the arena. The
    /// arena is never garbage-collected, so the pointer stays valid for the
    /// lifetime of the map.
    pub(super) fn materialize_into_arena(
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
    /// view and cached (absence included) in the shared map for the rest of the
    /// block, keyed by the group's state key.
    fn base_group(&self, group_state_key: &StateKey) -> Result<Option<Arc<GroupMembers>>> {
        if let Some(members) = self.unsync_map.group_base_cached(group_state_key) {
            return Ok(members);
        }
        let members = match self
            .base_view
            .get_state_value(group_state_key)
            .map_err(|e| anyhow!("group read failed: {e}"))?
        {
            Some(value) => Some(Arc::new(decode_group_members(value.bytes(), self.guard)?)),
            None => None,
        };
        self.unsync_map
            .cache_group_base(group_state_key.clone(), members.clone());
        Ok(members)
    }

    /// Seeds the running merged state of the group at `group_key` from its stored
    /// members: the canonical struct-tag map and whether the slot exists in
    /// storage. Called once per group, on the first write; later writes mutate
    /// the seeded state in place.
    fn seed_running_group(&self, group_key: &StateKey) -> Result<RunningGroup> {
        let base = self.base_group(group_key)?;
        let mut members = BTreeMap::new();
        for (ty, bytes) in base.iter().flat_map(|group| group.iter()) {
            members.insert(nominal_tag(*ty)?, bytes.clone());
        }
        Ok(RunningGroup {
            members,
            exists: base.is_some(),
        })
    }
}

/// A base value in the map: a pointer into the map's own arena, so it needs no
/// heap anchor. `Modification` is inert here (base values never enter a write
/// set).
fn base_value(ptr: NonNull<u8>) -> MonoValue {
    MonoValue::Write {
        ptr,
        kind: WriteOpKind::Modification,
        heap: None,
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
                let state_key = key.as_state_key().map_err(|e| internal(format!("{e:#}")))?;
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
                let tag = nominal_tag(group_ty).map_err(|e| internal(format!("{e:#}")))?;
                let address = key.address();
                let group_state_key = StateKey::resource_group(&address, &tag);
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
    /// The stored (pre-block) members of a resource group, read from the base
    /// view only. Serves member reads in `get_resource`; group writes go through
    /// [`group_write_op`](BlockSTMProvider::group_write_op), which tracks
    /// membership across transactions.
    fn group_members(&self, group_key: &StateKey) -> Result<Option<Arc<GroupMembers>>> {
        self.base_group(group_key)
    }

    /// Overlays this transaction's member ops onto the group's running merged
    /// state and emits the slot's write op. The running state is seeded from
    /// storage on the first write of the block and then mutated in place, so it
    /// accumulates every earlier transaction's member writes. That fixes the
    /// stored-only limitation of the default: a member another transaction added
    /// earlier in the block is present when a later transaction touches the group.
    //
    // TODO(completeness): the running state is mutated in place, which is sound
    // only under in-order sequential commit. Parallel execution must derive the
    // group's state functionally at the transaction's read version from the
    // versioned group data instead.
    fn group_write_op(
        &self,
        group_key: &StateKey,
        member_ops: HashMap<StructTag, Option<Bytes>>,
    ) -> Result<WriteOp> {
        self.unsync_map.with_group_running_mut(
            group_key,
            || self.seed_running_group(group_key),
            |running| finalize_group(&mut running.members, &mut running.exists, member_ops),
        )
    }
}
