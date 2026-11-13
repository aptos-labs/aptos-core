// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The resource providers backing MonoMove execution on Block-STM: one over the
//! unsync map for sequential execution, one over the multi-version map for
//! parallel execution. Both are created once per transaction.

use crate::{
    mono_move::{
        group_chain::{GroupMetadata, GroupView},
        member_tag, tag_bytes, MonoValue,
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
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueMetadata},
        TStateView,
    },
    write_set::WriteOpKind,
};
use mono_move_aptos_transaction_executor::decode_group_members;
use mono_move_core::{
    storage::resource_provider::{
        InMemoryStorageKey, ResourceProvider, ResourceProviderError, StorageRead, Version,
    },
    types::InternedType,
    GroupSlot,
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

    /// Converts stored value into MonoMove memory representation and records
    /// it in the cache.
    fn cache_existing_value(
        &self,
        key: &InMemoryStorageKey,
        stored: &StateValue,
    ) -> Result<StorageRead, ResourceProviderError> {
        let value = deserialize_value(
            self.guard,
            self.arena,
            key.value_ty(),
            stored.bytes(),
            stored.metadata().clone(),
        )?;
        let read = value.as_storage_read(None)?;
        self.unsync_map.set_base_value(key.clone(), value);
        Ok(read)
    }

    /// Records non existing entry in the cache.
    fn cache_not_existing_value(&self, key: &InMemoryStorageKey) -> StorageRead {
        self.unsync_map
            .set_base_value(key.clone(), MonoValue::Deletion);
        StorageRead::DoesNotExist { version: None }
    }

    /// Reads one member of a resource group. `key` names the member's own slot,
    /// which is where the map holds it.
    ///
    /// A member no transaction wrote in this block is looked up in the view at
    /// the group's slot. That is the metadata of the latest materialized
    /// writer, or of the stored group, so its view is the group as this
    /// transaction sees it.
    fn read_group_member(
        &self,
        key: &InMemoryStorageKey,
        group_key: &InMemoryStorageKey,
    ) -> Result<StorageRead, ResourceProviderError> {
        if let Some(value) = self.unsync_map.fetch_data(key) {
            return value.as_storage_read(None);
        }
        let view = self
            .group_metadata(group_key)?
            .current()
            .ok_or_else(|| invariant_violation("The group slot's metadata has no view"))?;
        let value = get_group_member(self.guard, self.arena, &view, key)?;
        let read = value.as_storage_read(None)?;
        self.unsync_map.set_base_value(key.clone(), value);
        Ok(read)
    }

    /// The latest metadata at the group's slot: that of the latest
    /// materialized writer, or of the stored group, which is read on the
    /// block's first access to the group.
    fn group_metadata(
        &self,
        group_key: &InMemoryStorageKey,
    ) -> Result<Arc<GroupMetadata<InMemoryStorageKey>>, ResourceProviderError> {
        match self.unsync_map.fetch_data(group_key) {
            Some(MonoValue::GroupMetadata { group: metadata }) => Ok(metadata),
            Some(MonoValue::Write { .. } | MonoValue::Deletion) => Err(invariant_violation(
                "A group slot holds a value that is not group metadata",
            )),
            None => {
                let (view, slot_metadata) = get_pre_block_group(self.base_view, group_key)?;
                let metadata = Arc::new(GroupMetadata::from_storage(view, slot_metadata));
                self.unsync_map
                    .set_base_value(group_key.clone(), MonoValue::GroupMetadata {
                        group: metadata.clone(),
                    });
                Ok(metadata)
            },
        }
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
                    return value.as_storage_read(None);
                }

                // Otherwise, need to fetch from base view.
                let state_key = key
                    .as_state_key()
                    .map_err(|e| invariant_violation(format!("{e:#}")))?;
                match self.base_view.get_state_value(&state_key).map_err(|_| {
                    // TODO(completeness): this is not an invariant violation?
                    invariant_violation("Storage error")
                })? {
                    Some(value) => self.cache_existing_value(key, &value),
                    None => Ok(self.cache_not_existing_value(key)),
                }
            },
            Some(group_ty) => {
                let group_key = InMemoryStorageKey::resource_group(key.address(), group_ty);
                self.read_group_member(key, &group_key)
            },
        }
    }

    fn get_resource_group_slot(
        &self,
        group_key: &InMemoryStorageKey,
    ) -> Result<GroupSlot, ResourceProviderError> {
        Ok(self.group_metadata(group_key)?.slot().clone())
    }
}

