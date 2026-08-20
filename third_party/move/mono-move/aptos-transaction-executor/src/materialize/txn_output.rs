// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Creates a `TransactionOutput` from a transaction's side effects.

use crate::{
    errors::MaterializationError,
    providers::{nominal_tag, AptosDataProvider},
};
use aptos_types::{
    state_store::{state_key::StateKey, table::TableHandle},
    transaction::{TransactionAuxiliaryData, TransactionOutput, TransactionStatus},
    write_set::{WriteOp, WriteSet},
};
use bytes::Bytes;
use mono_move_core::{storage::resource_provider::InMemoryStorageKey, types::InternedType};
use mono_move_global_context::ExecutionGuard;
use mono_move_output::to_contract_events;
use mono_move_runtime::{serialize_value, SessionEffects, WriteClass};
use move_core_types::{language_storage::StructTag, vm_status::StatusCode};
use std::{
    collections::{BTreeMap, HashMap},
    ptr::NonNull,
};

// TODO(security): audit all error messages and make sure they do not lead to
// deep recursions or OOMs.

/// Creates the output of an executed transaction from its effects.
pub(crate) fn executed_output(
    effects: &SessionEffects,
    guard: &ExecutionGuard,
    provider: &dyn AptosDataProvider,
    gas_used: u64,
    status: TransactionStatus,
    auxiliary_data: TransactionAuxiliaryData,
) -> Result<TransactionOutput, MaterializationError> {
    let write_set = drain_write_set(effects, guard, provider).map_err(MaterializationError::new)?;
    // SAFETY: the effects' frozen heap (which event payloads point into) is
    // live for the duration of output assembly.
    let events = unsafe { to_contract_events(&effects.extensions, guard) }
        .map_err(|e| MaterializationError::new(vec![format!("event finalization failed: {e}")]))?;
    Ok(TransactionOutput::new(
        write_set,
        events,
        gas_used,
        status,
        auxiliary_data,
    ))
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

/// One member's new state within a resource group.
struct MemberOp {
    /// The member's new bytes, or `None` if the transaction removed it.
    bytes: Option<Bytes>,
    /// Whether the member existed in storage before this transaction. A
    /// modification or deletion read the member first, so it pre-existed; a
    /// creation did not. This lets `merge_group` decide the group slot's write
    /// class without relying on `old.is_some()`, which is unreliable when a
    /// provider returns post-apply members (see `merge_group`).
    pre_existed: bool,
}

/// The state key of an own-slot value: a resource under its struct tag, or a
/// table item under its handle. A group slot is never a read-write-set key --
/// its members carry their own `Resource` keys -- so it is an error here.
fn own_slot_state_key(key: &InMemoryStorageKey) -> Result<StateKey, String> {
    match key {
        InMemoryStorageKey::Resource { address, ty } => {
            StateKey::resource(address, &nominal_tag(*ty).map_err(|e| format!("{e:#}"))?)
                .map_err(|e| format!("bad state key: {e}"))
        },
        InMemoryStorageKey::TableItem { handle, key, .. } => {
            Ok(StateKey::table_item(&TableHandle(handle.address()), key))
        },
        InMemoryStorageKey::Group { .. } => Err(
            "cannot write a group slot key directly; its members have their own keys".to_string(),
        ),
    }
}

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
    guard: &ExecutionGuard<'_>,
    provider: &dyn AptosDataProvider,
) -> Result<WriteSet, Vec<String>> {
    let mut writes: Vec<(StateKey, WriteOp)> = vec![];
    // Keyed by the group slot's state key (where the merged blob is written);
    // the value pairs the slot's in-memory identity with its member ops.
    let mut group_ops: HashMap<StateKey, (InMemoryStorageKey, HashMap<InternedType, MemberOp>)> =
        HashMap::new();
    let mut failures: Vec<String> = vec![];

    // SAFETY: written pointers refer to live values in the effects' frozen
    // heap, and no GC runs during the drain.
    let written_bytes = |ptr: NonNull<u8>, ty: InternedType| -> Result<Bytes, String> {
        let blob = unsafe { serialize_value(guard, ptr, ty) }
            .map_err(|e| format!("failed to serialize written value: {e}"))?;
        Ok(Bytes::from(blob))
    };
    // `group` is the resolved resource group of the key's type, or `None` for an
    // own-slot value. It is resolved at the op site from the version-pinned
    // read-set, so this function performs no membership lookup.
    let mut convert = |key: &InMemoryStorageKey,
                       class: WriteClass,
                       group: Option<InternedType>|
     -> Result<(), String> {
        match group {
            None => {
                let state_key = own_slot_state_key(key)?;
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
                let address = key.address();
                let member = key.value_ty();
                let group_state_key = StateKey::resource_group(
                    &address,
                    &nominal_tag(group_ty).map_err(|e| format!("{e:#}"))?,
                );
                let group_key = InMemoryStorageKey::group(address, group_ty);
                let member_op = match class {
                    WriteClass::Creation(ptr) => MemberOp {
                        bytes: Some(written_bytes(ptr, member)?),
                        pre_existed: false,
                    },
                    WriteClass::Modification(ptr) => MemberOp {
                        bytes: Some(written_bytes(ptr, member)?),
                        pre_existed: true,
                    },
                    WriteClass::Deletion => MemberOp {
                        bytes: None,
                        pre_existed: true,
                    },
                };
                // Record the member before merging so a later group
                // enumeration (this or a future transaction) can see it.
                provider
                    .note_group_member(&group_key, member)
                    .map_err(|e| format!("{e:#}"))?;
                let entry = group_ops
                    .entry(group_state_key)
                    .or_insert_with(|| (group_key, HashMap::new()));
                entry.1.insert(member, member_op);
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
    for (key, class, group) in effects.read_write_set.writes_unordered() {
        // TODO(perf): currently we collect all errors and sort them to ensure determinism.
        // We should however revisit the design later and see if we want to switch to an alternative approach.
        //   - What if you want to fail fast on error?
        //   - Are we concerned about too many writes here?
        if let Err(e) = convert(key, class, group) {
            failures.push(e);
        }
    }

    // Merge each group's member ops into its stored members and emit one write
    // per group.
    for (group_state_key, (group_key, member_ops)) in group_ops {
        match merge_group(provider, &group_state_key, &group_key, member_ops) {
            Ok(op) => writes.push((group_state_key, op)),
            Err(e) => failures.push(e),
        }
    }

    if !failures.is_empty() {
        return Err(failures);
    }
    // `WriteSet::new` collects into a `BTreeMap`, so we do not need to sort the
    // `writes` vector here.
    WriteSet::new(writes).map_err(|e| vec![format!("failed to build the write set: {e:?}")])
}

/// Merges a group's member ops into a single write for the group's slot.
///
/// The stored blob is canonical in struct-tag order.
///
/// SAFETY: this function needs to guarantee that its result (both the write set
/// and the error) are deterministic, not depending on iteration order.
fn merge_group(
    provider: &dyn AptosDataProvider,
    group_state_key: &StateKey,
    group_key: &InMemoryStorageKey,
    member_ops: HashMap<InternedType, MemberOp>,
) -> Result<WriteOp, String> {
    let old = provider
        .group_members(group_key)
        .map_err(|e| format!("failed to read the members of group {group_state_key:?}: {e:#}"))?;
    let mut members: BTreeMap<StructTag, Bytes> = BTreeMap::new();

    // Members the transaction did not touch, then the ones it wrote, which take
    // precedence. Removed members are left out as part of this process.
    let untouched = old
        .iter()
        .flat_map(|stored| stored.iter())
        .filter(|(ty, _)| !member_ops.contains_key(ty));
    let written = member_ops
        .iter()
        .filter_map(|(ty, op)| Some((ty, op.bytes.as_ref()?)));
    for (ty, bytes) in untouched.chain(written) {
        // The error must not name the member to avoid non-determinism -- both
        // iterators are unordered.
        let tag = nominal_tag(*ty)
            .map_err(|_| format!("group {group_state_key:?} has a non-nominal member"))?;
        members.insert(tag, bytes.clone());
    }

    // The group slot pre-existed if any member the transaction did not touch is
    // still stored, or if any member op read an existing member (a modification
    // or deletion). `old.is_some()` alone is unreliable: a provider may return
    // post-apply members, which include members this transaction just created,
    // so a freshly created group would look pre-existing. The write class is
    // the authoritative signal, and for a pre-transaction `old` it yields the
    // same answer as `old.is_some()`.
    let existed = old
        .iter()
        .flat_map(|stored| stored.iter())
        .any(|(ty, _)| !member_ops.contains_key(ty))
        || member_ops.values().any(|op| op.pre_existed);

    let new_bytes = if members.is_empty() {
        None
    } else {
        let blob = bcs::to_bytes(&members).map_err(|e| {
            format!("failed to encode the members of group {group_state_key:?}: {e}")
        })?;
        Some(Bytes::from(blob))
    };
    group_op(existed, new_bytes)
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
