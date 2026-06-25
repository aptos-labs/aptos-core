// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! A [`ResourceProvider`] backed by the captured read-set. Each resource is materialized
//! (BCS → flat) into a long-lived heap arena on first access and served as a pointer thereafter.
//! Materialization is lazy because a resource type's layout is only published once the function
//! that accesses it has been lowered.

use crate::{data::StoredResource, v2::intern_type_tag};
use mono_move_core::{
    storage::resource_provider::{
        InMemoryStorageKey, ResourceProvider, ResourceProviderError, StorageRead,
    },
    types::InternedType,
    FrameOffset, LayoutKind, LayoutProvider, ValueLayout, OBJECT_HEADER_SIZE,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{deserialize_into, Heap};
use move_core_types::language_storage::TypeTag;
use std::{cell::RefCell, collections::HashMap, ptr::NonNull};

/// Holds the read-set's resource blobs up front and the arena of materialized objects.
pub struct ReadSetResourceProvider<'guard, 'ctx> {
    guard: &'guard ExecutionGuard<'ctx>,
    /// BCS bytes of each read-set resource, keyed the same way MonoMove keys a `borrow_global`.
    resources: HashMap<InMemoryStorageKey, Vec<u8>>,
    materialized: RefCell<Materialized>,
}

struct Materialized {
    /// Long-lived arena holding the flat representation of materialized resources. Must outlive the
    /// interpreter; never reset between runs.
    heap: Heap,
    cache: HashMap<InMemoryStorageKey, NonNull<u8>>,
}

impl<'guard, 'ctx> ReadSetResourceProvider<'guard, 'ctx> {
    pub fn new(
        guard: &'guard ExecutionGuard<'ctx>,
        resources: &[StoredResource],
        heap_size: usize,
    ) -> Self {
        let mut blobs = HashMap::new();
        for resource in resources {
            let tag = TypeTag::Struct(Box::new(resource.struct_tag.clone()));
            // Interning needs no layouts; it just produces the key MonoMove will look up.
            if let Ok(ty) = intern_type_tag(guard, &tag) {
                blobs.insert(
                    InMemoryStorageKey::resource(resource.address, ty),
                    resource.blob.clone(),
                );
            }
        }
        Self {
            guard,
            resources: blobs,
            materialized: RefCell::new(Materialized {
                heap: Heap::new(heap_size),
                cache: HashMap::new(),
            }),
        }
    }
}

impl ResourceProvider for ReadSetResourceProvider<'_, '_> {
    fn get_resource(&self, key: &InMemoryStorageKey) -> Result<StorageRead, ResourceProviderError> {
        // Cache hit?
        {
            let materialized = self.materialized.borrow();
            if let Some(&ptr) = materialized.cache.get(key) {
                return Ok(StorageRead::ExternalHeap { ptr, version: 0 });
            }
        }

        let Some(blob) = self.resources.get(key) else {
            return Ok(StorageRead::DoesNotExist);
        };
        let InMemoryStorageKey::Resource { ty, .. } = key else {
            return Ok(StorageRead::DoesNotExist);
        };

        let mut materialized = self.materialized.borrow_mut();
        match materialize_one(&mut materialized.heap, self.guard, *ty, blob) {
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
