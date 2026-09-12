// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! In-memory [`ResourceProvider`]s shared by the MonoMove test harnesses.
//!
//! Resources and table items are held as BCS bytes and materialized (BCS ->
//! flat) into a heap the provider owns on first access, then served as a
//! pointer. Materialization is lazy because a type's layout and GC descriptor
//! are only published once the function that accesses it has been lowered.

use mono_move_core::{
    nominal_tag,
    storage::resource_provider::{
        InMemoryStorageKey, ResourceProvider, ResourceProviderError, StorageRead,
    },
    types::InternedType,
    LayoutProvider, OBJECT_HEADER_SIZE,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{deserialize_into, Heap, DEFAULT_HEAP_SIZE};
use move_core_types::account_address::AccountAddress;
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::resolver::ResourceResolver;
use std::{cell::RefCell, collections::HashMap, ptr::NonNull};

/// Size of a provider's materialization heap: the interpreter's, so a value
/// that fit when it was written fits when it is read back.
pub(crate) const MATERIALIZATION_HEAP_SIZE: usize = DEFAULT_HEAP_SIZE;

/// Serves resources and table items to MonoMove, materializing each on first access.
pub struct InMemoryResourceProvider<'guard, 'ctx> {
    /// BCS bytes of each resource, keyed by address and interned type.
    resources: HashMap<(AccountAddress, InternedType), Vec<u8>>,
    /// BCS bytes of each table item, keyed by table handle and serialized key.
    table_items: HashMap<(AccountAddress, Vec<u8>), Vec<u8>>,
    materializer: Materializer<'guard, 'ctx>,
}

impl<'guard, 'ctx> InMemoryResourceProvider<'guard, 'ctx> {
    /// Creates an empty provider whose materialization arena is `heap_size` bytes.
    pub fn new(guard: &'guard ExecutionGuard<'ctx>, heap_size: usize) -> Self {
        Self {
            resources: HashMap::new(),
            table_items: HashMap::new(),
            materializer: Materializer::new(guard, heap_size),
        }
    }

    pub fn add_resource(&mut self, address: AccountAddress, ty: InternedType, bytes: Vec<u8>) {
        self.resources.insert((address, ty), bytes);
    }

    pub fn add_table_item(&mut self, handle_address: AccountAddress, key: Vec<u8>, bytes: Vec<u8>) {
        self.table_items.insert((handle_address, key), bytes);
    }

    fn entry(&self, key: &InMemoryStorageKey) -> Option<(&Vec<u8>, InternedType)> {
        match key {
            InMemoryStorageKey::Resource { address, ty } => {
                Some((self.resources.get(&(*address, *ty))?, *ty))
            },
            InMemoryStorageKey::TableItem {
                handle,
                key,
                value_ty,
            } => Some((
                self.table_items.get(&(handle.address(), key.to_vec()))?,
                *value_ty,
            )),
        }
    }
}

impl ResourceProvider for InMemoryResourceProvider<'_, '_> {
    fn get_resource(
        &self,
        key: &InMemoryStorageKey,
        _group: Option<InternedType>,
    ) -> Result<StorageRead, ResourceProviderError> {
        if let Some(read) = self.materializer.cached(key) {
            return Ok(read);
        }
        let Some((blob, ty)) = self.entry(key) else {
            return Ok(StorageRead::DoesNotExist);
        };
        self.materializer.materialize(key, ty, blob)
    }
}

/// Serves the resources of V1's [`InMemoryStorage`] to MonoMove, looking each
/// up by the struct tag of its interned type. Table items are never present:
/// nothing writes them to the storage.
pub(crate) struct StorageResourceProvider<'guard, 'ctx> {
    storage: &'guard InMemoryStorage,
    materializer: Materializer<'guard, 'ctx>,
}

impl<'guard, 'ctx> StorageResourceProvider<'guard, 'ctx> {
    /// Creates a provider over `storage` whose materialization arena is
    /// `heap_size` bytes.
    pub(crate) fn new(
        guard: &'guard ExecutionGuard<'ctx>,
        storage: &'guard InMemoryStorage,
        heap_size: usize,
    ) -> Self {
        Self {
            storage,
            materializer: Materializer::new(guard, heap_size),
        }
    }
}

impl ResourceProvider for StorageResourceProvider<'_, '_> {
    fn get_resource(
        &self,
        key: &InMemoryStorageKey,
        _group: Option<InternedType>,
    ) -> Result<StorageRead, ResourceProviderError> {
        if let Some(read) = self.materializer.cached(key) {
            return Ok(read);
        }
        let InMemoryStorageKey::Resource { address, ty } = key else {
            return Ok(StorageRead::DoesNotExist);
        };
        let tag = nominal_tag(*ty)
            .map_err(|err| ResourceProviderError::InvariantViolation(format!("{err:#}")))?;
        let (blob, _) = self
            .storage
            .get_resource_bytes_with_metadata_and_layout(address, &tag, &[], None)
            .map_err(|err| ResourceProviderError::InvariantViolation(err.to_string()))?;
        match blob {
            Some(blob) => self.materializer.materialize(key, *ty, &blob),
            None => Ok(StorageRead::DoesNotExist),
        }
    }
}

/// Materializes BCS blobs into a long-lived heap, once per storage key.
struct Materializer<'guard, 'ctx> {
    guard: &'guard ExecutionGuard<'ctx>,
    state: RefCell<Materialized>,
}

struct Materialized {
    /// Holds the flat representation of materialized values for the
    /// provider's lifetime, which must cover every interpreter reading them.
    heap: Heap,
    cache: HashMap<InMemoryStorageKey, NonNull<u8>>,
}

impl<'guard, 'ctx> Materializer<'guard, 'ctx> {
    fn new(guard: &'guard ExecutionGuard<'ctx>, heap_size: usize) -> Self {
        Self {
            guard,
            state: RefCell::new(Materialized {
                heap: Heap::new(heap_size),
                cache: HashMap::new(),
            }),
        }
    }

    /// The value already materialized for `key`, if any.
    fn cached(&self, key: &InMemoryStorageKey) -> Option<StorageRead> {
        self.state
            .borrow()
            .cache
            .get(key)
            .map(|&ptr| StorageRead::ExternalHeap { ptr, version: 0 })
    }

    /// Materializes `blob` as a value of type `ty` and caches it under `key`.
    fn materialize(
        &self,
        key: &InMemoryStorageKey,
        ty: InternedType,
        blob: &[u8],
    ) -> Result<StorageRead, ResourceProviderError> {
        let mut state = self.state.borrow_mut();
        let ptr = materialize_one(&mut state.heap, self.guard, ty, blob)?;
        state.cache.insert(key.clone(), ptr);
        Ok(StorageRead::ExternalHeap { ptr, version: 0 })
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
