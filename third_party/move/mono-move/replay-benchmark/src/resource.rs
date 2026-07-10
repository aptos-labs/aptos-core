// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! A [`ResourceProvider`] backed by the captured read-set. Each resource is materialized
//! (BCS → flat) into its own heap on first access and served as a pointer (with the heap's
//! `Arc`) thereafter. Materialization is lazy because a resource type's layout is only
//! published once the function that accesses it has been lowered.

use crate::{data::ReadSet, v2::intern_struct_tag};
use anyhow::Result;
use aptos_types::{access_path::Path, state_store::state_key::inner::StateKeyInner};
use mono_move_core::{
    storage::resource_provider::{InMemoryStorageKey, ResourceProviderError},
    types::InternedType,
    FrameOffset, LayoutKind, LayoutProvider, ValueLayout, OBJECT_HEADER_SIZE,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{deserialize_into, Heap, ResourceProvider, StorageRead};
use move_core_types::{account_address::AccountAddress, language_storage::StructTag};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    ptr::NonNull,
    sync::Arc,
};

/// Sizing of a resource's heap: the flat representation of `n` BCS bytes is bounded by a small
/// constant factor, plus slack for vector headers.
const HEAP_BYTES_PER_BLOB_BYTE: usize = 8;
const HEAP_SLACK_BYTES: usize = 4 * 1024;

/// Serves the read-set's resources and table items to MonoMove, materializing each on first access.
pub struct ReadSetResourceProvider<'guard, 'ctx> {
    guard: &'guard ExecutionGuard<'ctx>,
    /// BCS bytes of each resource, keyed by address and interned type.
    resources: HashMap<(AccountAddress, InternedType), Vec<u8>>,
    /// BCS bytes of each table item, keyed by table handle and serialized key.
    table_items: HashMap<(AccountAddress, Vec<u8>), Vec<u8>>,
    /// Materialized values: the flat object pointer and the heap it lives in. Heaps must outlive
    /// the interpreter; never reset between runs.
    materialized: RefCell<HashMap<InMemoryStorageKey, (NonNull<u8>, Arc<Heap>)>>,
}

impl<'guard, 'ctx> ReadSetResourceProvider<'guard, 'ctx> {
    pub fn new(guard: &'guard ExecutionGuard<'ctx>, read_set: &ReadSet) -> Result<Self> {
        let mut resources = HashMap::new();
        let mut table_items = HashMap::new();
        for (state_key, value) in &read_set.data {
            match state_key.inner() {
                StateKeyInner::AccessPath(ap) => match ap.get_path() {
                    // Modules are ignored.
                    Path::Code(_) => {},
                    Path::Resource(struct_tag) => {
                        let ty = intern_struct_tag(guard, &struct_tag)?;
                        resources.insert((ap.address, ty), value.bytes().to_vec());
                    },
                    // A resource group: add each resource in the group individually.
                    Path::ResourceGroup(_) => {
                        let members: BTreeMap<StructTag, Vec<u8>> = bcs::from_bytes(value.bytes())?;
                        for (struct_tag, blob) in members {
                            let ty = intern_struct_tag(guard, &struct_tag)?;
                            resources.insert((ap.address, ty), blob);
                        }
                    },
                },
                StateKeyInner::TableItem { handle, key } => {
                    table_items.insert((handle.0, key.clone()), value.bytes().to_vec());
                },
                // Neither resources nor table items.
                StateKeyInner::Raw(_) | StateKeyInner::TradingNative(_) => {},
            }
        }
        Ok(Self {
            guard,
            resources,
            table_items,
            materialized: RefCell::new(HashMap::new()),
        })
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

impl ResourceProvider for ReadSetResourceProvider<'_, '_> {
    fn get_resource(&self, key: &InMemoryStorageKey) -> Result<StorageRead, ResourceProviderError> {
        // Cache hit?
        {
            let materialized = self.materialized.borrow();
            if let Some((ptr, heap)) = materialized.get(key) {
                return Ok(StorageRead::external_heap_in_storage(*ptr, heap.clone()));
            }
        }

        let Some((blob, ty)) = self.entry(key) else {
            return Ok(StorageRead::does_not_exist_in_storage());
        };

        match materialize_one(self.guard, ty, blob) {
            Some((ptr, heap)) => {
                let heap = Arc::new(heap);
                self.materialized
                    .borrow_mut()
                    .insert(key.clone(), (ptr, heap.clone()));
                Ok(StorageRead::external_heap_in_storage(ptr, heap))
            },
            None => Ok(StorageRead::does_not_exist_in_storage()),
        }
    }
}

/// Materializes one resource of type `ty` from its BCS `blob` into a fresh heap sized for it,
/// returning a pointer to the flat object (header at the preceding bytes) and the heap. Returns
/// `None` if the layout is unavailable (the accessing function wasn't lowered) or the heap is
/// full.
fn materialize_one(
    guard: &ExecutionGuard,
    ty: InternedType,
    blob: &[u8],
) -> Option<(NonNull<u8>, Heap)> {
    let layout = guard.layout_by_ty(ty)?;
    let size = layout.size as usize;

    // The GC descriptor records which payload slots are heap pointers. Lowering already published
    // it for this resource type; `publish_struct_descriptor` is idempotent and returns that one
    // (our offsets are a fallback that is ignored on the fast path).
    let mut offsets = vec![];
    collect_pointer_offsets(guard, layout, 0, &mut offsets);
    let frame_offsets: Vec<FrameOffset> = offsets.into_iter().map(FrameOffset).collect();
    let descriptor = guard.publish_struct_descriptor(ty, layout.size, &frame_offsets);

    let mut heap = Heap::new(
        OBJECT_HEADER_SIZE + size + blob.len() * HEAP_BYTES_PER_BLOB_BYTE + HEAP_SLACK_BYTES,
    );
    let obj = heap.alloc_object(OBJECT_HEADER_SIZE + size, descriptor)?;
    // SAFETY: `obj` is a freshly reserved object with `size` payload bytes; `deserialize_into`
    // writes the flat value there and boxes any nested vectors in `heap`.
    unsafe { deserialize_into(guard, &mut heap, ty, blob, obj.as_ptr()) }.ok()?;
    Some((obj, heap))
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
