// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The resource providers backing MonoMove execution on Block-STM: one over the
//! unsync map for sequential execution, one over the multi-version map for
//! parallel execution. Both are created once per transaction.
//!
//! The speculative read loops live in [`ParallelReader`] rather than in the
//! parallel provider so that the combinatorial tests can drive them with their
//! own key type.

use crate::{
    mono_move::{
        group_chain::{member_tag, tag_bytes, GroupLink, GroupView},
        MonoValue,
    },
    scheduler_wrapper::SchedulerWrapper,
    view::wait_for_dependency,
};
use aptos_mvhashmap::{
    types::{Incarnation, MVDataError, MVDataOutput, TxnIndex, Version as MVVersion},
    unsync_map::UnsyncMap,
    MVHashMap,
};
use aptos_types::{
    state_store::{state_key::StateKey, TStateView},
    write_set::WriteOpKind,
};
use bytes::Bytes;
use mono_move_aptos_transaction_executor::decode_group_members;
use mono_move_core::{
    storage::resource_provider::{
        InMemoryStorageKey, ResourceProvider, ResourceProviderError, StorageRead, Version,
    },
    types::InternedType,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::SegmentedArena;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{cell::Cell, fmt::Debug, hash::Hash, sync::Arc};

type MonoUnsyncMap = UnsyncMap<InMemoryStorageKey, (), MonoValue, DelayedFieldID>;

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

/// Materializes a stored blob into the in-memory value for a slot of type `ty`.
fn materialized_value(
    guard: &ExecutionGuard<'_>,
    arena: &SegmentedArena,
    ty: InternedType,
    blob: &[u8],
) -> Result<MonoValue, ResourceProviderError> {
    let (ptr, pin) = arena.deserialize(guard, ty, blob)?;
    Ok(MonoValue::Write {
        ptr,
        // A value that is in storage exists, so reading it back is a
        // modification; an empty slot is stored as `MonoValue::Deletion`.
        kind: WriteOpKind::Modification,
        pin,
    })
}

/// Serves a value one of the maps holds at a slot as a read at `version`.
/// Sequential execution does not speculate, so it passes [`None`].
fn storage_read(value: &MonoValue, version: Version) -> Result<StorageRead, ResourceProviderError> {
    Ok(match value {
        MonoValue::Write { ptr, pin, .. } => StorageRead::ExternalHeap {
            ptr: *ptr,
            version,
            pin: pin.clone(),
        },
        MonoValue::Deletion => StorageRead::DoesNotExist { version },
        // A group's own slot is only read at commit, never by a transaction.
        MonoValue::GroupLink(_) => {
            return Err(invariant_violation("A group link served as a read"))
        },
    })
}

/// Decodes a stored group's blob into its view.
fn decode_group_view(blob: &[u8]) -> Result<GroupView, ResourceProviderError> {
    decode_group_members(blob)?
        .into_iter()
        .map(|(tag, bytes)| Ok((tag_bytes(&tag).map_err(invariant_violation)?, bytes)))
        .collect()
}

fn invariant_violation(msg: impl ToString) -> ResourceProviderError {
    ResourceProviderError::InvariantViolation(msg.to_string())
}

/// Pre-block state, served by whichever provider owns the connection to it.
/// The speculative read loops call back into this when the multi-version map
/// has nothing at a slot yet.
pub(crate) trait StorageBase<K> {
    /// The value stored at `key` before the block, already materialized, or
    /// [`None`] if the slot is empty.
    fn resource(&self, key: &K) -> Result<Option<MonoValue>, ResourceProviderError>;

    /// The members of the group stored at `group_key` before the block, in the
    /// encoding storage holds them in. Empty if no group is stored there.
    fn group_view(&self, group_key: &K) -> Result<GroupView, ResourceProviderError>;

    /// Materializes a stored group member, whose type `key` names.
    fn materialize_member(&self, key: &K, blob: &Bytes)
        -> Result<MonoValue, ResourceProviderError>;
}

/// Reads the multi-version map on behalf of one incarnation of one transaction,
/// waiting out the dependencies it runs into.
pub(crate) struct ParallelReader<'a, K> {
    versioned_map: &'a MVHashMap<K, (), MonoValue, DelayedFieldID>,
    scheduler: SchedulerWrapper<'a>,
    txn_idx: TxnIndex,
    incarnation: Incarnation,
    /// Set when a read could not be served because this transaction is doomed:
    /// it depends on a write that is still speculative, or the block halted.
    speculative_failure: Cell<bool>,
}

impl<'a, K: Hash + Clone + Eq + Debug> ParallelReader<'a, K> {
    pub(crate) fn new(
        versioned_map: &'a MVHashMap<K, (), MonoValue, DelayedFieldID>,
        scheduler: SchedulerWrapper<'a>,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
    ) -> Self {
        Self {
            versioned_map,
            scheduler,
            txn_idx,
            incarnation,
            speculative_failure: Cell::new(false),
        }
    }

