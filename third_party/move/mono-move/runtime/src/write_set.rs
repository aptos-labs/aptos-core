// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Transaction write-set generation.
//!
//! Serializes the resources and table items a finished transaction wrote into
//! an Aptos [`WriteSet`], keyed by [`StateKey`].
//!
//! Every `borrow_global_mut` / `move_to` counts as a write, even when the value
//! is unchanged (see `../docs/global_storage_design.md`).
//!
//! TODO(correctness, perf): revisit this over-approximation — drop writes whose
//! value equals the pre-state so the write set reflects real changes only.

use crate::{
    error::{RuntimeError, RuntimeResult},
    global_storage::{ResourceReadWriteSet, WriteClass},
    invariant_violation, value_utils,
};
use aptos_types::{
    state_store::{state_key::StateKey, table::TableHandle},
    write_set::{WriteOp, WriteSet},
};
use mono_move_core::{
    storage::resource_provider::InMemoryStorageKey, struct_tag_of, types::InternedType,
    value_layout::LayoutProvider,
};
use std::ptr::NonNull;

/// Builds the Aptos [`WriteSet`] from a read-write set. `layouts` BCS-serializes
/// created / modified values.
///
/// Write ops carry no `StateValueMetadata` (slot deposits, refunds) — that is a
/// separate workstream.
pub(crate) fn build_write_set<L: LayoutProvider + ?Sized>(
    rws: &ResourceReadWriteSet,
    layouts: &L,
) -> RuntimeResult<WriteSet> {
    let mut writes = Vec::new();
    for (key, class) in rws.writes() {
        let state_key = state_key_of(key)?;
        // TODO(correctness): these are metadata-less legacy write ops; we need to set the
        // `StateValueMetadata` (slot deposit / refund) carried over from the pre-state.
        let op = match class {
            WriteClass::Creation(ptr) => {
                WriteOp::legacy_creation(serialize_value(layouts, ptr, value_type(key))?.into())
            },
            WriteClass::Modification(ptr) => {
                WriteOp::legacy_modification(serialize_value(layouts, ptr, value_type(key))?.into())
            },
            WriteClass::Deletion => WriteOp::legacy_deletion(),
        };
        writes.push((state_key, op));
    }
    // Collects into a `BTreeMap`, so the result is canonically ordered by
    // `StateKey` regardless of the read-write set's iteration order.
    match WriteSet::new(writes) {
        Ok(write_set) => Ok(write_set),
        Err(_) => invariant_violation!(Unreachable("failed to freeze write set".to_string())),
    }
}

/// The Aptos [`StateKey`] for a read-write-set key.
fn state_key_of(key: &InMemoryStorageKey) -> RuntimeResult<StateKey> {
    match key {
        InMemoryStorageKey::Resource { address, ty } => {
            let Some(struct_tag) = struct_tag_of(*ty) else {
                invariant_violation!(Unreachable(
                    "resource key type must be a nominal type".to_string()
                ));
            };
            // A resource type nested deeper than the state key encoding allows is a runtime limit,
            // not an invariant violation: it is reachable with a sufficiently nested type.
            StateKey::resource(address, &struct_tag).map_err(|_| RuntimeError::StateKeyTypeTooDeep)
        },
        InMemoryStorageKey::TableItem { handle, key, .. } => {
            Ok(StateKey::table_item(&TableHandle(handle.address()), key))
        },
    }
}

/// The interned type of the value stored at a key (used to serialize it).
fn value_type(key: &InMemoryStorageKey) -> InternedType {
    match key {
        InMemoryStorageKey::Resource { ty, .. } => *ty,
        InMemoryStorageKey::TableItem { value_ty, .. } => *value_ty,
    }
}

/// BCS-serializes the value at `ptr` of type `ty`.
fn serialize_value<L: LayoutProvider + ?Sized>(
    layouts: &L,
    ptr: NonNull<u8>,
    ty: InternedType,
) -> RuntimeResult<Vec<u8>> {
    // SAFETY: `ptr` is the current value of a `LocalHeap` entry — a fully
    // initialized value of type `ty` living in this transaction's heap. The
    // heap stays alive for the call and no GC runs during write-set
    // generation, so the value (and everything it reaches) remains valid.
    unsafe { value_utils::serialize(layouts, ptr.as_ptr(), ty) }
}
