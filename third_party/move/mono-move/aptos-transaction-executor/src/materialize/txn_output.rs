// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Creates a `TransactionOutput` from a transaction's side effects.

use crate::{errors::MaterializationError, providers::AptosDataProvider};
use aptos_types::{
    contract_event::ContractEvent,
    state_store::state_key::StateKey,
    transaction::{
        ExecutionStatus, TransactionAuxiliaryData, TransactionOutput, TransactionStatus,
    },
    write_set::{WriteOp, WriteSet},
};
use bytes::Bytes;
use mono_move_core::{
    nominal_tag, storage::resource_provider::InMemoryStorageKey, types::InternedType,
    value_layout::LayoutProvider,
};
use mono_move_output::to_contract_events;
use mono_move_runtime::{serialize, GroupEntry, SessionEffects, WriteClass};
use move_core_types::{language_storage::StructTag, vm_status::StatusCode};
use shared_dsa::UnorderedMap;
use std::{collections::BTreeMap, ptr::NonNull};

// TODO(security): audit all error messages and make sure they do not lead to
// deep recursions or OOMs.

// TODO(security): pin providers so that execution and materialization use
// identical layout and data providers.

/// A transaction's output without its resource-group writes. The caller
/// completes it with them.
pub struct TransactionOutputWithoutGroups {
    writes: Vec<(StateKey, WriteOp)>,
    events: Vec<ContractEvent>,
    gas_used: u64,
    status: TransactionStatus,
    auxiliary_data: TransactionAuxiliaryData,
}

impl TransactionOutputWithoutGroups {
    /// Completes the output with one write per resource group the transaction
    /// changed.
    pub fn with_group_writes(
        self,
        group_writes: impl IntoIterator<Item = (StateKey, WriteOp)>,
    ) -> Result<TransactionOutput, MaterializationError> {
        // `WriteSet::new` collects into a `BTreeMap`, so the order of the writes
        // does not reach the result.
        let write_set =
            WriteSet::new(self.writes.into_iter().chain(group_writes)).map_err(|e| {
                MaterializationError::new(vec![format!("failed to build the write set: {e:?}")])
            })?;
        Ok(TransactionOutput::new(
            write_set,
            self.events,
            self.gas_used,
            self.status,
            self.auxiliary_data,
        ))
    }
}

/// Creates the output of an executed transaction from its effects, except for
/// its resource groups.
pub(crate) fn executed_output(
    effects: &SessionEffects,
    layouts: &impl LayoutProvider,
    gas_used: u64,
    status: TransactionStatus,
    auxiliary_data: TransactionAuxiliaryData,
) -> Result<TransactionOutputWithoutGroups, MaterializationError> {
    let writes = resource_writes(effects, layouts).map_err(MaterializationError::new)?;
    let events = to_contract_events(effects, layouts)
        .map_err(|e| MaterializationError::new(vec![format!("event finalization failed: {e}")]))?;
    Ok(TransactionOutputWithoutGroups {
        writes,
        events,
        gas_used,
        status,
        auxiliary_data,
    })
}

/// Creates the output of a discarded transaction: empty, carrying the reason.
pub(crate) fn discarded_output(
    status_code: StatusCode,
    auxiliary_data: TransactionAuxiliaryData,
) -> TransactionOutputWithoutGroups {
    empty_output(TransactionStatus::Discard(status_code), auxiliary_data)
}

/// Creates the committed empty output: successful, no writes, no events, no
/// fee.
pub(crate) fn empty_success_output(
    auxiliary_data: TransactionAuxiliaryData,
) -> TransactionOutputWithoutGroups {
    empty_output(
        TransactionStatus::Keep(ExecutionStatus::Success),
        auxiliary_data,
    )
}

fn empty_output(
    status: TransactionStatus,
    auxiliary_data: TransactionAuxiliaryData,
) -> TransactionOutputWithoutGroups {
    TransactionOutputWithoutGroups {
        writes: vec![],
        events: vec![],
        gas_used: 0,
        status,
        auxiliary_data,
    }
}