    /// Whether any read failed speculatively. Nothing this transaction produced
    /// may commit once this is set.
    pub(crate) fn speculative_failure(&self) -> bool {
        self.speculative_failure.get()
    }

    fn abort(&self, msg: impl ToString) -> ResourceProviderError {
        self.speculative_failure.set(true);
        ResourceProviderError::SpeculativeAbort(msg.to_string())
    }

    /// Blocks until `dep_idx` resolves, failing the read if the block halted
    /// first.
    fn wait(&self, dep_idx: TxnIndex) -> Result<(), ResourceProviderError> {
        match wait_for_dependency(&self.scheduler, self.txn_idx, dep_idx) {
            Ok(true) => Ok(()),
            Ok(false) => Err(self.abort("Block execution was halted")),
            Err(e) => Err(invariant_violation(format!(
                "Failed to wait for a dependency: {e:?}"
            ))),
        }
    }

    /// Reads the value in its own storage slot, provisioning the pre-block
    /// value first if this is the block's first read of it.
    pub(crate) fn read_resource(
        &self,
        base: &dyn StorageBase<K>,
        key: &K,
    ) -> Result<StorageRead, ResourceProviderError> {
        self.read_data(key, || {
            Ok(base.resource(key)?.unwrap_or(MonoValue::Deletion))
        })
    }

    /// Reads one member of a resource group. `key` names the member's own slot,
    /// which is where the multi-version map holds it; `group_key` and `tag`
    /// locate its pre-block value in the stored group.
    pub(crate) fn read_group_member(
        &self,
        base: &dyn StorageBase<K>,
        key: &K,
        group_key: &K,
        tag: impl Fn() -> Result<Bytes, ResourceProviderError>,
    ) -> Result<StorageRead, ResourceProviderError> {
        self.read_data(key, || {
            let view = self.stored_group_view(base, group_key)?;
            match view.get(&tag()?) {
                Some(blob) => base.materialize_member(key, blob),
                None => Ok(MonoValue::Deletion),
            }
        })
    }

    /// Reads the data map at `key`, provisioning it with `base_value` if this
    /// is the block's first read of it.
    fn read_data(
        &self,
        key: &K,
        base_value: impl Fn() -> Result<MonoValue, ResourceProviderError>,
    ) -> Result<StorageRead, ResourceProviderError> {
        let data = self.versioned_map.data();
        loop {
            let read = if self.scheduler.is_v2() {
                data.fetch_data_and_record_dependency(key, self.txn_idx, self.incarnation)
            } else {
                data.fetch_data_no_record(key, self.txn_idx)
            };
            match read {
                Ok(MVDataOutput::Versioned(version, value)) => {
                    return storage_read(&value, mono_version(version))
                },
                Err(MVDataError::Uninitialized) => {
                    let value = base_value()?;
                    // Another worker may be provisioning the same slot from the
                    // same storage, so whichever value is already there stands.
                    data.set_base_value(key.clone(), value, |_, _| {});
                },
                Err(MVDataError::Dependency(dep_idx)) => self.wait(dep_idx)?,
            }
        }
    }

    /// The stored group's view, provisioning the group's slot with it on the
    /// block's first read of the group. Reading the slot at index 0 sees only
    /// its base entry, never a transaction's link, so this records nothing.
    fn stored_group_view(
        &self,
        base: &dyn StorageBase<K>,
        group_key: &K,
    ) -> Result<Arc<GroupView>, ResourceProviderError> {
        let data = self.versioned_map.data();
        loop {
            match data.fetch_data_no_record(group_key, 0) {
                Ok(MVDataOutput::Versioned(_, MonoValue::GroupLink(link))) => {
                    return link
                        .built_view()
                        .cloned()
                        .ok_or_else(|| invariant_violation("The stored group's link has no view"));
                },
                Ok(MVDataOutput::Versioned(_, MonoValue::Write { .. } | MonoValue::Deletion)) => {
                    return Err(invariant_violation(
                        "A group slot's base entry is not a link",
                    ))
                },
                Err(MVDataError::Uninitialized) => {
                    let link = GroupLink::stored(base.group_view(group_key)?);
                    // Another worker may be provisioning the same group from the
                    // same storage, so whichever link is already there stands.
                    data.set_base_value(
                        group_key.clone(),
                        MonoValue::GroupLink(Arc::new(link)),
                        |_, _| {},
                    );
                },
                Err(MVDataError::Dependency(_)) => {
                    return Err(invariant_violation(
                        "A group slot's base entry has a dependency",
                    ))
                },
            }
        }
    }
}

/// Block-STM's version of a read, as MonoMove records it. The pre-block state
/// is [`None`] on both sides.
fn mono_version(version: MVVersion) -> Version {
    version.ok()
}

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
