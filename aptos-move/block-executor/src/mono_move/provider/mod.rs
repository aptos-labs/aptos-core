// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The resource providers backing MonoMove execution on Block-STM: one over the
//! unsync map for sequential execution, one over the multi-version map for
//! parallel execution. Both are created once per transaction.
//!
//! The speculative read loops live here rather than in the parallel provider so
//! that the combinatorial tests can drive them with their own key and tag types.

mod parallel;
mod sequential;

use crate::{mono_move::MonoValue, scheduler_wrapper::SchedulerWrapper, view::wait_for_dependency};
use aptos_mvhashmap::{
    types::{Incarnation, MVDataError, MVDataOutput, MVGroupError, TxnIndex, Version as MVVersion},
    MVHashMap,
};
use aptos_types::write_set::WriteOpKind;
use bytes::Bytes;
use mono_move_core::{
    storage::resource_provider::{ResourceProviderError, StorageRead, Version},
    types::InternedType,
    LayoutProvider, VMInternalError, OBJECT_HEADER_SIZE,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{deserialize_into, Heap, RuntimeError, SegmentedArena, SharedArena};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
pub(crate) use parallel::BlockSTMParallelProvider;
pub(crate) use sequential::BlockSTMSequentialProvider;
use serde::Serialize;
use std::{cell::Cell, fmt::Debug, hash::Hash, ptr::NonNull, sync::Arc};

/// Converts BCS bytes into MonoMove's memory representation, storing the result
/// in `arena`. Returns the value and the allocation to pin it by.
pub(crate) fn deserialize_into_arena(
    guard: &ExecutionGuard<'_>,
    arena: &SegmentedArena,
    ty: InternedType,
    blob: &[u8],
) -> Result<(NonNull<u8>, Arc<SharedArena>), ResourceProviderError> {
    let layout = guard
        .layout_by_ty(ty)
        .ok_or_else(|| invariant_violation("No layout found when deserializing base value"))?;
    let descriptor = guard.struct_descriptor(ty).ok_or_else(|| {
        invariant_violation("No GC descriptor found when deserializing base value")
    })?;
    let total_size = OBJECT_HEADER_SIZE + layout.size as usize;

    // The flat root and everything nested under it must land in one segment, so
    // they are allocated together and retried together.
    let (result, segment) = arena
        .alloc_in(|segment| {
            segment.with_heap_mut(|heap: &mut Heap| {
                let obj = heap.alloc_object(total_size, descriptor)?;
                // SAFETY: `obj` is a freshly reserved object sized for the
                // value's layout; `deserialize_into` writes the flat value
                // there and puts the nested allocations in the same heap.
                match unsafe { deserialize_into(guard, heap, ty, blob, obj.as_ptr()) } {
                    Ok(()) => Some(Ok(obj)),
                    // Only running out of room is worth a larger segment.
                    Err(e) if is_out_of_heap_memory(&e) => None,
                    Err(e) => Some(Err(e)),
                }
            })
        })
        .ok_or_else(|| invariant_violation("Storage value does not fit in an arena segment"))?;

    let obj = result
        .map_err(|e| invariant_violation(format!("Storage value failed to deserialize: {e}")))?;
    Ok((obj, segment))
}

/// Materializes a stored blob into the in-memory value for a slot of type `ty`.
pub(crate) fn materialized_value(
    guard: &ExecutionGuard<'_>,
    arena: &SegmentedArena,
    ty: InternedType,
    blob: &[u8],
) -> Result<MonoValue, ResourceProviderError> {
    let (ptr, pin) = deserialize_into_arena(guard, arena, ty, blob)?;
    Ok(MonoValue::Write {
        ptr,
        ty,
        // A value that is in storage exists, so reading it back is a
        // modification; an empty slot is stored as `MonoValue::Deletion`.
        kind: WriteOpKind::Modification,
        pin,
    })
}

/// Serves a read the value the multi-version map holds at its slot.
fn storage_read(value: MonoValue, version: Version) -> Result<StorageRead, ResourceProviderError> {
    Ok(match value {
        MonoValue::Write { ptr, pin, .. } => StorageRead::ExternalHeap { ptr, version, pin },
        MonoValue::Deletion => StorageRead::DoesNotExist { version },
        // Both are upgraded or filtered out before a read gets here: stored
        // group members become writes, and a group's own slot is never read.
        MonoValue::RawFromStorage(_) => {
            return Err(invariant_violation(
                "Undecoded storage bytes served as a read",
            ))
        },
        MonoValue::GroupMetadata => {
            return Err(invariant_violation("Group metadata served as a read"))
        },
    })
}

/// Whether the value ran out of room, which a larger segment can fix.
fn is_out_of_heap_memory(e: &VMInternalError) -> bool {
    matches!(
        e.downcast_ref::<RuntimeError>(),
        Some(RuntimeError::OutOfHeapMemory { .. })
    )
}

pub(crate) fn invariant_violation(msg: impl ToString) -> ResourceProviderError {
    ResourceProviderError::InvariantViolation(msg.to_string())
}

/// Pre-block state, served by whichever provider owns the connection to it.
/// The speculative read loops call back into this when the multi-version map
/// has nothing at a slot yet.
pub(crate) trait StorageBase<K, T> {
    /// The value stored at `key` before the block, already materialized, or
    /// [`None`] if the slot is empty.
    fn resource(&self, key: &K) -> Result<Option<MonoValue>, ResourceProviderError>;

    /// The members of the group stored at `group_key` before the block, in the
    /// encoding storage holds them in.
    fn group_members(&self, group_key: &K) -> Result<Vec<(T, Bytes)>, ResourceProviderError>;

    /// Materializes a stored group member, whose type `key` names.
    fn materialize_member(&self, key: &K, blob: &Bytes)
        -> Result<MonoValue, ResourceProviderError>;
}

/// Reads the multi-version map on behalf of one incarnation of one transaction,
/// waiting out the dependencies it runs into.
pub(crate) struct ParallelReader<'a, K, T> {
    versioned_map: &'a MVHashMap<K, T, MonoValue, DelayedFieldID>,
    scheduler: SchedulerWrapper<'a>,
    txn_idx: TxnIndex,
    incarnation: Incarnation,
    /// Set when a read could not be served because this transaction is doomed:
    /// it depends on a write that is still speculative, or the block halted.
    speculative_failure: Cell<bool>,
}

