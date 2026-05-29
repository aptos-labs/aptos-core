// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Low-level memory utilities: aligned buffer and raw pointer helpers.

use crate::VEC_DATA_OFFSET;
use mono_move_alloc::GlobalArenaPtr;
use mono_move_core::{
    types::{InternedType, Type},
    MAX_ALIGN,
};
use std::{
    alloc::{self, Layout},
    ptr::NonNull,
};

// ---------------------------------------------------------------------------
// Aligned buffer — owns a zeroed, [`MAX_ALIGN`]-aligned allocation
// ---------------------------------------------------------------------------

pub struct MemoryRegion {
    ptr: *mut u8,
    layout: Layout,
}

impl MemoryRegion {
    /// Allocates a zeroed, [`MAX_ALIGN`]-aligned memory region of the given size.
    ///
    /// OOM is handled by aborting via `handle_alloc_error`.
    pub fn new(size: usize) -> Self {
        debug_assert!(size > 0);
        let layout = Layout::from_size_align(size, MAX_ALIGN).expect("invalid memory layout");
        // SAFETY: layout is valid (power-of-two alignment) and `alloc_zeroed` handles
        // zero-size layouts per the GlobalAlloc contract. Null is checked below.
        let ptr = unsafe { alloc::alloc_zeroed(layout) };
        if ptr.is_null() {
            alloc::handle_alloc_error(layout);
        }
        Self { ptr, layout }
    }

    #[inline(always)]
    pub fn as_ptr(&self) -> *mut u8 {
        self.ptr
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.layout.size()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.layout.size() == 0
    }
}

impl Drop for MemoryRegion {
    fn drop(&mut self) {
        unsafe { alloc::dealloc(self.ptr, self.layout) };
    }
}

// ---------------------------------------------------------------------------
// Raw pointer helpers
// ---------------------------------------------------------------------------

/// # Safety
/// `base.add(byte_offset)` must be valid and point to an initialized `u8`.
#[inline(always)]
pub unsafe fn read_u8(base: *const u8, byte_offset: impl Into<usize>) -> u8 {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { base.add(byte_offset.into()).read() }
}

/// # Safety
/// `base.add(byte_offset)` must be valid, aligned, and point to an initialized `u64`.
#[inline(always)]
pub unsafe fn read_u64(base: *const u8, byte_offset: impl Into<usize>) -> u64 {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { (base.add(byte_offset.into()) as *const u64).read() }
}

/// # Safety
/// `base.add(byte_offset)` must be valid, aligned, and writable for a `u64`.
#[inline(always)]
pub unsafe fn write_u64(base: *mut u8, byte_offset: impl Into<usize>, val: u64) {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { (base.add(byte_offset.into()) as *mut u64).write(val) }
}

/// # Safety
/// `base.add(byte_offset)` must be valid, aligned, and point to an initialized pointer.
#[inline(always)]
pub unsafe fn read_ptr(base: *const u8, byte_offset: impl Into<usize>) -> *mut u8 {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { (base.add(byte_offset.into()) as *const *mut u8).read() }
}

/// # Safety
/// `base.add(byte_offset)` must be valid, aligned, and writable for a pointer.
#[inline(always)]
pub unsafe fn write_ptr(base: *mut u8, byte_offset: impl Into<usize>, ptr: *const u8) {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { (base.add(byte_offset.into()) as *mut *const u8).write(ptr) }
}

/// Byte offset of the scalar `offset` half within a 16-byte fat pointer; the
/// `base` half occupies the first 8 bytes.
const FAT_PTR_OFFSET_HALF: usize = 8;

/// Read a 16-byte fat pointer `(base, offset)` whose base half starts at
/// `byte_offset`.
///
/// # Safety
/// `base.add(byte_offset)` must be valid, aligned, and point to an initialized
/// 16-byte fat pointer.
#[inline(always)]
pub unsafe fn read_fat_ptr(base: *const u8, byte_offset: impl Into<usize>) -> (*mut u8, u64) {
    let byte_offset = byte_offset.into();
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe {
        (
            read_ptr(base, byte_offset),
            read_u64(base, byte_offset + FAT_PTR_OFFSET_HALF),
        )
    }
}

/// Write a 16-byte fat pointer `(ptr, offset)` whose base half starts at
/// `byte_offset`.
///
/// # Safety
/// `base.add(byte_offset)` must be valid, aligned, and writable for a 16-byte
/// fat pointer.
#[inline(always)]
pub unsafe fn write_fat_ptr(
    base: *mut u8,
    byte_offset: impl Into<usize>,
    ptr: *const u8,
    offset: u64,
) {
    let byte_offset = byte_offset.into();
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe {
        write_ptr(base, byte_offset, ptr);
        write_u64(base, byte_offset + FAT_PTR_OFFSET_HALF, offset);
    }
}

/// # Safety
/// `base.add(byte_offset)` must be valid, aligned, and point to an initialized `u32`.
#[inline(always)]
pub unsafe fn read_u32(base: *const u8, byte_offset: impl Into<usize>) -> u32 {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { (base.add(byte_offset.into()) as *const u32).read() }
}

