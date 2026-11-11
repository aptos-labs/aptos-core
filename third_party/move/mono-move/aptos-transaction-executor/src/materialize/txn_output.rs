// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Creates a `TransactionOutput` from a transaction's side effects.

use crate::{
    errors::MaterializationError,
    group_assembly::{GroupAssembly, PER_TXN_ASSEMBLY_GROUPS, PER_TXN_ASSEMBLY_NANOS},
    providers::{AptosDataProvider, GroupMembers, MaterializedGroups},
};
use aptos_types::{
    state_store::state_key::StateKey,
    transaction::{
        ExecutionStatus, TransactionAuxiliaryData, TransactionOutput, TransactionStatus,
    },
    write_set::{PendingGroup, WriteOp, WriteSet},
};
use bytes::Bytes;
use mono_move_core::{
    nominal_tag, storage::resource_provider::InMemoryStorageKey, types::InternedType,
    value_layout::LayoutProvider,
};
use mono_move_output::to_contract_events;
use mono_move_runtime::{serialize, SessionEffects, WriteClass};
use move_core_types::{language_storage::StructTag, vm_status::StatusCode};
use shared_dsa::UnorderedMap;
use std::{collections::BTreeMap, ptr::NonNull, sync::atomic::Ordering, time::Instant};

// TODO(security): audit all error messages and make sure they do not lead to
// deep recursions or OOMs.

// TODO(security): pin providers so that execution and materialization use
// identical layout and data providers.

/// Creates the output of an executed transaction from its effects.
pub(crate) fn executed_output(
    effects: &SessionEffects,
    layouts: &impl LayoutProvider,
    provider: &dyn AptosDataProvider,
    gas_used: u64,
    status: TransactionStatus,
    auxiliary_data: TransactionAuxiliaryData,
) -> Result<(TransactionOutput, MaterializedGroups), MaterializationError> {
    let (write_set, groups) =
        drain_write_set(effects, layouts, provider).map_err(MaterializationError::new)?;
    let events = to_contract_events(effects, layouts)
        .map_err(|e| MaterializationError::new(vec![format!("event finalization failed: {e}")]))?;
    let output = TransactionOutput::new(write_set, events, gas_used, status, auxiliary_data);
    Ok((output, groups))
}

/// Creates the output of a discarded transaction: empty, carrying the reason.
pub(crate) fn discarded_output(
    status_code: StatusCode,
    auxiliary_data: TransactionAuxiliaryData,
) -> TransactionOutput {
    TransactionOutput::new(
        WriteSet::default(),
        vec![],
        0,
        TransactionStatus::Discard(status_code),
        auxiliary_data,
    )
}

/// Creates the committed empty output: successful, no writes, no events, no
/// fee.
pub(crate) fn empty_success_output(auxiliary_data: TransactionAuxiliaryData) -> TransactionOutput {
    TransactionOutput::new(
        WriteSet::default(),
        vec![],
        0,
        TransactionStatus::Keep(ExecutionStatus::Success),
        auxiliary_data,
    )
}

/// One member's new state within a resource group: new bytes or removal.
type MemberOp = Option<Bytes>;

