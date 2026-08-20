// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Creates a `TransactionOutput` from a transaction's side effects.

use crate::{errors::MaterializationError, providers::AptosDataProvider};
use aptos_types::{
    state_store::state_key::StateKey,
    transaction::{TransactionAuxiliaryData, TransactionOutput, TransactionStatus},
    write_set::{WriteOp, WriteSet},
};
use bytes::Bytes;
use mono_move_core::{
    nominal_tag, storage::resource_provider::InMemoryStorageKey, types::InternedType,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_output::to_contract_events;
use mono_move_runtime::{serialize, SessionEffects, WriteClass};
use move_core_types::{language_storage::StructTag, vm_status::StatusCode};
use std::{collections::HashMap, ptr::NonNull};

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
    guard: &ExecutionGuard<'_>,
    provider: &dyn AptosDataProvider,
) -> Result<WriteSet, Vec<String>> {
    let mut writes: Vec<(StateKey, WriteOp)> = vec![];
    let mut group_ops: HashMap<StateKey, HashMap<StructTag, MemberOp>> = HashMap::new();
    let mut failures: Vec<String> = vec![];

    // SAFETY: written pointers refer to live values in the effects' frozen
    // heap, and no GC runs during the drain.
    let written_bytes = |ptr: NonNull<u8>, ty: InternedType| -> Result<Bytes, String> {
        // SAFETY: forwarded from this function's contract.
        let blob = unsafe { serialize(guard, ptr.as_ptr(), ty) }
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
                let group_key = StateKey::resource_group(
                    &key.address(),
                    &nominal_tag(group_ty).map_err(|e| format!("{e:#}"))?,
                );
                let member_tag = nominal_tag(key.value_ty()).map_err(|e| format!("{e:#}"))?;
                let member_op = match class {
                    WriteClass::Creation(ptr) | WriteClass::Modification(ptr) => {
                        Some(written_bytes(ptr, key.value_ty())?)
                    },
                    WriteClass::Deletion => None,
                };
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
    for (key, class, group) in effects.read_write_set.writes_unordered() {
        // TODO(perf): currently we collect all errors and sort them to ensure determinism.
        // We should however revisit the design later and see if we want to switch to an alternative approach.
        //   - What if you want to fail fast on error?
        //   - Are we concerned about too many writes here?
        if let Err(e) = convert(key, class, group) {
            failures.push(e);
        }
    }

    // Emit one write per group: the provider overlays this transaction's member
    // ops onto the group's current state and returns the slot's write op.
    for (group_key, member_ops) in group_ops {
        match provider.group_write_op(&group_key, member_ops) {
            Ok(op) => writes.push((group_key, op)),
            Err(e) => failures.push(format!("failed to write group {group_key:?}: {e:#}")),
        }
    }

    if !failures.is_empty() {
        return Err(failures);
    }
    // `WriteSet::new` collects into a `BTreeMap`, so we do not need to sort the
    // `writes` vector here.
    WriteSet::new(writes).map_err(|e| vec![format!("failed to build the write set: {e:?}")])
}
