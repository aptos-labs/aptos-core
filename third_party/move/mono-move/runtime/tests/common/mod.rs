// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Test helpers shared by the global-storage integration tests.
//!
//! Each test binary picks the subset of helpers it needs; Rust's
//! per-binary dead-code analysis flags the rest. Suppress the
//! noise here since the warning is uninformative — every helper is
//! used by *some* test binary.
#![allow(dead_code)]

use mono_move_core::{
    align::{checked_align_max, MAX_ALIGN},
    storage::resource_provider::InMemoryStorageKey,
    types::InternedType,
    DescriptorId, ResourceProvider, ResourceProviderError, StorageRead, OBJECT_HEADER_SIZE,
};
use mono_move_runtime::write_object_header;
use move_core_types::account_address::AccountAddress;
use std::{cell::RefCell, collections::HashMap, ptr::NonNull};

/// Builds an interned module id for hand-built test functions.
#[macro_export]
macro_rules! program_module_id {
    ($name:literal) => {{
        static MODULE_ID: ::mono_move_core::interner::ModuleId =
            ::mono_move_core::interner::ModuleId::new(
                ::move_core_types::account_address::AccountAddress::ONE,
                ::mono_move_alloc::GlobalArenaPtr::from_static($name),
            );
        ::mono_move_alloc::GlobalArenaPtr::from_static(&MODULE_ID)
    }};
}

/// In-memory [`ResourceProvider`] for tests. Owns the resource
/// bytes in `Box<[u64]>` buffers (giving `MAX_ALIGN` alignment and
/// stable addresses); `install_global` writes a leading object
/// header so `load_global` can hand out a valid object pointer.
///
/// `anchors` holds owned sub-allocations (e.g., a child vector
/// pointed to by a struct field) that are not keyed in `entries`
/// but must outlive the resource. Use [`install_anchor`] to add
/// one and embed its returned pointer into a parent payload.
pub struct InMemoryResources {
    /// Map keys to the data-region pointer of an allocation in
    /// `anchors`. The allocation outlives `self`; the pointer is
    /// safe to hand to `get_resource`.
    entries: RefCell<HashMap<InMemoryStorageKey, *const u8>>,
    anchors: RefCell<Vec<Box<[u64]>>>,
}

impl InMemoryResources {
    pub fn new() -> Self {
        Self {
            entries: RefCell::new(HashMap::new()),
            anchors: RefCell::new(Vec::new()),
        }
    }

    /// Install an anchored sub-allocation. Writes an object header
    /// at the start of a fresh `Box<[u64]>`, copies `payload` into
    /// the data region, and keeps the box alive for the lifetime
    /// of `self`. Returns the data-region pointer the caller can
    /// embed into a parent payload (e.g., a struct field).
    pub fn install_anchor(&self, descriptor_id: DescriptorId, payload: &[u8]) -> *mut u8 {
        let total = checked_align_max(OBJECT_HEADER_SIZE + payload.len())
            .expect("install_anchor: payload size overflows after alignment");
        debug_assert_eq!(MAX_ALIGN, 8, "Box<[u64]> alignment matches MAX_ALIGN");
        let words = total / MAX_ALIGN;
        let mut storage = vec![0u64; words].into_boxed_slice();
        // SAFETY: storage spans `total` bytes; `obj_ptr` is in-bounds
        // and the header at `obj_ptr - OBJECT_HEADER_SIZE` sits
        // inside the storage buffer.
        let obj_ptr = unsafe { (storage.as_mut_ptr() as *mut u8).add(OBJECT_HEADER_SIZE) };
        unsafe { write_object_header(obj_ptr, descriptor_id, total as u32) };
        if !payload.is_empty() {
            // SAFETY: source and destination spans don't overlap.
            unsafe { std::ptr::copy_nonoverlapping(payload.as_ptr(), obj_ptr, payload.len()) };
        }
        self.anchors.borrow_mut().push(storage);
        obj_ptr
    }

    /// Install a resource at `(addr, ty)`. Returns the data-region
    /// pointer that subsequent `get_resource` calls hand out.
    pub fn install_global(
        &self,
        addr: AccountAddress,
        ty: InternedType,
        descriptor_id: DescriptorId,
        payload: &[u8],
    ) -> *const u8 {
        let obj_ptr = self.install_anchor(descriptor_id, payload);
        self.entries_install(addr, ty, obj_ptr);
        obj_ptr as *const u8
    }

    /// Register `ptr` (typically from a prior [`install_anchor`])
    /// as the resource at `(addr, ty)`. Use this when a resource
    /// graph spans multiple anchored allocations.
    pub fn entries_install(&self, addr: AccountAddress, ty: InternedType, ptr: *const u8) {
        self.entries
            .borrow_mut()
            .insert(InMemoryStorageKey::resource(addr, ty), ptr);
    }
}

impl ResourceProvider for InMemoryResources {
    fn get_resource(&self, key: &InMemoryStorageKey) -> Result<StorageRead, ResourceProviderError> {
        Ok(match self.entries.borrow().get(key) {
            Some(&ptr) => {
                // SAFETY: pointer came from `install_anchor`, which
                // returned the data-region start of a live anchored
                // allocation. Non-null by construction.
                let ptr = unsafe { NonNull::new_unchecked(ptr as *mut u8) };
                StorageRead::ExternalHeap { ptr, version: 0 }
            },
            None => StorageRead::DoesNotExist,
        })
    }
}