/// Drains the read-write set into a write set, serializing written values
/// straight out of the effects' frozen heap.
///
/// SAFETY: this function needs to guarantee that its result (both the write set
/// and the errors) are deterministic, not depending on the order of iteration of
/// any data structures.
//
// TODO(correctness): every copied-on-write entry is emitted as a write, even
// when the bytes are unchanged. Eventually we will want to byte-compare against
// the original bytes.
//
// TODO(metering): writes are not charged IO gas or storage fees.
fn drain_write_set(
    effects: &SessionEffects,
    layouts: &impl LayoutProvider,
    provider: &dyn AptosDataProvider,
) -> Result<(WriteSet, MaterializedGroups), Vec<String>> {
    let mut writes: Vec<(StateKey, WriteOp)> = vec![];
    // Unordered on purpose: nothing below may depend on the order groups are
    // visited in (see the SAFETY note above). The inner map is ordered because
    // its keys are user-chosen struct tags and it merges into `GroupMembers`.
    let mut group_ops: UnorderedMap<InMemoryStorageKey, BTreeMap<StructTag, MemberOp>> =
        UnorderedMap::new();
    let mut materialized_groups = MaterializedGroups::new();
    let mut failures: Vec<String> = vec![];

    // SAFETY: written pointers refer to live values in the effects' frozen
    // heap, `layouts` describes their interned types, and no GC runs during the
    // drain.
    let written_bytes = |ptr: NonNull<u8>, ty: InternedType| -> Result<Bytes, String> {
        // SAFETY: forwarded from this function's contract.
        let blob = unsafe { serialize(layouts, ptr.as_ptr(), ty) }
            .map_err(|e| format!("failed to serialize written value: {e}"))?;
        Ok(Bytes::from(blob))
    };
    let mut convert = |key: &InMemoryStorageKey,
                       class: WriteClass,
                       group: Option<InternedType>|
     -> Result<(), String> {
        match group {
            None => {
                let state_key = key.as_state_key().map_err(|e| format!("{e:#}"))?;
                // TODO(correctness): ops carry no `StateValueMetadata` (slot
                // deposits, refunds, creation time); the legacy VM's
                // `WriteOpConverter` fills these.
                let op = match class {
                    WriteClass::Creation(ptr) => {
                        WriteOp::legacy_creation(written_bytes(ptr, key.value_ty())?)
                    },
                    WriteClass::Modification(ptr) => {
                        WriteOp::legacy_modification(written_bytes(ptr, key.value_ty())?)
                    },
                    WriteClass::Deletion => WriteOp::legacy_deletion(),
                };
                writes.push((state_key, op));
            },
            Some(group_ty) => {
                let group_key = InMemoryStorageKey::resource_group(key.address(), group_ty);
                let member_op = match class {
                    WriteClass::Creation(ptr) | WriteClass::Modification(ptr) => {
                        Some(written_bytes(ptr, key.value_ty())?)
                    },
                    WriteClass::Deletion => None,
                };
                let member_tag = nominal_tag(key.value_ty()).map_err(|e| format!("{e:#}"))?;
                group_ops
                    .entry(group_key)
                    .or_default()
                    .insert(member_tag, member_op);
            },
        }
        Ok(())
    };
    // Note on determinism: the read-write set iterates in an unspecified order,
    // so nothing here may depend on it.
    //
    // TODO(correctness): consider sorting the keys first to ensure determinism. This
    // cannot currently be done because keys contain `InternedType`, which is
    // basically a pointer and does not implement `Ord`.
    for (key, class, group) in effects.read_write_set().writes_unordered() {
        // TODO(perf): currently we collect all errors and sort them to ensure determinism.
        // We should however revisit the design later and see if we want to switch to an alternative approach.
        //   - What if you want to fail fast on error?
        //   - Are we concerned about too many writes here?
        if let Err(e) = convert(key, class, group) {
            failures.push(e);
        }
    }

    // At block end, the executor assembles groups from every transaction's
    // member ops, so they are passed on as they are.
    let mut pending: BTreeMap<StateKey, PendingGroup> = BTreeMap::new();
    match GroupAssembly::get() {
        GroupAssembly::BlockEnd | GroupAssembly::BlockEndFlat => {
            for (group_key, member_ops) in group_ops.into_iter_unordered() {
                match group_key.as_state_key() {
                    Ok(state_key) => {
                        pending.insert(state_key, member_ops);
                    },
                    Err(e) => failures.push(format!("{e:#}")),
                }
            }
        },
        // The block executor assembles groups from the links it versions.
        GroupAssembly::Chained => {},
        GroupAssembly::PerTxn => {
            // Merge each group's member ops into its stored members and emit one
            // write per group. The assembled members are returned so the caller can
            // cache them without decoding the write op again.
            //
            // Note on determinism: the groups are visited in an unspecified order.
            // `WriteSet::new` sorts the writes and `MaterializationError::new` sorts
            // the failures, so the order does not reach the result.
            let start = Instant::now();
            let num_groups = group_ops.len() as u64;
            for (group_key, member_ops) in group_ops.into_iter_unordered() {
                let merged =
                    merge_group(provider, &group_key, member_ops).and_then(|(op, members)| {
                        let state_key = group_key.as_state_key().map_err(|e| format!("{e:#}"))?;
                        Ok((state_key, op, members))
                    });
                match merged {
                    Ok((state_key, op, members)) => {
                        materialized_groups.insert(group_key, members);
                        writes.push((state_key, op));
                    },
                    Err(e) => failures.push(e),
                }
            }
            if num_groups > 0 {
                PER_TXN_ASSEMBLY_NANOS
                    .fetch_add(start.elapsed().as_nanos() as u64, Ordering::Relaxed);
                PER_TXN_ASSEMBLY_GROUPS.fetch_add(num_groups, Ordering::Relaxed);
            }
        },
    }

    if !failures.is_empty() {
        return Err(failures);
    }
    // `WriteSet::new` collects into a `BTreeMap`, so we do not need to sort the
    // `writes` vector here.
    let mut write_set =
        WriteSet::new(writes).map_err(|e| vec![format!("failed to build the write set: {e:?}")])?;
    if !pending.is_empty() {
        write_set.add_pending_groups(pending);
    }
    Ok((write_set, materialized_groups))
}

/// Merges a group's member ops into a single write for the group's slot, and
/// returns the assembled members (`None` when the group is now empty).
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
    member_ops: BTreeMap<StructTag, MemberOp>,
) -> Result<(WriteOp, Option<GroupMembers>), String> {
    let old = provider
        .group_members(group_key)
        .map_err(|e| format!("failed to read the members of group {group_key:?}: {e:#}"))?;
    let existed = old.is_some();

    // Members the transaction wrote take precedence; removals are left out.
    let mut members = old.unwrap_or_default();
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
    if members.is_empty() {
        Ok((group_op(existed, None)?, None))
    } else {
        let blob = bcs::to_bytes(&members)
            .map_err(|e| format!("failed to encode the members of group {group_key:?}: {e}"))?;
        let op = group_op(existed, Some(Bytes::from(blob)))?;
        Ok((op, Some(members)))
    }
}

/// Builds the write op for a resource-group slot: creation/modification/
/// deletion by whether the group existed before and has members after.
//
// TODO(correctness): ops carry no `StateValueMetadata` (slot deposits, refunds,
// creation time); the legacy VM's `WriteOpConverter` fills these.
fn group_op(existed: bool, new_bytes: Option<Bytes>) -> Result<WriteOp, String> {
    Ok(match (existed, new_bytes) {
        (false, Some(bytes)) => WriteOp::legacy_creation(bytes),
        (true, Some(bytes)) => WriteOp::legacy_modification(bytes),
        (true, None) => WriteOp::legacy_deletion(),
        (false, None) => {
            // Unreachable: a member deletion implies the member (and so the
            // group) existed in storage.
            return Err("an empty group emitted a write".to_string());
        },
    })
}
