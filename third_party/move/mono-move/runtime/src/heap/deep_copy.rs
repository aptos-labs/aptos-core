// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Single-pass recursive deep-copy of a heap-based value.
//!
//! TODO(metering): reimplement this with non-recursive algorithm or add
//!   depth checks.

use crate::{
    error::{RuntimeError, RuntimeInvariantViolation},
    heap::{heap_alloc, AllocationError, AllocationResult, Heap},
    types::VEC_DATA_OFFSET,
};
use mono_move_core::{
    memory::{MemAddr, Obj, ObjHeader},
    DescriptorId, DescriptorProvider, ObjectDescriptorInner, CAPTURED_DATA_VALUES_OFFSET,
    CLOSURE_CAPTURED_DATA_PTR_OFFSET, ENUM_DATA_OFFSET, OBJECT_HEADER_SIZE,
};

impl Heap {
    /// Single-pass deep copy of the value tree rooted at the given source.
    /// Move values form a tree (no sharing, no cycles), so a tree copy is
    /// sufficient. Returns the data-region pointer of the freshly-allocated
    /// root copy, or [`AllocationError::OutOfHeapMemory`] on the first
    /// allocation that does not fit. There is no GC during copying: on
    /// out-of-memory the caller may have to run GC and retry.
    ///
    /// `src` is a live object by [`Obj`]'s construction invariant; its header
    /// must be valid (object-header integrity).
    pub(crate) fn try_deep_copy<P: DescriptorProvider + ?Sized>(
        &mut self,
        descriptors: &P,
        src: Obj,
    ) -> AllocationResult<Obj> {
        let ObjHeader { descriptor, size } = src.header();
        let src_size = size as usize;
        let src_desc_id = DescriptorId(descriptor);

        let new = heap_alloc(self, src_size, src_desc_id)?;

        // SAFETY: `src` is live with `src_size - OBJECT_HEADER_SIZE` bytes of
        // payload (its header says so), `new` was just allocated with the same
        // size, and the two are distinct allocations, so they don't overlap.
        unsafe {
            src.addr()
                .copy_to_nonoverlapping(new.addr(), src_size - OBJECT_HEADER_SIZE);
        }
        let desc = descriptors.descriptor(src_desc_id).ok_or_else(|| {
            RuntimeError::InvariantViolation(RuntimeInvariantViolation::DescriptorNotFound {
                descriptor_id: src_desc_id.as_u32(),
            })
        })?;

        // Walk pointer fields. The freshly-copied object still holds the old
        // child pointers, so each child is deep-copied and the corresponding
        // slot in the cloned object is patched with the new pointer.
        match desc.inner() {
            ObjectDescriptorInner::Trivial => {},
            ObjectDescriptorInner::Struct {
                pointer_offsets, ..
            } => {
                for &offset in pointer_offsets {
                    // SAFETY: the descriptor lists `offset` as a pointer slot
                    // of this struct layout.
                    unsafe { self.try_deep_copy_slot(descriptors, new.field(offset as usize))? };
                }
            },
            ObjectDescriptorInner::CapturedData { pointer_offsets } => {
                for &offset in pointer_offsets {
                    // Shared across objects of different `values_size`
                    // (offset-shape dedup), so an offset isn't bounded by this
                    // object's size; the layout guarantees `values_size >=
                    // offset + 8`, which `field_range` checks for this object.
                    //
                    // SAFETY: the descriptor lists `offset` as a pointer slot
                    // within the values region.
                    unsafe {
                        self.try_deep_copy_slot(
                            descriptors,
                            new.field_range(CAPTURED_DATA_VALUES_OFFSET + offset as usize, 8),
                        )?
                    };
                }
            },
            ObjectDescriptorInner::Enum {
                variant_pointer_offsets,
                ..
            } => {
                // SAFETY: the descriptor says `new` is an enum, so its tag
                // lives at `ENUM_TAG_OFFSET`.
                let tag = unsafe { new.enum_tag() } as usize;
                if tag >= variant_pointer_offsets.len() {
                    return Err(AllocationError::from(RuntimeError::InvariantViolation(
                        RuntimeInvariantViolation::EnumTagOutOfRange {
                            tag: tag as u64,
                            variant_count: variant_pointer_offsets.len(),
                        },
                    )));
                }
                for &offset in &variant_pointer_offsets[tag] {
                    // SAFETY: the descriptor lists `offset` as a pointer slot
                    // of the active variant, relative to the data after the tag.
                    unsafe {
                        self.try_deep_copy_slot(
                            descriptors,
                            new.field(ENUM_DATA_OFFSET + offset as usize),
                        )?
                    };
                }
            },
            ObjectDescriptorInner::Vector {
                elem_size,
                elem_pointer_offsets,
            } => {
                // SAFETY: the descriptor says `new` is a vector, so its length
                // lives at `VEC_LENGTH_OFFSET`.
                let length = unsafe { new.vec_len() };
                // Length comes from the GC-maintained object header, not from
                // attacker-controlled bytes, so a debug assertion suffices
                // (unlike the enum tag, which is validated unconditionally).
                debug_assert!(
                    VEC_DATA_OFFSET + (length as usize).saturating_mul(*elem_size as usize)
                        <= src_size - OBJECT_HEADER_SIZE,
                    "vector length {length} exceeds object payload"
                );
                for i in 0..length {
                    for &offset in elem_pointer_offsets {
                        // SAFETY: `i < length` is within the vector's data
                        // region and `offset` is a pointer slot of its element
                        // layout.
                        unsafe {
                            self.try_deep_copy_slot(
                                descriptors,
                                new.vec_elem(i, *elem_size).add(offset as usize),
                            )?
                        };
                    }
                }
            },
            ObjectDescriptorInner::Closure => {
                // SAFETY: the descriptor says `new` is a closure, whose only
                // heap pointer is `captured_data_ptr` at a fixed offset.
                unsafe {
                    self.try_deep_copy_slot(
                        descriptors,
                        new.field(CLOSURE_CAPTURED_DATA_PTR_OFFSET),
                    )?
                };
            },
        }

        Ok(new)
    }

    /// Deep-copies the object the pointer slot at `slot` points at (if any)
    /// and patches the slot with the copy.
    ///
    /// # Safety
    ///
    /// `slot` must be a pointer slot of a freshly copied object, located by
    /// that object's descriptor: in-bounds, pointer-aligned, and holding null
    /// or the data pointer of a live object.
    unsafe fn try_deep_copy_slot<P: DescriptorProvider + ?Sized>(
        &mut self,
        descriptors: &P,
        slot: MemAddr,
    ) -> AllocationResult<()> {
        // SAFETY: `slot` is a pointer slot (this function's contract).
        let child_src = unsafe { slot.load_ptr() };
        if child_src.is_null() {
            return Ok(());
        }
        // SAFETY: the parent's descriptor lists this slot as a heap pointer,
        // so a non-null `child_src` is the data pointer of a live object.
        let child_src = unsafe { Obj::new(child_src) };
        let child_dst = self.try_deep_copy(descriptors, child_src)?;
        // SAFETY: same slot as the read above.
        unsafe {
            slot.store_ptr(child_dst.addr());
        }
        Ok(())
    }
}