impl<'a, K, T> ParallelReader<'a, K, T>
where
    K: Hash + Clone + Eq + Debug,
    T: Hash + Clone + Eq + Debug + Serialize,
{
    pub(crate) fn new(
        versioned_map: &'a MVHashMap<K, T, MonoValue, DelayedFieldID>,
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

    pub(crate) fn versioned_map(&self) -> &'a MVHashMap<K, T, MonoValue, DelayedFieldID> {
        self.versioned_map
    }

    pub(crate) fn txn_idx(&self) -> TxnIndex {
        self.txn_idx
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
        base: &dyn StorageBase<K, T>,
        key: &K,
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
                    return storage_read(value, mono_version(version))
                },
                Err(MVDataError::Uninitialized) => {
                    let value = base.resource(key)?.unwrap_or(MonoValue::Deletion);
                    // Another worker may be provisioning the same slot from the
                    // same storage, so whichever value is already there stands.
                    data.set_base_value(key.clone(), value, |_, _| {});
                },
                Err(MVDataError::Dependency(dep_idx)) => self.wait(dep_idx)?,
            }
        }
    }

    /// Reads one member of a resource group. `key` names the member's own slot,
    /// which is what gives its type; `group_key` and `tag` locate it in the map.
    pub(crate) fn read_group_member(
        &self,
        base: &dyn StorageBase<K, T>,
        key: &K,
        group_key: &K,
        tag: &T,
    ) -> Result<StorageRead, ResourceProviderError> {
        let group_data = self.versioned_map.group_data();
        loop {
            let read = if self.scheduler.is_v2() {
                group_data.fetch_tagged_data_and_record_dependency(
                    group_key,
                    tag,
                    self.txn_idx,
                    self.incarnation,
                )
            } else {
                group_data.fetch_tagged_data_no_record(group_key, tag, self.txn_idx)
            };
            match read {
                Ok((_, MonoValue::RawFromStorage(blob))) => {
                    // A stored group does not name its members' types, so the
                    // first reader that knows one replaces the bytes with the
                    // flat value. Re-read, as another worker may have won.
                    let value = base.materialize_member(key, &blob)?;
                    group_data.update_tagged_base_value_with_layout(
                        group_key.clone(),
                        tag.clone(),
                        value,
                        |prev, new| {
                            if matches!(prev, MonoValue::RawFromStorage(_)) {
                                *prev = new;
                            }
                        },
                    );
                },
                Ok((version, value)) => return storage_read(value, mono_version(version)),
                Err(MVGroupError::Uninitialized) => {
                    let members = base
                        .group_members(group_key)?
                        .into_iter()
                        .map(|(tag, blob)| (tag, MonoValue::RawFromStorage(blob)))
                        .collect();
                    // The whole group goes in at once: a group left half
                    // initialized cannot be repaired by a later read.
                    group_data
                        .set_raw_base_values(group_key.clone(), members)
                        .map_err(|e| {
                            invariant_violation(format!("Failed to set group base values: {e:#}"))
                        })?;
                },
                // The group is there but the member is not. Record that as a
                // deletion at the pre-block version and retry: BlockSTMv2 can
                // only track a read that has an entry to hang it off, so
                // returning without one would miss later writes to this tag.
                Err(MVGroupError::TagNotFound) => {
                    group_data.update_tagged_base_value_with_layout(
                        group_key.clone(),
                        tag.clone(),
                        MonoValue::Deletion,
                        |_, _| {},
                    );
                },
                Err(MVGroupError::Dependency(dep_idx)) => self.wait(dep_idx)?,
            }
        }
    }
}

/// Block-STM's version of a read, as MonoMove records it. The pre-block state
/// is [`None`] on both sides.
fn mono_version(version: MVVersion) -> Version {
    version.ok()
}