/// One member's new state within a resource group: new bytes or removal.
type MemberOp = Option<Bytes>;

/// Serializes a written value straight out of the effects' frozen heap.
///
/// SAFETY: `ptr` must point to a live value of type `ty` in the effects' frozen
/// heap, `layouts` must describe `ty`, and no GC may run meanwhile.
unsafe fn written_bytes(
    layouts: &impl LayoutProvider,
    ptr: NonNull<u8>,
    ty: InternedType,
) -> Result<Bytes, String> {
    // SAFETY: forwarded from this function's contract.
    let blob = unsafe { serialize(layouts, ptr.as_ptr(), ty) }
        .map_err(|e| format!("failed to serialize written value: {e}"))?;
    Ok(Bytes::from(blob))
}

/// Converts every write outside a resource group into a write op.
///
/// SAFETY: this function needs to guarantee that its result (both the writes
/// and the errors) are deterministic, not depending on the order of iteration
/// of any data structures.
//
// Only entries whose value changed reach this point: at the end of the
// session, a copied-on-write value that still equals the value it was read
// from is no longer a write. This diverges from the V1 VM, which emits a
// modification for every resource it marks dirty (written through a mutable
// reference, or moved out and back), even when its bytes are unchanged.
//
// TODO(metering): writes are not charged IO gas or storage fees.
fn resource_writes(
    effects: &SessionEffects,
    layouts: &impl LayoutProvider,
) -> Result<Vec<(StateKey, WriteOp)>, Vec<String>> {
    let convert = |key: &InMemoryStorageKey, class: WriteClass| -> Result<_, String> {
        let state_key = key.as_state_key().map_err(|e| format!("{e:#}"))?;
        // SAFETY: written pointers refer to live values in the effects' frozen
        // heap, `layouts` describes their interned types, and no GC runs here.
        let op = match class {
            WriteClass::Creation { ptr, metadata, .. } => WriteOp::creation(
                unsafe { written_bytes(layouts, ptr, key.value_ty()) }?,
                metadata,
            ),
            WriteClass::Modification { ptr, metadata, .. } => WriteOp::modification(
                unsafe { written_bytes(layouts, ptr, key.value_ty()) }?,
                metadata,
            ),
            WriteClass::Deletion { metadata } => WriteOp::deletion(metadata),
        };
        Ok((state_key, op))
    };
    // Note on determinism: the read-write set iterates in an unspecified order,
    // so nothing here may depend on it. The caller's `WriteSet::new` sorts the
    // writes and `MaterializationError::new` sorts the failures.
    //
    // TODO(correctness): consider sorting the keys first to ensure determinism. This
    // cannot currently be done because keys contain `InternedType`, which is
    // basically a pointer and does not implement `Ord`.
    //
    // TODO(perf): currently we collect all errors and sort them to ensure determinism.
    // We should however revisit the design later and see if we want to switch to an alternative approach.
    //   - What if you want to fail fast on error?
    //   - Are we concerned about too many writes here?
    let mut writes = vec![];
    let mut failures = vec![];
    for (key, class, group) in effects.read_write_set().writes_unordered() {
        if group.is_some() {
            continue;
        }
        match convert(key, class) {
            Ok(write) => writes.push(write),
            Err(e) => failures.push(e),
        }
    }
    if failures.is_empty() {
        Ok(writes)
    } else {
        Err(failures)
    }
}

