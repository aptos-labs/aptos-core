// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! An owned, [`MAX_ALIGN`]-aligned VM memory buffer.

use crate::MAX_ALIGN;
use std::alloc::{self, Layout};

pub struct MemoryRegion {
    ptr: *mut u8,
    layout: Layout,
}

// SAFETY: A region uniquely owns its allocation and holds no thread-affine
// state, so moving that ownership to another thread is sound. Deliberately not
// `Sync`: callers write through `as_ptr` with no synchronization.
unsafe impl Send for MemoryRegion {}

impl MemoryRegion {
    /// Allocates a zeroed, [`MAX_ALIGN`]-aligned memory region of the given size.
    ///
    /// Use this whenever a caller may read a slot before writing it. For memory
    /// that is always written before it is read, prefer [`Self::new_uninit`] to
    /// skip the zeroing.
    ///
    /// OOM is handled by aborting via `handle_alloc_error`.
    pub fn new_zeroed(size: usize) -> Self {
        Self::new::<true>(size)
    }

    /// Allocates an uninitialized, [`MAX_ALIGN`]-aligned memory region of the
    /// given size. The bytes hold arbitrary values, so the region must be
    /// written before it is read.
    ///
    /// # Invariants
    ///
    /// The caller must write every byte before it is read. The region carries
    /// no guarantee about its initial contents; callers must not rely on any
    /// (in particular, must not assume it is zeroed).
    ///
    /// OOM is handled by aborting via `handle_alloc_error`.
    pub fn new_uninit(size: usize) -> Self {
        let region = Self::new::<false>(size);

        // The allocator often hands back fresh, already-zeroed OS pages, so in
        // practice uninitialized memory reads as zeros and code that wrongly
        // relies on zeroing would keep passing. Poison the region in debug
        // builds so the write-before-read contract is exercised in tests and
        // CI. Gated on `not(miri)`: this write initializes the memory, which
        // would otherwise hide genuine uninitialized reads from Miri.
        #[cfg(all(debug_assertions, not(miri)))]
        // SAFETY: `region.ptr` is a valid, `size`-byte allocation just returned
        // by `Self::new` above.
        unsafe {
            std::ptr::write_bytes(region.ptr, 0xAA, size);
        }
        region
    }

    /// Hands a region to a new owner. Whatever bytes are already there carry
    /// over, so the new owner still owes [`Self::new_uninit`]'s
    /// write-before-read contract.
    ///
    /// Debug builds poison. Without this, the region holds plausible data
    /// rather than 0xAA, and a read-before-write bug would go unnoticed.
    pub fn recycle(&mut self) {
        #[cfg(all(debug_assertions, not(miri)))]
        // SAFETY: `ptr` is a valid allocation of `layout.size()` bytes, and
        // `&mut self` rules out a concurrent reader.
        unsafe {
            std::ptr::write_bytes(self.ptr, 0xAA, self.layout.size());
        }
    }

    /// Shared body of [`Self::new_zeroed`] / [`Self::new_uninit`]. `ZEROED`
    /// selects `alloc_zeroed` vs `alloc`; both paths null-check and abort via
    /// `handle_alloc_error` on OOM.
    fn new<const ZEROED: bool>(size: usize) -> Self {
        assert!(size > 0);
        let layout = Layout::from_size_align(size, MAX_ALIGN).expect("invalid memory layout");
        // SAFETY: layout is valid (power-of-two alignment, non-zero size). Null
        // is checked below.
        let ptr = unsafe {
            if ZEROED {
                alloc::alloc_zeroed(layout)
            } else {
                alloc::alloc(layout)
            }
        };
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
