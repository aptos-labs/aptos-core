// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! A [`ResourceProvider`] backed by an in-memory map of BCS-encoded resources.
//!
//! Resources are materialized lazily on first read: the BCS bytes are
//! deserialized into a provider-owned heap (allocated with a real object
//! header so `deep_copy` and GC scanning work when the value is mutably
//! borrowed), and the resulting pointer is handed back as
//! [`StorageRead::ExternalHeap`] and memoized so later reads return the same
//! stable pointer. The provider heap is never garbage-collected, so its
//! pointers stay valid for the whole run (the interpreter never relocates
//! external reads). Intended for differential testing and benchmarking against
//! real downloaded state, not for production.

use crate::{
    heap::{heap_alloc, Heap},
    value_utils,
};
use mono_move_core::{
    storage::resource_provider::{
        InMemoryStorageKey, ResourceProvider, ResourceProviderError, StorageRead,
    },
    types::type_to_string,
    LayoutProvider, OBJECT_HEADER_SIZE,
};
use mono_move_global_context::ExecutionGuard;
use move_core_types::account_address::AccountAddress;
use std::{cell::RefCell, collections::HashMap, ptr::NonNull};

/// A read-only resource provider that materializes BCS-encoded values into a
/// dedicated heap on demand. See the module docs for the lifetime/GC model.
pub struct MaterializingResourceProvider<'g, 'ctx> {
    /// Source of layouts and struct descriptors for the resource types. Must be
    /// the same guard the interpreter runs against, so the interned types match
    /// the ones produced by lowering.
    guard: &'g ExecutionGuard<'ctx>,
    /// Pre-interned resource keys to their raw BCS bytes.
    bcs_by_key: HashMap<InMemoryStorageKey, Vec<u8>>,
    /// Table item bytes keyed by `(table handle address, serialized key)`. Kept
    /// separate from `bcs_by_key` because a table item's storage key carries a
    /// value type that is unknown when the cache is built; the type is supplied
    /// by the reader (it lives in the `TableItem` key) and used to materialize.
    table_bytes: HashMap<(AccountAddress, Box<[u8]>), Vec<u8>>,
    /// Provider-owned heap holding materialized values. Never GC'd.
    heap: RefCell<Heap>,
    /// Memoizes materialized reads so a key always maps to the same pointer.
    materialized: RefCell<HashMap<InMemoryStorageKey, StorageRead>>,
}

impl<'g, 'ctx> MaterializingResourceProvider<'g, 'ctx> {
    /// Builds a provider over `bcs_by_key` (pre-interned resource keys to their
    /// BCS bytes). The internal heap is sized from the total input size; the
    /// in-memory form is larger than BCS (object headers, pointer boxing,
    /// alignment), so the size is over-provisioned. If a resource still does
    /// not fit, [`get_resource`](Self::get_resource) surfaces an invariant
    /// violation.
    pub fn new(
        guard: &'g ExecutionGuard<'ctx>,
        bcs_by_key: HashMap<InMemoryStorageKey, Vec<u8>>,
        table_bytes: HashMap<(AccountAddress, Box<[u8]>), Vec<u8>>,
    ) -> Self {
        let total_bytes: usize = bcs_by_key
            .values()
            .chain(table_bytes.values())
            .map(|b| b.len())
            .sum();
        let entries = bcs_by_key.len() + table_bytes.len();
        let heap_size = total_bytes
            .saturating_mul(16)
            .saturating_add(entries.saturating_mul(4096))
            .max(1 << 20);
        Self {
            guard,
            bcs_by_key,
            table_bytes,
            heap: RefCell::new(Heap::new(heap_size)),
            materialized: RefCell::new(HashMap::new()),
        }
    }

    /// Deserialize the resource at `key` into the provider heap and return a
    /// pointer to it. Returns [`StorageRead::DoesNotExist`] when the key is
    /// absent (or is a table item, which this provider does not serve).
    fn materialize(&self, key: &InMemoryStorageKey) -> Result<StorageRead, ResourceProviderError> {
        // A resource is keyed (and looked up) by its full type-bearing key; a
        // table item's bytes are stored type-less by `(handle, key)`, and its
        // value type travels in the storage key.
        let (bytes, ty) = match key {
            InMemoryStorageKey::Resource { ty, .. } => {
                let Some(bytes) = self.bcs_by_key.get(key) else {
                    return Ok(StorageRead::DoesNotExist);
                };
                (bytes.as_slice(), *ty)
            },
            InMemoryStorageKey::TableItem {
                handle,
                key,
                value_ty,
            } => {
                let Some(bytes) = self.table_bytes.get(&(handle.address(), key.clone())) else {
                    return Ok(StorageRead::DoesNotExist);
                };
                (bytes.as_slice(), *value_ty)
            },
        };

        let size = self
            .guard
            .layout_by_ty(ty)
            .ok_or_else(|| {
                ResourceProviderError::InvariantViolation(format!(
                    "no layout published for type {}",
                    type_to_string(ty),
                ))
            })?
            .size;
        let desc_id = self.guard.struct_descriptor_for(ty).ok_or_else(|| {
            ResourceProviderError::InvariantViolation(format!(
                "no struct descriptor published for type {}",
                type_to_string(ty),
            ))
        })?;

        let total = OBJECT_HEADER_SIZE + size as usize;
        let mut heap = self.heap.borrow_mut();
        let obj = heap_alloc(&mut heap, total, desc_id).map_err(|e| {
            ResourceProviderError::InvariantViolation(format!("provider heap allocation: {e:?}"))
        })?;
        // SAFETY: `obj` is a freshly-allocated, header-stamped object whose
        // payload is `size` bytes; `guard` supplies the layout for `ty`. The
        // provider heap never GCs, so `obj` stays valid for the whole run.
        unsafe {
            value_utils::deserialize_with_interner(self.guard, self.guard, &mut heap, ty, bytes, obj)
                .map_err(|e| {
                    ResourceProviderError::InvariantViolation(format!("deserialize resource: {e:?}"))
                })?;
        }
        // SAFETY: `heap_alloc` never returns a null object pointer.
        let ptr = unsafe { NonNull::new_unchecked(obj) };
        Ok(StorageRead::ExternalHeap { ptr, version: 0 })
    }
}

impl ResourceProvider for MaterializingResourceProvider<'_, '_> {
    fn get_resource(&self, key: &InMemoryStorageKey) -> Result<StorageRead, ResourceProviderError> {
        if let Some(read) = self.materialized.borrow().get(key) {
            return Ok(*read);
        }
        let read = self.materialize(key)?;
        self.materialized.borrow_mut().insert(key.clone(), read);
        Ok(read)
    }
}