/// Merges the transaction's resource-group member writes into the groups'
/// stored members, which `provider` supplies, and returns one write per group.
///
/// SAFETY: this function needs to guarantee that its result (both the writes
/// and the errors) are deterministic, not depending on the order of iteration
/// of any data structures.
pub(crate) fn group_writes(
    effects: &SessionEffects,
    layouts: &impl LayoutProvider,
    provider: &dyn AptosDataProvider,
) -> Result<Vec<(StateKey, WriteOp)>, MaterializationError> {
    // Unordered on purpose: nothing below may depend on the order groups are
    // visited in (see the SAFETY note above). The inner map is ordered because
    // its keys are user-chosen struct tags and it merges into `GroupMembers`.
    let mut group_ops: UnorderedMap<InMemoryStorageKey, BTreeMap<StructTag, MemberOp>> =
        UnorderedMap::new();
    let mut failures = vec![];

    let mut collect = |key: &InMemoryStorageKey,
                       class: WriteClass,
                       group_ty: InternedType|
     -> Result<(), String> {
        // SAFETY: written pointers refer to live values in the effects' frozen
        // heap, `layouts` describes their interned types, and no GC runs here.
        let member_op = match class {
            WriteClass::Creation { ptr, .. } | WriteClass::Modification { ptr, .. } => {
                Some(unsafe { written_bytes(layouts, ptr, key.value_ty()) }?)
            },
            WriteClass::Deletion { .. } => None,
        };
        let member_tag = nominal_tag(key.value_ty()).map_err(|e| format!("{e:#}"))?;
        group_ops
            .entry(InMemoryStorageKey::resource_group(key.address(), group_ty))
            .or_default()
            .insert(member_tag, member_op);
        Ok(())
    };
    for (key, class, group) in effects.read_write_set().writes_unordered() {
        if let Some(group_ty) = group {
            if let Err(e) = collect(key, class, group_ty) {
                failures.push(e);
            }
        }
    }

    // Note on determinism: the groups are visited in an unspecified order. The
    // caller's `WriteSet::new` sorts the writes and `MaterializationError::new`
    // sorts the failures, so the order does not reach the result.
    let mut writes = vec![];
    for (group_key, member_ops) in group_ops.into_iter_unordered() {
        let merged = effects
            .read_write_set()
            .written_group(&group_key)
            .ok_or_else(|| format!("written group {group_key:?} was not recorded"))
            .and_then(|group| merge_group(provider, &group_key, group, member_ops));
        match merged {
            Ok(write) => writes.push(write),
            Err(e) => failures.push(e),
        }
    }
    if failures.is_empty() {
        Ok(writes)
    } else {
        Err(MaterializationError::new(failures))
    }
}

/// Merges a group's member ops into a single write for the group's slot:
/// creation, modification or deletion by the group's slot before and after the
/// transaction, which `group` records.
///
/// The stored blob is canonical in struct-tag order.
///
/// SAFETY: this function needs to guarantee that its result (both the write set
/// and the error) are deterministic, not depending on iteration order.
//
// TODO(cleanup): refactor this code so that serialization is decoupled from
// assembling the group.
fn merge_group(
    provider: &dyn AptosDataProvider,
    group_key: &InMemoryStorageKey,
    group: &GroupEntry,
    member_ops: BTreeMap<StructTag, MemberOp>,
) -> Result<(StateKey, WriteOp), String> {
    let stored = provider
        .stored_group(group_key)
        .map_err(|e| format!("failed to read the members of group {group_key:?}: {e:#}"))?;
    let state_key = group_key.as_state_key().map_err(|e| format!("{e:#}"))?;
    let slot = group
        .slot
        .as_ref()
        .ok_or_else(|| format!("the size of group {group_key:?} does not add up"))?;

    // Members the transaction wrote take precedence; removals are left out.
    let mut members = stored.map(|stored| stored.members).unwrap_or_default();
    for (tag, op) in member_ops {
        match op {
            Some(bytes) => {
                members.insert(tag, bytes);
            },
            None => {
                members.remove(&tag);
            },
        }
    }
    let encode = || {
        bcs::to_bytes(&members)
            .map(Bytes::from)
            .map_err(|e| format!("failed to encode the members of group {group_key:?}: {e}"))
    };
    let op = match (&group.slot_before.metadata, &slot.metadata) {
        (None, Some(metadata)) => WriteOp::creation(encode()?, metadata.clone()),
        (Some(_), Some(metadata)) => WriteOp::modification(encode()?, metadata.clone()),
        (Some(metadata), None) => WriteOp::deletion(metadata.clone()),
        // Unreachable: a member deletion implies the member (and so the
        // group) existed in storage.
        (None, None) => return Err("an empty group emitted a write".to_string()),
    };
    Ok((state_key, op))
}
