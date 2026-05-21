// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Single-pass recursive deep-copy of a heap-based value.
//!
//! TODO(security): reimplement this with non-recursive algorithm or add
//!   depth checks.

use crate::{
    error::{RuntimeError, RuntimeInvariantViolation},
    heap::{heap_alloc, AllocationError, AllocationResult, Heap},
    memory::{read_descriptor, read_obj_size, read_ptr, read_u64, write_ptr},
    types::{VEC_DATA_OFFSET, VEC_LENGTH_OFFSET},
};
use mono_move_core::{
    DescriptorId, DescriptorProvider, ObjectDescriptorInner, CAPTURED_DATA_VALUES_OFFSET,
    CLOSURE_CAPTURED_DATA_PTR_OFFSET, ENUM_DATA_OFFSET, ENUM_TAG_OFFSET, OBJECT_HEADER_SIZE,
};
use std::ptr::NonNull;

impl Heap {
    /// Single-pass deep copy of the value graph rooted at the given source.
    /// Returns the data-region pointer of the freshly-allocated root copy,
    /// or [`AllocationError::OutOfHeapMemory`] on the first allocation that
    /// does not fit. There is no GC during copying and the caller has to
    /// handle memory.
    ///
    /// # Safety
    ///
    /// Source must point to the data region of a live object whose header is
    /// at `src - OBJECT_HEADER_SIZE` and is valid.
    pub(crate) fn try_deep_copy<P: DescriptorProvider + ?Sized>(
        &mut self,
        descriptors: &P,
        src: NonNull<u8>,
    ) -> AllocationResult<NonNull<u8>> {
        // SAFETY: caller's contract on `src`.
        let src_size = unsafe { read_obj_size(src.as_ptr()) } as usize;
        let src_desc_id = DescriptorId(unsafe { read_descriptor(src.as_ptr()) });
        debug_assert!(src_size >= OBJECT_HEADER_SIZE);

        let new_raw = heap_alloc(self, src_size, src_desc_id)?;
        // SAFETY: `heap_alloc` returns non-null on success.
        let new = unsafe { NonNull::new_unchecked(new_raw) };

        // SAFETY: regions don't overlap (different allocations).
        unsafe {
            std::ptr::copy_nonoverlapping(
                src.as_ptr().cast_const(),
                new.as_ptr(),
                src_size - OBJECT_HEADER_SIZE,
            );
        }
        let desc = descriptors.descriptor(src_desc_id).ok_or_else(|| {
            RuntimeError::InvariantViolation(RuntimeInvariantViolation::DescriptorNotFound {
                descriptor_id: src_desc_id.as_u32(),
            })
        })?;

        // Walk pointer fields. The pointer values are pointing to old data,
        // so they need to be deep-copied and the source has to be patched.
        match desc.inner() {
            ObjectDescriptorInner::Trivial => {},
            ObjectDescriptorInner::Struct {
                pointer_offsets, ..
            } => {
                for &offset in pointer_offsets {
                    self.try_deep_copy_at_offset(descriptors, new, offset as usize)?;
                }
            },
            ObjectDescriptorInner::CapturedData {
                pointer_offsets, ..
            } => {
                for &offset in pointer_offsets {
                    self.try_deep_copy_at_offset(
                        descriptors,
                        new,
                        CAPTURED_DATA_VALUES_OFFSET + offset as usize,
                    )?;
                }
            },
            ObjectDescriptorInner::Enum {
                variant_pointer_offsets,
                ..
            } => {
                // SAFETY: tag lives at `ENUM_TAG_OFFSET` of every enum
                // payload.
                let tag = unsafe { read_u64(new.as_ptr(), ENUM_TAG_OFFSET) } as usize;
                if tag >= variant_pointer_offsets.len() {
                    return Err(AllocationError::from(RuntimeError::InvariantViolation(
                        RuntimeInvariantViolation::EnumTagOutOfRange {
                            tag: tag as u64,
                            variant_count: variant_pointer_offsets.len(),
                        },
                    )));
                }
                for &offset in &variant_pointer_offsets[tag] {
                    self.try_deep_copy_at_offset(
                        descriptors,
                        new,
                        ENUM_DATA_OFFSET + offset as usize,
                    )?;
                }
            },
            ObjectDescriptorInner::Vector {
                elem_size,
                elem_pointer_offsets,
            } => {
                // SAFETY: length lives at `VEC_LENGTH_OFFSET` of every
                // vector payload.
                let length = unsafe { read_u64(new.as_ptr(), VEC_LENGTH_OFFSET) } as usize;
                let elem_size = *elem_size as usize;
                for i in 0..length {
                    for &offset in elem_pointer_offsets {
                        let offset = VEC_DATA_OFFSET + i * elem_size + offset as usize;
                        self.try_deep_copy_at_offset(descriptors, new, offset)?;
                    }
                }
            },
            ObjectDescriptorInner::Closure => {
                self.try_deep_copy_at_offset(descriptors, new, CLOSURE_CAPTURED_DATA_PTR_OFFSET)?;
            },
        }

        Ok(new)
    }

    /// Deep-copy the parent's pointer at offset and patch it with the copied
    /// value.
    fn try_deep_copy_at_offset<P: DescriptorProvider + ?Sized>(
        &mut self,
        descriptors: &P,
        parent: NonNull<u8>,
        offset: usize,
    ) -> AllocationResult<()> {
        // SAFETY: parent's descriptor invariant says `field_off + 8`
        // bytes are in-bounds of the parent's data region.
        let Some(child_src) = NonNull::new(unsafe { read_ptr(parent.as_ptr(), offset) }) else {
            return Ok(());
        };
        let child_dst = self.try_deep_copy(descriptors, child_src)?;
        // SAFETY: same slot as the read above.
        unsafe {
            write_ptr(parent.as_ptr(), offset, child_dst.as_ptr());
        }
        Ok(())
    }
}
