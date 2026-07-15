// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! An in-memory [`ResourceProvider`] shared by the MonoMove test harnesses.
//!
//! Resources and table items are held as BCS bytes and materialized (BCS ->
//! flat) into a long-lived heap on first access, then served as a pointer.
//! Materialization is lazy because a type's layout is only published once the
//! function that accesses it has been lowered.

use mono_move_core::{
    storage::resource_provider::{
        InMemoryStorageKey, ResourceProvider, ResourceProviderError, StorageRead,
    },
    types::InternedType,
    FrameOffset, LayoutKind, LayoutProvider, ValueLayout, OBJECT_HEADER_SIZE,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{deserialize_into, Heap};
use move_core_types::account_address::AccountAddress;
use std::{cell::RefCell, collections::HashMap, ptr::NonNull};

/// Serves resources and table items to MonoMove, materializing each on first access.
pub struct InMemoryResourceProvider<'guard, 'ctx> {
    guard: &'guard ExecutionGuard<'ctx>,
    /// BCS bytes of each resource, keyed by address and interned type.
    resources: HashMap<(AccountAddress, InternedType), Vec<u8>>,
    /// BCS bytes of each table item, keyed by table handle and serialized key.
    table_items: HashMap<(AccountAddress, Vec<u8>), Vec<u8>>,
    materialized: RefCell<Materialized>,
}

struct Materialized {
    /// Long-lived arena holding the flat representation of materialized resources. Must outlive the
    /// interpreter; never reset between runs.
    heap: Heap,
    cache: HashMap<InMemoryStorageKey, NonNull<u8>>,
}

impl<'guard, 'ctx> InMemoryResourceProvider<'guard, 'ctx> {
    /// Creates an empty provider whose materialization arena is `heap_size` bytes.
    pub fn new(guard: &'guard ExecutionGuard<'ctx>, heap_size: usize) -> Self {
        Self {
            guard,
            resources: HashMap::new(),
            table_items: HashMap::new(),
            materialized: RefCell::new(Materialized {
                heap: Heap::new(heap_size),
                cache: HashMap::new(),
            }),
        }
    }

    /// Adds a resource of interned type `ty` published at `address`, given its BCS bytes.
    pub fn add_resource(&mut self, address: AccountAddress, ty: InternedType, bytes: Vec<u8>) {
        self.resources.insert((address, ty), bytes);
    }

    /// Adds a table item at `handle_address` keyed by serialized `key`, given its BCS bytes.
    pub fn add_table_item(&mut self, handle_address: AccountAddress, key: Vec<u8>, bytes: Vec<u8>) {
        self.table_items.insert((handle_address, key), bytes);
    }

    /// Returns the raw blob and the type to materialize it as, or `None` if the key isn't present.
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
    fn get_resource(&self, key: &InMemoryStorageKey) -> Result<StorageRead, ResourceProviderError> {
        // Cache hit?
        {
            let materialized = self.materialized.borrow();
            if let Some(&ptr) = materialized.cache.get(key) {
                return Ok(StorageRead::ExternalHeap { ptr, version: 0 });
            }
        }

        let Some((blob, ty)) = self.entry(key) else {
            return Ok(StorageRead::DoesNotExist);
        };

        let mut materialized = self.materialized.borrow_mut();
        match materialize_one(&mut materialized.heap, self.guard, ty, blob) {
            Some(ptr) => {
                materialized.cache.insert(key.clone(), ptr);
                Ok(StorageRead::ExternalHeap { ptr, version: 0 })
            },
            None => Ok(StorageRead::DoesNotExist),
        }
    }
}

/// Materializes one resource of type `ty` from its BCS `blob` into `heap`, returning a pointer to
/// the flat object (header at the preceding bytes). Returns `None` if the layout is unavailable
/// (the accessing function wasn't lowered) or the arena is full.
fn materialize_one(
    heap: &mut Heap,
    guard: &ExecutionGuard,
    ty: InternedType,
    blob: &[u8],
) -> Option<NonNull<u8>> {
    let layout = guard.layout_by_ty(ty)?;
    let size = layout.size as usize;

    // The GC descriptor records which payload slots are heap pointers. Lowering already published
    // it for this resource type; `publish_struct_descriptor` is idempotent and returns that one
    // (our offsets are a fallback that is ignored on the fast path).
    let mut offsets = vec![];
    collect_pointer_offsets(guard, layout, 0, &mut offsets);
    let frame_offsets: Vec<FrameOffset> = offsets.into_iter().map(FrameOffset).collect();
    let descriptor = guard.publish_struct_descriptor(ty, layout.size, &frame_offsets);

    let obj = heap.alloc_object(OBJECT_HEADER_SIZE + size, descriptor)?;
    // SAFETY: `obj` is a freshly reserved object with `size` payload bytes; `deserialize_into`
    // writes the flat value there and boxes any nested vectors in `heap`.
    unsafe { deserialize_into(guard, heap, ty, blob, obj.as_ptr()) }.ok()?;
    Some(obj)
}

/// Collects the byte offsets (within the payload) of 8-byte heap-pointer slots, matching what
/// MonoMove's lowering computes for an object descriptor.
fn collect_pointer_offsets(
    guard: &ExecutionGuard,
    layout: &ValueLayout,
    base: u32,
    out: &mut Vec<u32>,
) {
    if layout.has_no_pointers_no_padding() {
        return;
    }
    match &layout.kind {
        LayoutKind::Vector { .. }
        | LayoutKind::Function
        | LayoutKind::Ref
        | LayoutKind::FrozenEnum { .. } => out.push(base),
        LayoutKind::Struct { fields } => {
            for field in fields.iter() {
                if let Some(sub) = guard.layout(field.id) {
                    collect_pointer_offsets(guard, sub, base + field.offset, out);
                }
            }
        },
        LayoutKind::Bool
        | LayoutKind::UnsignedInt
        | LayoutKind::SignedInt
        | LayoutKind::Address => {},
    }
}
