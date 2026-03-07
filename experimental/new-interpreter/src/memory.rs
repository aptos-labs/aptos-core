// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Low-level memory utilities: aligned buffer and raw pointer helpers.

use crate::VEC_DATA_OFFSET;
use std::alloc::{self, Layout};

// ---------------------------------------------------------------------------
// Aligned buffer — owns a zeroed, 8-byte-aligned allocation
// ---------------------------------------------------------------------------

pub struct MemoryRegion {
    ptr: *mut u8,
    layout: Layout,
}

impl MemoryRegion {
    pub fn new(size: usize) -> Self {
        let layout = Layout::from_size_align(size, 8).expect("invalid memory layout");
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
/// `base.add(byte_offset)` must be valid, aligned, and point to an initialized `u64`.
#[inline(always)]
pub unsafe fn read_u64(base: *const u8, byte_offset: usize) -> u64 {
    unsafe { (base.add(byte_offset) as *const u64).read() }
}

/// # Safety
/// `base.add(byte_offset)` must be valid, aligned, and writable for a `u64`.
#[inline(always)]
pub unsafe fn write_u64(base: *mut u8, byte_offset: usize, val: u64) {
    unsafe { (base.add(byte_offset) as *mut u64).write(val) }
}

/// # Safety
/// `base.add(byte_offset)` must be valid, aligned, and point to an initialized pointer.
#[inline(always)]
pub unsafe fn read_ptr(base: *const u8, byte_offset: usize) -> *mut u8 {
    unsafe { (base.add(byte_offset) as *const *mut u8).read() }
}

/// # Safety
/// `base.add(byte_offset)` must be valid, aligned, and writable for a pointer.
#[inline(always)]
pub unsafe fn write_ptr(base: *mut u8, byte_offset: usize, ptr: *const u8) {
    unsafe { (base.add(byte_offset) as *mut *const u8).write(ptr) }
}

/// # Safety
/// `base.add(byte_offset)` must be valid, aligned, and point to an initialized `u32`.
#[inline(always)]
pub unsafe fn read_u32(base: *const u8, byte_offset: usize) -> u32 {
    unsafe { (base.add(byte_offset) as *const u32).read() }
}

/// # Safety
/// `base.add(byte_offset)` must be valid, aligned, and writable for a `u32`.
#[inline(always)]
pub unsafe fn write_u32(base: *mut u8, byte_offset: usize, val: u32) {
    unsafe { (base.add(byte_offset) as *mut u32).write(val) }
}

/// Pointer to the `idx`-th element inside a vector's data region.
///
/// # Safety
/// The resulting pointer must be within the vector's allocated data region.
#[inline(always)]
pub unsafe fn vec_elem_ptr(vec_ptr: *const u8, idx: u64, elem_size: u32) -> *const u8 {
    unsafe { vec_ptr.add(VEC_DATA_OFFSET + idx as usize * elem_size as usize) }
}
