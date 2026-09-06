// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Serves resources to MonoMove from a lookup of their BCS bytes.
//!
//! Resources are decoded from BCS into flat values in the provider's arena
//! on first access. Each storage read includes a pointer and shared ownership of the arena.
//! Materialization is lazy because a type's layout and GC descriptor are only
//! published once the function that accesses it has been lowered.

use bytes::Bytes;
use mono_move_core::{
    nominal_tag,
    storage::resource_provider::{
        InMemoryStorageKey, ResourceProvider, ResourceProviderError, StorageRead,
    },
    types::InternedType,
    LayoutProvider, OBJECT_HEADER_SIZE,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{deserialize_into, Heap, SharedArena, DEFAULT_HEAP_SIZE};
use move_core_types::{account_address::AccountAddress, language_storage::StructTag};
use std::{cell::RefCell, collections::HashMap, ptr::NonNull, sync::Arc};

/// Capacity shared by all materialized resources, matching the default interpreter heap size.
const MATERIALIZATION_HEAP_SIZE: usize = DEFAULT_HEAP_SIZE;

/// Serves the resources `lookup` finds, looking each up by the struct tag of
/// its interned type and materializing it on first access. Table items are
/// never served.
pub struct LazyResourceProvider<'guard, 'ctx, L> {
    guard: &'guard ExecutionGuard<'ctx>,
    lookup: L,
    materialized: RefCell<Materialized>,
}

struct Materialized {
    /// Holds decoded values and is never reset. Each storage read retains an
    /// `Arc` to keep the arena alive while its values are in use.
    arena: Arc<SharedArena>,
    cache: HashMap<InMemoryStorageKey, NonNull<u8>>,
}

impl<'guard, 'ctx, L> LazyResourceProvider<'guard, 'ctx, L>
where
    L: Fn(AccountAddress, StructTag) -> Result<Option<Bytes>, ResourceProviderError>,
{
    /// `lookup` returns a resource's BCS bytes, or `None` if it does not
    /// exist.
    pub fn new(guard: &'guard ExecutionGuard<'ctx>, lookup: L) -> Self {
        Self {
            guard,
            lookup,
            materialized: RefCell::new(Materialized {
                #[allow(clippy::arc_with_non_send_sync)]
                arena: Arc::new(SharedArena::new(MATERIALIZATION_HEAP_SIZE)),
                cache: HashMap::new(),
            }),
        }
    }

    /// The value already materialized for `key`, if any.
    fn cached(&self, key: &InMemoryStorageKey) -> Option<StorageRead> {
        let materialized = self.materialized.borrow();
        materialized
            .cache
            .get(key)
            .map(|&ptr| StorageRead::ExternalHeap {
                ptr,
                version: None,
                pin: materialized.arena.clone(),
            })
    }

    /// Materializes `blob` as a value of type `ty` and caches it under `key`.
    fn materialize(
        &self,
        key: &InMemoryStorageKey,
        ty: InternedType,
        blob: &[u8],
    ) -> Result<StorageRead, ResourceProviderError> {
        let mut materialized = self.materialized.borrow_mut();
        let arena = materialized.arena.clone();
        let ptr = arena.with_heap_mut(|heap| materialize_one(heap, self.guard, ty, blob))?;
        materialized.cache.insert(key.clone(), ptr);
        Ok(StorageRead::ExternalHeap {
            ptr,
            version: None,
            pin: arena,
        })
    }
}

impl<L> ResourceProvider for LazyResourceProvider<'_, '_, L>
where
    L: Fn(AccountAddress, StructTag) -> Result<Option<Bytes>, ResourceProviderError>,
{
    fn get_resource(
        &self,
        key: &InMemoryStorageKey,
        _group: Option<InternedType>,
    ) -> Result<StorageRead, ResourceProviderError> {
        if let Some(read) = self.cached(key) {
            return Ok(read);
        }
        match key {
            InMemoryStorageKey::Resource { address, ty } => {
                let tag = nominal_tag(*ty)
                    .map_err(|err| ResourceProviderError::InvariantViolation(format!("{err:#}")))?;
                match (self.lookup)(*address, tag)? {
                    Some(blob) => self.materialize(key, *ty, &blob),
                    None => Ok(StorageRead::DoesNotExist { version: None }),
                }
            },
            // TODO(testing): serve table items through the lookup, so the
            // transactional session can keep tables across tasks once it
            // commits table writes.
            InMemoryStorageKey::TableItem { .. } => Ok(StorageRead::DoesNotExist { version: None }),
            // Resource groups are a storage-layer concern; this provider serves
            // members directly by their own key.
            InMemoryStorageKey::ResourceGroup { .. } => {
                Ok(StorageRead::DoesNotExist { version: None })
            },
        }
    }
}

/// Allocates a value of type `ty` on `heap` and decodes `blob`, its BCS
/// encoding, into it. `ty`'s layout and GC descriptor must be published.
fn materialize_one(
    heap: &mut Heap,
    guard: &ExecutionGuard,
    ty: InternedType,
    blob: &[u8],
) -> Result<NonNull<u8>, ResourceProviderError> {
    let layout = guard.layout_by_ty(ty).ok_or_else(|| {
        ResourceProviderError::InvariantViolation("no layout for a stored value's type".to_string())
    })?;
    // Lowering the accessing function publishes the resource type's GC descriptor.
    let descriptor = guard.struct_descriptor(ty).ok_or_else(|| {
        ResourceProviderError::InvariantViolation(
            "no GC descriptor for a stored value's type".to_string(),
        )
    })?;
    let obj = heap
        .alloc_object(OBJECT_HEADER_SIZE + layout.size as usize, descriptor)
        .ok_or_else(|| {
            ResourceProviderError::InvariantViolation(
                "the materialization heap is exhausted".to_string(),
            )
        })?;
    // SAFETY: `obj` is a fresh allocation of the type's size, and `blob` is
    // the BCS encoding of a value of type `ty`.
    unsafe { deserialize_into(guard, heap, ty, blob, obj.as_ptr()) }.map_err(|err| {
        ResourceProviderError::InvariantViolation(format!("a stored value does not decode: {err}"))
    })?;
    Ok(obj)
}
