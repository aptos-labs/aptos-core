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

use crate::{
    mono_move::{
        group_chain::{tag_bytes, GroupLink, GroupView},
        MonoValue,
    },
    scheduler_wrapper::SchedulerWrapper,
    view::wait_for_dependency,
};
use aptos_mvhashmap::{
    types::{Incarnation, MVDataError, MVDataOutput, TxnIndex, Version as MVVersion},
    MVHashMap,
};
use aptos_types::write_set::WriteOpKind;
use bytes::Bytes;
use mono_move_aptos_transaction_executor::decode_group_members;
use mono_move_core::{
    storage::resource_provider::{ResourceProviderError, StorageRead, Version},
    types::InternedType,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::SegmentedArena;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
pub(crate) use parallel::BlockSTMParallelProvider;
pub(crate) use sequential::BlockSTMSequentialProvider;
use serde::Serialize;
use std::{cell::Cell, fmt::Debug, hash::Hash, sync::Arc};

/// Materializes a stored blob into the in-memory value for a slot of type `ty`.
pub(crate) fn materialized_value(
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
pub(crate) fn storage_read(
    value: &MonoValue,
    version: Version,
) -> Result<StorageRead, ResourceProviderError> {
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
pub(crate) fn decode_group_view(blob: &[u8]) -> Result<GroupView, ResourceProviderError> {
    decode_group_members(blob)?
        .into_iter()
        .map(|(tag, bytes)| Ok((tag_bytes(&tag).map_err(invariant_violation)?, bytes)))
        .collect()
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
    /// encoding storage holds them in. Empty if no group is stored there.
    fn group_view(&self, group_key: &K) -> Result<GroupView, ResourceProviderError>;

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
        self.read_data(key, || {
            Ok(base.resource(key)?.unwrap_or(MonoValue::Deletion))
        })
    }

    /// Reads one member of a resource group. `key` names the member's own slot,
    /// which is where the multi-version map holds it; `group_key` and `tag`
    /// locate its pre-block value in the stored group.
    pub(crate) fn read_group_member(
        &self,
        base: &dyn StorageBase<K, T>,
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
        base: &dyn StorageBase<K, T>,
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