/// Deserializes a stored blob into the in-memory value for a slot of type
/// `ty` with the given metadata.
fn deserialize_value(
    guard: &ExecutionGuard<'_>,
    arena: &SegmentedArena,
    ty: InternedType,
    blob: &[u8],
    metadata: StateValueMetadata,
) -> Result<MonoValue, ResourceProviderError> {
    let (ptr, pin) = arena.deserialize(guard, ty, blob)?;
    Ok(MonoValue::Write {
        ptr,
        bcs_size: blob.len(),
        metadata,
        // A value that is in storage exists, so reading it back is a
        // modification; an empty slot is stored as `MonoValue::Deletion`.
        kind: WriteOpKind::Modification,
        pin,
    })
}

/// The value stored at `key` before the block, or [`None`] for an empty slot.
fn get_pre_block_value(
    base_view: &impl TStateView<Key = StateKey>,
    key: &InMemoryStorageKey,
) -> Result<Option<StateValue>, ResourceProviderError> {
    let state_key = key
        .as_state_key()
        .map_err(|e| invariant_violation(format!("{e:#}")))?;
    base_view
        .get_state_value(&state_key)
        .map_err(|e| invariant_violation(format!("Storage read failed: {e}")))
}

/// The members of the group stored at `group_key` before the block, and the
/// metadata of its slot. No members and [`None`] if no group is stored there.
fn get_pre_block_group(
    base_view: &impl TStateView<Key = StateKey>,
    group_key: &InMemoryStorageKey,
) -> Result<(GroupView, Option<StateValueMetadata>), ResourceProviderError> {
    match get_pre_block_value(base_view, group_key)? {
        Some(stored) => Ok((
            decode_group_view(stored.bytes())?,
            Some(stored.into_metadata()),
        )),
        None => Ok((GroupView::default(), None)),
    }
}

/// The value of the member at `key` in the group `view`, deserialized into
/// `arena`, or a deletion if the group has no such member.
fn get_group_member(
    guard: &ExecutionGuard<'_>,
    arena: &SegmentedArena,
    view: &GroupView,
    key: &InMemoryStorageKey,
) -> Result<MonoValue, ResourceProviderError> {
    let tag = member_tag(key.value_ty()).map_err(invariant_violation)?;
    match view.get(&tag) {
        // A member has no slot of its own, so it has no metadata.
        Some(blob) => deserialize_value(
            guard,
            arena,
            key.value_ty(),
            blob,
            StateValueMetadata::none(),
        ),
        None => Ok(MonoValue::Deletion),
    }
}

/// Decodes a stored group's blob into its view.
fn decode_group_view(blob: &[u8]) -> Result<GroupView, ResourceProviderError> {
    let members = decode_group_members(blob)?
        .into_iter()
        .map(|(tag, bytes)| Ok((tag_bytes(&tag).map_err(invariant_violation)?, bytes)))
        .collect::<Result<Vec<_>, ResourceProviderError>>()?;
    Ok(GroupView::new(members))
}

fn invariant_violation(msg: impl ToString) -> ResourceProviderError {
    ResourceProviderError::InvariantViolation(msg.to_string())
}

/// Reads the multi-version map on behalf of one incarnation of one transaction,
/// waiting out the dependencies it runs into.
pub(crate) struct ParallelReader<'a, K> {
    versioned_map: &'a MVHashMap<K, (), MonoValue<K>, DelayedFieldID>,
    scheduler: SchedulerWrapper<'a>,
    txn_idx: TxnIndex,
    incarnation: Incarnation,
    /// Set when a read could not be served because this transaction is doomed:
    /// it depends on a write that is still speculative, or the block halted.
    speculative_failure: Cell<bool>,
}

