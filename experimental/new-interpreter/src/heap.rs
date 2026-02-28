// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Bump-allocated heap with a copying garbage collector (Cheney's algorithm).
//!
//! All heap methods are implemented on `InterpreterContext` so they can
//! directly access interpreter state (frame pointer, functions, descriptors)
//! without passing it through arguments.

use crate::{
    interpreter::InterpreterContext, read_ptr, read_u32, read_u64, write_ptr, write_u32, write_u64,
    MemoryRegion, ObjectDescriptor, FORWARDED_MARKER, FRAME_METADATA_SIZE, OBJECT_HEADER_SIZE,
    SENTINEL_FUNC_ID, STRUCT_DATA_OFFSET, VEC_CAPACITY_OFFSET, VEC_DATA_OFFSET, VEC_LENGTH_OFFSET,
};
use anyhow::{bail, Result};

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
        let aligned = (size + 7) & !7;

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
        descriptor_id: u16,
        elem_size: u32,
        capacity: u64,
    ) -> Result<*mut u8> {
        let total_size = VEC_DATA_OFFSET + (capacity as usize) * (elem_size as usize);
        let ptr = self.heap_alloc(total_size)?;
        unsafe {
            write_u32(ptr, 0, descriptor_id as u32);
            write_u32(ptr, 4, total_size as u32);
            write_u64(ptr, VEC_LENGTH_OFFSET, 0);
            write_u64(ptr, VEC_CAPACITY_OFFSET, capacity);
        }
        Ok(ptr)
    }

    /// Allocate a new zeroed struct on the heap. Size comes from the `Struct`
    /// descriptor at `descriptor_id`.
    pub(crate) fn alloc_struct(&mut self, descriptor_id: u16) -> Result<*mut u8> {
        let payload_size = match &self.descriptors[descriptor_id as usize] {
            ObjectDescriptor::Struct { size, .. } => *size as usize,
            _ => panic!("alloc_struct called with non-Struct descriptor {}", descriptor_id),
        };
        let total_size = OBJECT_HEADER_SIZE + payload_size;
        let ptr = self.heap_alloc(total_size)?;
        unsafe {
            write_u32(ptr, 0, descriptor_id as u32);
            write_u32(ptr, 4, total_size as u32);
        }
        Ok(ptr)
    }

    /// Grow a vector to at least `required_cap` elements. The stack slot at
    /// `vec_slot` is updated in place; returns the new object pointer.
    pub(crate) fn grow_vec(
        &mut self,
        vec_slot: *mut u64,
        elem_size: u32,
        required_cap: u64,
    ) -> Result<*mut u8> {
        // SAFETY: vec_slot and the heap pointers it references are valid for
        // the duration of this call; alloc_vec may trigger GC which relocates
        // objects but updates vec_slot via stack maps.
        unsafe {
            let old_ptr = vec_slot.read() as *const u8;
            let old_len = read_u64(old_ptr, VEC_LENGTH_OFFSET);
            let old_cap = read_u64(old_ptr, VEC_CAPACITY_OFFSET);
            let descriptor_id = read_u32(old_ptr, 0) as u16;

            let mut new_cap = if old_cap == 0 { 4 } else { old_cap * 2 };
            if new_cap < required_cap {
                new_cap = required_cap;
            }

            // alloc_vec may trigger GC, which moves the old vector and updates
            // vec_slot. Re-read the old pointer from the slot afterward.
            let new_ptr = self.alloc_vec(descriptor_id, elem_size, new_cap)?;
            let old_ptr = vec_slot.read() as *const u8;

            let byte_count = old_len as usize * elem_size as usize;
            if byte_count > 0 {
                std::ptr::copy_nonoverlapping(
                    old_ptr.add(VEC_DATA_OFFSET),
                    new_ptr.add(VEC_DATA_OFFSET),
                    byte_count,
                );
            }
            write_u64(new_ptr, VEC_LENGTH_OFFSET, old_len);
            vec_slot.write(new_ptr as u64);
            Ok(new_ptr)
        }
    }

    // -----------------------------------------------------------------------
    // Garbage collection
    // -----------------------------------------------------------------------

    /// Run Cheney's copying GC. Walks the call stack to find roots via stack
    /// maps, then does a breadth-first copy of all reachable objects.
    pub(crate) fn gc_collect(&mut self) -> Result<()> {
        self.heap.gc_count += 1;

        let to_space = MemoryRegion::new(self.heap.buffer.len());
        let mut free_ptr = to_space.as_ptr();

        // SAFETY: we walk the call stack via saved frame metadata and use
        // stack maps to locate heap-pointer slots. All pointers come from the
        // interpreter's own structures.
        unsafe {
            // Phase 1: scan roots from the call stack.
            let mut fp = self.frame_ptr;
            let mut fid = self.func_id;
            let mut current_pc = self.pc;

            loop {
                if let Some(ref_offsets) = self
                    .functions
                    .get(fid)
                    .and_then(|f| f.stack_maps.get(&current_pc))
                {
                    for &offset in ref_offsets {
                        let old_ptr = read_ptr(fp, offset as usize);
                        if !old_ptr.is_null() && self.is_heap_ptr(old_ptr) {
                            let new_ptr = self.gc_copy_object(old_ptr, &mut free_ptr);
                            write_ptr(fp, offset as usize, new_ptr);
                        }
                    }
                }

                let meta = fp.sub(FRAME_METADATA_SIZE);
                let saved_func_id = read_u64(meta, 16);
                if saved_func_id == SENTINEL_FUNC_ID {
                    break;
                }
                current_pc = read_u64(meta, 0) as usize;
                fp = read_ptr(meta, 8);
                fid = saved_func_id as usize;
            }

            // Phase 2: Cheney-style breadth-first scan of copied objects.
            let mut scan_ptr = to_space.as_ptr();
            while (scan_ptr as usize) < (free_ptr as usize) {
                let descriptor_id = read_u32(scan_ptr, 0);
                let obj_size = read_u32(scan_ptr, 4) as usize;

                if obj_size == 0 {
                    bail!("GC encountered zero-size object during scan (heap corruption?)");
                }

                if descriptor_id != FORWARDED_MARKER {
                    self.gc_scan_object(scan_ptr, descriptor_id, &mut free_ptr);
                }

                let aligned_size = (obj_size + 7) & !7;
                scan_ptr = scan_ptr.add(aligned_size);
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
    fn gc_copy_object(&self, old_ptr: *mut u8, free_ptr: &mut *mut u8) -> *mut u8 {
        unsafe {
            let descriptor_id = read_u32(old_ptr, 0);

            if descriptor_id == FORWARDED_MARKER {
                return read_ptr(old_ptr, OBJECT_HEADER_SIZE);
            }

            let obj_size = read_u32(old_ptr, 4) as usize;
            let aligned_size = (obj_size + 7) & !7;
            let new_ptr = *free_ptr;

            std::ptr::copy_nonoverlapping(old_ptr, new_ptr, obj_size);
            *free_ptr = new_ptr.add(aligned_size);

            write_u32(old_ptr, 0, FORWARDED_MARKER);
            write_ptr(old_ptr, OBJECT_HEADER_SIZE, new_ptr);

            new_ptr
        }
    }

    /// Scan a copied object for internal heap references and copy them too.
    fn gc_scan_object(&self, obj_ptr: *mut u8, descriptor_id: u32, free_ptr: &mut *mut u8) {
        let desc = if (descriptor_id as usize) < self.descriptors.len() {
            &self.descriptors[descriptor_id as usize]
        } else {
            return;
        };

        match desc {
            ObjectDescriptor::Trivial => {},
            ObjectDescriptor::Vector {
                elem_size,
                elem_ref_offsets,
            } => {
                if elem_ref_offsets.is_empty() {
                    return;
                }
                unsafe {
                    let length = read_u64(obj_ptr, VEC_LENGTH_OFFSET) as usize;
                    let data_start = obj_ptr.add(VEC_DATA_OFFSET);

                    for i in 0..length {
                        let elem_base = data_start.add(i * (*elem_size as usize));
                        for &ref_off in elem_ref_offsets {
                            let old_ptr = read_ptr(elem_base, ref_off as usize);
                            if !old_ptr.is_null() && self.is_heap_ptr(old_ptr) {
                                let new_ptr = self.gc_copy_object(old_ptr, free_ptr);
                                write_ptr(elem_base, ref_off as usize, new_ptr);
                            }
                        }
                    }
                }
            },
            ObjectDescriptor::Struct { ref_offsets, .. } => {
                if ref_offsets.is_empty() {
                    return;
                }
                unsafe {
                    for &off in ref_offsets {
                        let old_ptr = read_ptr(obj_ptr, STRUCT_DATA_OFFSET + off as usize);
                        if !old_ptr.is_null() && self.is_heap_ptr(old_ptr) {
                            let new_ptr = self.gc_copy_object(old_ptr, free_ptr);
                            write_ptr(obj_ptr, STRUCT_DATA_OFFSET + off as usize, new_ptr);
                        }
                    }
                }
            },
        }
    }
}
