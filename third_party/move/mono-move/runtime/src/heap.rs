// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Bump-allocated heap with a copying garbage collector (Cheney's algorithm).
//!
//! All heap methods are implemented on `InterpreterContext` so they can
//! directly access interpreter state (frame pointer, functions, descriptors)
//! without passing it through arguments.

use crate::{
    interpreter::InterpreterContext,
    memory::{read_ptr, read_u32, read_u64, write_ptr, write_u32, write_u64, MemoryRegion},
    types::{
        ObjectDescriptor, FORWARDED_MARKER, HEADER_DESCRIPTOR_OFFSET, HEADER_SIZE_OFFSET,
        META_SAVED_FP_OFFSET, META_SAVED_FUNC_PTR_OFFSET, VEC_DATA_OFFSET, VEC_LENGTH_OFFSET,
    },
};
use anyhow::{bail, Result};
use mono_move_core::{
    DescriptorId, Function, ENUM_DATA_OFFSET, ENUM_TAG_OFFSET, FRAME_METADATA_SIZE,
    OBJECT_HEADER_SIZE, STRUCT_DATA_OFFSET,
};
use std::ptr::NonNull;

const MAX_SINGLE_ALLOCATION_SIZE: usize = 10 * 1024 * 1024; // 10 MiB

/// Round `size` up to the next multiple of 8.
///
/// NOTE: the hard-coded 8-byte alignment is a placeholder. A more principled
/// scheme should be revisited once the object layout is more settled.
fn align8(size: usize) -> usize {
    (size + 7) & !7
}

// ---------------------------------------------------------------------------
// Heap (plain data)
// ---------------------------------------------------------------------------

pub struct Heap {
    pub(crate) buffer: MemoryRegion,
    pub(crate) bump_ptr: *mut u8,
    pub(crate) gc_count: usize,
}

