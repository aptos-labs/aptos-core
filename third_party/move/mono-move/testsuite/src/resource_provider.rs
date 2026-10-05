// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The in-memory [`ResourceProvider`] shared by the MonoMove test harnesses.
//!
//! Resources are held as BCS bytes in V1's [`InMemoryStorage`] and
//! materialized (BCS -> flat) into an arena the provider owns on first access,
//! then served as a pointer that pins the arena. Materialization is lazy
//! because a type's layout and GC descriptor are only published once the
//! function that accesses it has been lowered.

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
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::resolver::ResourceResolver;
use std::{cell::RefCell, collections::HashMap, ptr::NonNull, sync::Arc};

/// Size of a provider's materialization arena: the interpreter's, so a value
/// that fit when it was written fits when it is read back.
const MATERIALIZATION_HEAP_SIZE: usize = DEFAULT_HEAP_SIZE;

/// Serves the resources of V1's [`InMemoryStorage`] to MonoMove, looking each
/// up by the struct tag of its interned type and materializing it on first
/// access.
pub(crate) struct InMemoryResourceProvider<'guard, 'ctx> {
    guard: &'guard ExecutionGuard<'ctx>,
    storage: &'guard InMemoryStorage,
    materialized: RefCell<Materialized>,
}

struct Materialized {
    /// Holds the flat representation of materialized values. Shared so each
    /// read pins it; never reset, so every interpreter reading a value keeps
    /// it alive.
    arena: Arc<SharedArena>,
    cache: HashMap<InMemoryStorageKey, NonNull<u8>>,
}

impl<'guard, 'ctx> InMemoryResourceProvider<'guard, 'ctx> {
    /// Creates a provider over `storage`.
    pub(crate) fn new(
        guard: &'guard ExecutionGuard<'ctx>,
        storage: &'guard InMemoryStorage,
    ) -> Self {
        Self {
            guard,
            storage,
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
                version: 0,
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
            version: 0,
            pin: arena,
        })
    }
}

impl ResourceProvider for InMemoryResourceProvider<'_, '_> {
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
                let (blob, _) = self
                    .storage
                    .get_resource_bytes_with_metadata_and_layout(address, &tag, &[], None)
                    .map_err(|err| ResourceProviderError::InvariantViolation(err.to_string()))?;
                match blob {
                    Some(blob) => self.materialize(key, *ty, &blob),
                    None => Ok(StorageRead::DoesNotExist),
                }
            },
            // TODO(testing): serve table items from the storage's tables once the
            // session commits table writes, so transactional sources can keep tables
            // across tasks.
            InMemoryStorageKey::TableItem { .. } => Ok(StorageRead::DoesNotExist),
        }
    }
}

fn materialize_one(
    heap: &mut Heap,
    guard: &ExecutionGuard,
    ty: InternedType,
    blob: &[u8],
) -> Result<NonNull<u8>, ResourceProviderError> {
    let layout = guard.layout_by_ty(ty).ok_or_else(|| {
        ResourceProviderError::InvariantViolation("no layout for a stored value's type".to_string())
    })?;
    // Lowering the accessing function published the descriptor, as in
    // production.
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