impl<'a, K: Hash + Clone + Eq + Debug + Send + Sync> ParallelReader<'a, K> {
    pub(crate) fn new(
        versioned_map: &'a MVHashMap<K, (), MonoValue<K>, DelayedFieldID>,
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

    /// The entry the data map serves this transaction at `key`.
    fn fetch(&self, key: &K) -> Result<MVDataOutput<MonoValue<K>>, MVDataError> {
        let data = self.versioned_map.data();
        if self.scheduler.is_v2() {
            data.fetch_data_and_record_dependency(key, self.txn_idx, self.incarnation)
        } else {
            data.fetch_data_no_record(key, self.txn_idx)
        }
    }

    /// The slot of the group at `group_key` as this transaction sees it: the
    /// slot the latest metadata below it at the group's slot holds. Initializes
    /// the group's slot with `init`, which returns the stored group's view and
    /// the metadata of its slot, on the block's first read of the group.
    pub(crate) fn read_group_slot(
        &self,
        group_key: &K,
        init: impl Fn() -> Result<(GroupView, Option<StateValueMetadata>), ResourceProviderError>,
    ) -> Result<GroupSlot, ResourceProviderError> {
        loop {
            match self.fetch(group_key) {
                Ok(MVDataOutput::Versioned(_, MonoValue::GroupMetadata { group: metadata })) => {
                    return Ok(metadata.slot().clone())
                },
                Ok(MVDataOutput::Versioned(_, MonoValue::Write { .. } | MonoValue::Deletion)) => {
                    return Err(invariant_violation(
                        "A group slot holds a value that is not group metadata",
                    ))
                },
                Err(MVDataError::Uninitialized) => {
                    let (view, slot_metadata) = init()?;
                    let metadata = GroupMetadata::from_storage(view, slot_metadata);
                    // Another worker may be initializing the same group from the
                    // same storage, so whichever metadata is already there stands.
                    self.versioned_map.data().set_base_value(
                        group_key.clone(),
                        MonoValue::GroupMetadata {
                            group: Arc::new(metadata),
                        },
                        |_, _| {},
                    );
                },
                Err(MVDataError::Dependency(dep_idx)) => self.wait(dep_idx)?,
            }
        }
    }

    /// Reads the data map at `key`, initializing it with `init` if this is the
    /// block's first read of it.
    pub(crate) fn read_or_initialize(
        &self,
        key: &K,
        init: impl Fn() -> Result<MonoValue<K>, ResourceProviderError>,
    ) -> Result<StorageRead, ResourceProviderError> {
        let data = self.versioned_map.data();
        loop {
            match self.fetch(key) {
                Ok(MVDataOutput::Versioned(version, value)) => {
                    return value.as_storage_read(to_mono_version(version))
                },
                Err(MVDataError::Uninitialized) => {
                    let value = init()?;
                    // Another worker may be initializing the same slot from the
                    // same storage, so whichever value is already there stands.
                    data.set_base_value(key.clone(), value, |_, _| {});
                },
                Err(MVDataError::Dependency(dep_idx)) => self.wait(dep_idx)?,
            }
        }
    }

    /// The stored group's view, initializing the group's slot with `init` on
    /// the block's first read of the group. `init` returns the stored group's
    /// view and the metadata of its slot. Reading the slot at index 0 sees
    /// only its base entry, never a transaction's metadata, so this records
    /// nothing.
    pub(crate) fn get_or_initialize_pre_block_group(
        &self,
        group_key: &K,
        init: impl Fn() -> Result<(GroupView, Option<StateValueMetadata>), ResourceProviderError>,
    ) -> Result<Arc<GroupView>, ResourceProviderError> {
        let data = self.versioned_map.data();
        loop {
            match data.fetch_data_no_record(group_key, 0) {
                Ok(MVDataOutput::Versioned(_, MonoValue::GroupMetadata { group: metadata })) => {
                    return metadata.current().ok_or_else(|| {
                        invariant_violation("The stored group's metadata has no view")
                    });
                },
                Ok(MVDataOutput::Versioned(_, MonoValue::Write { .. } | MonoValue::Deletion)) => {
                    return Err(invariant_violation(
                        "A group slot's base entry is not group metadata",
                    ))
                },
                Err(MVDataError::Uninitialized) => {
                    let (view, slot_metadata) = init()?;
                    let metadata = GroupMetadata::from_storage(view, slot_metadata);
                    // Another worker may be initializing the same group from the
                    // same storage, so whichever metadata is already there stands.
                    data.set_base_value(
                        group_key.clone(),
                        MonoValue::GroupMetadata {
                            group: Arc::new(metadata),
                        },
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
fn to_mono_version(version: MVVersion) -> Version {
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

    /// The value stored at `key` before the block, or a deletion for an empty
    /// slot.
    fn get_pre_block_resource(
        &self,
        key: &InMemoryStorageKey,
    ) -> Result<MonoValue, ResourceProviderError> {
        match get_pre_block_value(self.base_view, key)? {
            Some(stored) => deserialize_value(
                self.guard,
                self.arena,
                key.value_ty(),
                stored.bytes(),
                stored.metadata().clone(),
            ),
            None => Ok(MonoValue::Deletion),
        }
    }
}

impl<S: TStateView<Key = StateKey>> ResourceProvider for BlockSTMParallelProvider<'_, '_, S> {
    fn get_resource(
        &self,
        key: &InMemoryStorageKey,
        group: Option<InternedType>,
    ) -> Result<StorageRead, ResourceProviderError> {
        match group {
            None => self
                .reader
                .read_or_initialize(key, || self.get_pre_block_resource(key)),
            Some(group_ty) => {
                let group_key = InMemoryStorageKey::resource_group(key.address(), group_ty);
                self.reader.read_or_initialize(key, || {
                    let group = self
                        .reader
                        .get_or_initialize_pre_block_group(&group_key, || {
                            get_pre_block_group(self.base_view, &group_key)
                        })?;
                    get_group_member(self.guard, self.arena, &group, key)
                })
            },
        }
    }

    fn get_resource_group_slot(
        &self,
        group_key: &InMemoryStorageKey,
    ) -> Result<GroupSlot, ResourceProviderError> {
        self.reader
            .read_group_slot(group_key, || get_pre_block_group(self.base_view, group_key))
    }
}