impl Heap {
    pub fn new(size: usize) -> Self {
        let buffer = MemoryRegion::new(size);
        Self {
            bump_ptr: buffer.as_ptr(),
            buffer,
            gc_count: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Heap operations on InterpreterContext
// ---------------------------------------------------------------------------

impl InterpreterContext<'_> {
    /// Allocate `size` bytes (8-byte aligned) on the heap.
    /// Triggers GC if the bump allocator is full; fails on OOM after GC.
    pub(crate) fn heap_alloc(&mut self, size: usize) -> Result<*mut u8> {
        if size == 0 {
            bail!("heap_alloc: size must not be zero");
        }
        if size > MAX_SINGLE_ALLOCATION_SIZE {
            bail!(
                "heap_alloc: size {} exceeds maximum single allocation size",
                size
            );
        }
        let aligned = align8(size);

        unsafe {
            let heap_end = self.heap.buffer.as_ptr().add(self.heap.buffer.len());
            if self.heap.bump_ptr.add(aligned) > heap_end {
                self.gc_collect()?;
                let heap_end = self.heap.buffer.as_ptr().add(self.heap.buffer.len());
                if self.heap.bump_ptr.add(aligned) > heap_end {
                    bail!("out of heap memory after GC (requested {} bytes)", size);
                }
            }

            let ptr = self.heap.bump_ptr;
            self.heap.bump_ptr = ptr.add(aligned);
            std::ptr::write_bytes(ptr, 0, aligned);
            Ok(ptr)
        }
    }

    /// Allocate a new vector object on the heap with the given parameters.
    pub(crate) fn alloc_vec(
        &mut self,
        descriptor_id: DescriptorId,
        elem_size: u32,
        capacity: u64,
    ) -> Result<*mut u8> {
        let total_size = (capacity as usize)
            .checked_mul(elem_size as usize)
            .and_then(|v| v.checked_add(VEC_DATA_OFFSET))
            .ok_or_else(|| anyhow::anyhow!("alloc_vec: size overflow"))?;
        let aligned_size = align8(total_size);
        let ptr = self.heap_alloc(total_size)?;
        unsafe {
            write_u32(ptr, HEADER_DESCRIPTOR_OFFSET, descriptor_id.as_u32());
            write_u32(ptr, HEADER_SIZE_OFFSET, aligned_size as u32);
            write_u64(ptr, VEC_LENGTH_OFFSET, 0);
        }
        Ok(ptr)
    }

    /// Allocate a new zeroed heap object (struct or enum). Size comes from the
    /// descriptor at `descriptor_id`.
    pub(crate) fn alloc_obj(&mut self, descriptor_id: DescriptorId) -> Result<*mut u8> {
        let payload_size = match &self.descriptors[descriptor_id.as_usize()] {
            ObjectDescriptor::Struct { size, .. } => *size as usize,
            ObjectDescriptor::Enum { size, .. } => *size as usize,
            ObjectDescriptor::Trivial | ObjectDescriptor::Vector { .. } => bail!(
                "alloc_obj called with non-Struct/Enum descriptor {}",
                descriptor_id
            ),
        };
        let total_size = OBJECT_HEADER_SIZE + payload_size;
        let aligned_size = align8(total_size);
        let ptr = self.heap_alloc(total_size)?;
        unsafe {
            write_u32(ptr, HEADER_DESCRIPTOR_OFFSET, descriptor_id.as_u32());
            write_u32(ptr, HEADER_SIZE_OFFSET, aligned_size as u32);
        }
        Ok(ptr)
    }

    /// Grow a vector to at least `required_cap` elements, accessed through
    /// a fat pointer reference at `fp + vec_ref_offset`. The vector pointer
    /// is written back through the reference; returns the new object pointer.
    ///
    /// # Safety
    ///
    /// `fp` must point to a valid frame. `vec_ref_offset` must be the byte
    /// offset of a 16-byte fat pointer `(base, offset)` whose target holds
    /// the current vector heap pointer. `alloc_vec` may trigger GC which
    /// relocates objects; the fat pointer's base in `pointer_offsets` and the
    /// vector pointer in the struct's `pointer_offsets` are updated by the GC.
    /// We re-read through the fat pointer after allocation.
    pub(crate) fn grow_vec_ref(
        &mut self,
        fp: *mut u8,
        vec_ref_offset: usize,
        elem_size: u32,
        required_cap: u64,
    ) -> Result<*mut u8> {
        unsafe {
            let base = read_ptr(fp, vec_ref_offset);
            let off = read_u64(fp, vec_ref_offset + 8) as usize;
            let old_ptr = read_ptr(base, off);

            let old_len = read_u64(old_ptr, VEC_LENGTH_OFFSET);
            let old_size = read_u32(old_ptr, HEADER_SIZE_OFFSET) as usize;
            let old_cap = ((old_size - VEC_DATA_OFFSET) / elem_size as usize) as u64;
            let descriptor_id = DescriptorId(read_u32(old_ptr, HEADER_DESCRIPTOR_OFFSET));

            let mut new_cap = if old_cap == 0 { 4 } else { old_cap * 2 };
            if new_cap < required_cap {
                new_cap = required_cap;
            }

            // alloc_vec may trigger GC. Re-read through the fat pointer afterward.
            let new_ptr = self.alloc_vec(descriptor_id, elem_size, new_cap)?;
            let base = read_ptr(fp, vec_ref_offset);
            let off = read_u64(fp, vec_ref_offset + 8) as usize;
            let old_ptr = read_ptr(base, off);

            let byte_count = old_len as usize * elem_size as usize;
            if byte_count > 0 {
                std::ptr::copy_nonoverlapping(
                    old_ptr.add(VEC_DATA_OFFSET),
                    new_ptr.add(VEC_DATA_OFFSET),
                    byte_count,
                );
            }
            write_u64(new_ptr, VEC_LENGTH_OFFSET, old_len);

            // Write new pointer back through the reference.
            write_ptr(base, off, new_ptr);
            Ok(new_ptr)
        }
    }

    // -----------------------------------------------------------------------
    // Garbage collection
    // -----------------------------------------------------------------------

    /// Run Cheney's copying GC. Walks the call stack to find roots via
    /// per-function pointer slot lists, then does a breadth-first copy of
    /// all reachable objects.
    ///
    /// # Safety assumptions
    ///
    /// Correctness relies on the following invariants maintained by the
    /// interpreter and the micro-op verifier:
    ///
    /// - **Frame metadata integrity**: each frame's saved `fp`, `func_ptr`,
    ///   and `pc` are written by `CallFunc`/`Return` and never modified by
    ///   user-visible micro-ops. A corrupted saved `fp` leads to
    ///   out-of-bounds stack reads (UB).
    /// - **Pointer-slot accuracy**: `Function::pointer_offsets` lists every
    ///   frame offset that may hold a live heap pointer, and *only* those
    ///   offsets. Missing entries → dangling pointers after GC; extra
    ///   entries → non-pointer data reinterpreted as a pointer (UB).
    /// - **Object header integrity**: the `descriptor_id` and `size` fields
    ///   in every heap object header are set by the allocator and never
    ///   overwritten by user code. Corrupted headers → wrong copy size or
    ///   wrong reference tracing (UB).
    pub(crate) fn gc_collect(&mut self) -> Result<()> {
        self.heap.gc_count += 1;

        let to_space = MemoryRegion::new(self.heap.buffer.len());
        let mut free_ptr = to_space.as_ptr();

        // Phase 1: scan roots from the call stack.
        let mut fp = self.frame_ptr;
        let mut func_ptr = self.current_func;

        loop {
            // SAFETY: func_ptr is a valid, non-null pointer — set from
            // `self.functions[]` or from `CallLocalFunc`, and saved/restored
            // via frame metadata. pointer_offsets is an arena pointer valid
            // for the lifetime of the executable. `fp.sub` retrieves saved
            // metadata written by the call protocol.
            let pointer_offsets = unsafe { func_ptr.as_ref().pointer_offsets.as_ref_unchecked() };
            unsafe {
                for &offset in pointer_offsets {
                    let old_ptr = read_ptr(fp, offset);
                    if !old_ptr.is_null() && self.is_heap_ptr(old_ptr) {
                        let new_ptr = self.gc_copy_object(old_ptr, &mut free_ptr);
                        write_ptr(fp, offset, new_ptr);
                    }
                }

                let meta = fp.sub(FRAME_METADATA_SIZE);
                let saved_func_ptr = read_ptr(meta, META_SAVED_FUNC_PTR_OFFSET) as *const Function;
                if saved_func_ptr.is_null() {
                    break;
                }
                fp = read_ptr(meta, META_SAVED_FP_OFFSET);
                // SAFETY: saved_func_ptr is non-null — we checked for the
                // null sentinel above and would have broken out of the loop.
                func_ptr = NonNull::new_unchecked(saved_func_ptr as *mut Function);
            }
        }

        // Phase 2: Cheney-style breadth-first scan of copied objects.
        let mut scan_ptr = to_space.as_ptr();
        while (scan_ptr as usize) < (free_ptr as usize) {
            // SAFETY: scan_ptr advances through to-space by each object's
            // aligned size. Object headers were copied verbatim by
            // gc_copy_object, so descriptor_id and size are valid as long
            // as the object-header-integrity invariant holds (see above).
            unsafe {
                let descriptor_id = read_u32(scan_ptr, HEADER_DESCRIPTOR_OFFSET);
                let obj_size = read_u32(scan_ptr, HEADER_SIZE_OFFSET) as usize;

                if obj_size == 0 || obj_size != align8(obj_size) {
                    bail!(
                        "GC scan: invalid object size {} (expected non-zero, 8-byte aligned)",
                        obj_size
                    );
                }

                if descriptor_id == FORWARDED_MARKER {
                    bail!("GC found forwarding marker in to-space (invariant violation)");
                }
                self.gc_scan_object(scan_ptr, descriptor_id, &mut free_ptr);

                scan_ptr = scan_ptr.add(obj_size);
            }
        }

        // Phase 3: swap — drop old heap, adopt new one.
        self.heap.buffer = to_space;
        self.heap.bump_ptr = free_ptr;
        Ok(())
    }

    fn is_heap_ptr(&self, ptr: *const u8) -> bool {
        let start = self.heap.buffer.as_ptr() as usize;
        let end = start + self.heap.buffer.len();
        let p = ptr as usize;
        p >= start && p < end
    }

    /// Copy a single object from the old heap into to-space (at `*free_ptr`),
    /// writing a forwarding pointer in the old location. If the object is
    /// already forwarded, just return the forwarding address.
    ///
    /// # Safety
    ///
    /// - `old_ptr` must point to the header of a live object in from-space
    ///   (the current `self.heap.buffer`). Its `descriptor_id` and `size`
    ///   header fields must be valid (see object-header-integrity invariant
    ///   on `gc_collect`).
    /// - `free_ptr` must point into to-space with at least `obj_size`
    ///   bytes remaining (already 8-byte aligned). The caller is responsible for
    ///   ensuring to-space is large enough (same size as from-space).
    /// - The from-space object must not have been partially overwritten
    ///   except by a prior call to this function (which installs a
    ///   forwarding marker).
    fn gc_copy_object(&mut self, old_ptr: *mut u8, free_ptr: &mut *mut u8) -> *mut u8 {
        unsafe {
            let descriptor_id = read_u32(old_ptr, HEADER_DESCRIPTOR_OFFSET);

            if descriptor_id == FORWARDED_MARKER {
                return read_ptr(old_ptr, OBJECT_HEADER_SIZE);
            }

            let obj_size = read_u32(old_ptr, HEADER_SIZE_OFFSET) as usize;
            debug_assert!(
                obj_size > 0 && obj_size == align8(obj_size),
                "gc_copy_object: invalid object size {}",
                obj_size
            );
            let new_ptr = *free_ptr;

            std::ptr::copy_nonoverlapping(old_ptr, new_ptr, obj_size);
            *free_ptr = new_ptr.add(obj_size);

            write_u32(old_ptr, HEADER_DESCRIPTOR_OFFSET, FORWARDED_MARKER);
            write_ptr(old_ptr, OBJECT_HEADER_SIZE, new_ptr);

            new_ptr
        }
    }

    /// Scan a copied object in to-space for internal heap references and
    /// copy the referenced objects as well (Cheney forwarding).
    ///
    /// # Safety
    ///
    /// - `obj_ptr` must point to a valid, fully-copied object in to-space.
    /// - `descriptor_id` must match the object's actual type (read from its
    ///   header by the caller).
    /// - The `ObjectDescriptor` at `descriptor_id` must accurately describe
    ///   the reference layout of the object — incorrect `pointer_offsets` or
    ///   `elem_pointer_offsets` will cause the GC to follow non-pointer data (UB).
    /// - `free_ptr` must point to the next free byte in to-space with enough
    ///   room for any objects that will be copied.
    fn gc_scan_object(&mut self, obj_ptr: *mut u8, descriptor_id: u32, free_ptr: &mut *mut u8) {
        debug_assert!(
            (descriptor_id as usize) < self.descriptors.len(),
            "gc_scan_object: descriptor_id {} out of bounds (have {} descriptors)",
            descriptor_id,
            self.descriptors.len()
        );
        let desc = if (descriptor_id as usize) < self.descriptors.len() {
            &self.descriptors[descriptor_id as usize]
        } else {
            return;
        };

        match desc {
            ObjectDescriptor::Trivial => {},
            ObjectDescriptor::Vector {
                elem_size,
                elem_pointer_offsets,
            } => {
                debug_assert!(*elem_size > 0, "elem_size must not be zero");
                if elem_pointer_offsets.is_empty() {
                    return;
                }
                unsafe {
                    let length = read_u64(obj_ptr, VEC_LENGTH_OFFSET) as usize;
                    let data_start = obj_ptr.add(VEC_DATA_OFFSET);

                    for i in 0..length {
                        let elem_base = data_start.add(i * (*elem_size as usize));
                        for &ptr_off in elem_pointer_offsets {
                            let old_ptr = read_ptr(elem_base, ptr_off as usize);
                            if !old_ptr.is_null() && self.is_heap_ptr(old_ptr) {
                                let new_ptr = self.gc_copy_object(old_ptr, free_ptr);
                                write_ptr(elem_base, ptr_off as usize, new_ptr);
                            }
                        }
                    }
                }
            },
            ObjectDescriptor::Struct {
                pointer_offsets, ..
            } => {
                if pointer_offsets.is_empty() {
                    return;
                }
                unsafe {
                    for &off in pointer_offsets {
                        let old_ptr = read_ptr(obj_ptr, STRUCT_DATA_OFFSET + off as usize);
                        if !old_ptr.is_null() && self.is_heap_ptr(old_ptr) {
                            let new_ptr = self.gc_copy_object(old_ptr, free_ptr);
                            write_ptr(obj_ptr, STRUCT_DATA_OFFSET + off as usize, new_ptr);
                        }
                    }
                }
            },
            ObjectDescriptor::Enum {
                variant_pointer_offsets,
                ..
            } => unsafe {
                let tag = read_u64(obj_ptr, ENUM_TAG_OFFSET) as usize;
                if tag >= variant_pointer_offsets.len() {
                    return;
                }
                let pointer_offsets = &variant_pointer_offsets[tag];
                if pointer_offsets.is_empty() {
                    return;
                }
                for &off in pointer_offsets {
                    let old_ptr = read_ptr(obj_ptr, ENUM_DATA_OFFSET + off as usize);
                    if !old_ptr.is_null() && self.is_heap_ptr(old_ptr) {
                        let new_ptr = self.gc_copy_object(old_ptr, free_ptr);
                        write_ptr(obj_ptr, ENUM_DATA_OFFSET + off as usize, new_ptr);
                    }
                }
            },
        }
    }
}