/// # Safety
/// `base.add(byte_offset)` must be valid, aligned, and writable for a `u32`.
#[inline(always)]
pub unsafe fn write_u32(base: *mut u8, byte_offset: impl Into<usize>, val: u32) {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { (base.add(byte_offset.into()) as *mut u32).write(val) }
}

/// Pointer to the `idx`-th element inside a vector's data region.
///
/// # Safety
/// The resulting pointer must be within the vector's allocated data region.
#[inline(always)]
pub unsafe fn vec_elem_ptr(vec_ptr: *const u8, idx: u64, elem_size: u32) -> *const u8 {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { vec_ptr.add(VEC_DATA_OFFSET + idx as usize * elem_size as usize) }
}

// ---------------------------------------------------------------------------
// Heap object header access (negative offsets)
// ---------------------------------------------------------------------------
//
// A heap object pointer (`obj_ptr`) points at the first byte of the object's
// data region. The 8-byte header is a single `InternedType` pointer sitting in
// the 8 bytes immediately preceding `obj_ptr` (offset -8). The runtime treats
// it as an opaque key: it never dereferences the type, only resolves it to a
// `DescriptorId` through `DescriptorProvider::descriptor_id_for_type`. A null
// header word marks a forwarded object during GC (an `InternedType` is never
// null). When `MAX_ALIGN > 8`, the allocator reserves
// `OBJECT_HEADER_SIZE = MAX_ALIGN` bytes before each data region so that the
// data start stays MAX_ALIGN-aligned; the type word always lives at the last 8
// bytes of that reservation (adjacent to the data, and the negative offset
// stays invariant across MAX_ALIGN values).

/// Byte offset of the interned-type header word from `obj_ptr`. Always `-8`.
const HEADER_TYPE_NEG_OFFSET: isize = -8;

/// Read the interned-type header word. Returns [`None`] when the word is null,
/// which marks a forwarded object during GC.
///
/// # Safety
/// `obj_ptr` must point to the data region of a valid heap object whose
/// header lies at `obj_ptr - 8`.
#[inline(always)]
pub(crate) unsafe fn read_header_type(obj_ptr: *const u8) -> Option<InternedType> {
    // SAFETY: caller must uphold the documented pointer requirements.
    let raw = unsafe { (obj_ptr.offset(HEADER_TYPE_NEG_OFFSET) as *const *mut Type).read() };
    // SAFETY: a non-null word was written by `write_header_type` from a live
    // interned type pointer; reconstructing it is sound under the same
    // arena-liveness contract the caller already relies on.
    NonNull::new(raw).map(|p| unsafe { GlobalArenaPtr::from_non_null(p) })
}

/// True if the object's header word is null — the forwarded marker.
///
/// # Safety
/// `obj_ptr` must point to the data region of a valid heap object whose
/// header lies at `obj_ptr - 8`.
#[inline(always)]
pub(crate) unsafe fn is_forwarded(obj_ptr: *const u8) -> bool {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { (obj_ptr.offset(HEADER_TYPE_NEG_OFFSET) as *const usize).read() == 0 }
}

/// Write the interned-type header word for the object whose data region starts
/// at the given pointer.
///
/// # Safety
/// Pointer points to the data region of a valid heap object with header start
/// at [`HEADER_TYPE_NEG_OFFSET`].
#[inline(always)]
pub unsafe fn write_header_type(obj_ptr: *mut u8, ty: InternedType) {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { (obj_ptr.offset(HEADER_TYPE_NEG_OFFSET) as *mut *const Type).write(ty.as_raw_ptr()) }
}

/// Mark an object as forwarded by writing a null header word. The forwarding
/// pointer is written separately into the data region via [`write_forwarding`].
///
/// # Safety
/// `obj_ptr` must point to the data region of a writable heap object whose
/// header lies at `obj_ptr - 8`.
#[inline(always)]
pub(crate) unsafe fn write_forwarded_marker(obj_ptr: *mut u8) {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { (obj_ptr.offset(HEADER_TYPE_NEG_OFFSET) as *mut usize).write(0) }
}

/// Read the forwarding pointer that the GC writes into a forwarded-from-space
/// object's first 8 data bytes.
///
/// # Safety
/// `obj_ptr` must point to the data region of a forwarded heap object (i.e.,
/// `is_forwarded(obj_ptr)`), with at least 8 bytes of data region.
#[inline(always)]
pub(crate) unsafe fn read_forwarding(obj_ptr: *const u8) -> *mut u8 {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { (obj_ptr as *const *mut u8).read() }
}

/// Write a forwarding pointer into a from-space object's first 8 data bytes.
/// Used together with [`write_forwarded_marker`] to mark an object as
/// forwarded during GC.
///
/// # Safety
/// `obj_ptr` must point to the data region of a writable heap object with
/// at least 8 bytes of data region.
#[inline(always)]
pub(crate) unsafe fn write_forwarding(obj_ptr: *mut u8, new_ptr: *mut u8) {
    // SAFETY: caller must uphold the documented pointer requirements.
    unsafe { (obj_ptr as *mut *mut u8).write(new_ptr) }
}
