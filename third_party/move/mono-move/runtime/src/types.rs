// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Misc runtime types and layout constants.
//!
//! Object descriptors live in [`crate::heap::object_descriptor`].

pub(crate) use mono_move_core::MAX_ALIGN;

// ---------------------------------------------------------------------------
// Step result
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum StepResult {
    /// There are more instructions to execute.
    Continue,
    /// The outermost function has returned — execution is complete.
    Done,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub(crate) const DEFAULT_STACK_SIZE: usize = 1024 * 1024; // 1 MiB

pub(crate) const DEFAULT_HEAP_SIZE: usize = 10 * 1024 * 1024; // 10 MiB

/// Byte offset of `saved_pc` within frame metadata.
pub(crate) const META_SAVED_PC_OFFSET: usize = 0;
/// Byte offset of `saved_fp` within frame metadata.
pub(crate) const META_SAVED_FP_OFFSET: usize = 8;
/// Byte offset of `saved_func_ptr` within frame metadata.
pub(crate) const META_SAVED_FUNC_PTR_OFFSET: usize = 16;

/// Offset of the `length` field within a vector object's data region.
/// Vectors put the length at the very start of the data region; element 0
/// begins at [`VEC_DATA_OFFSET`].
pub const VEC_LENGTH_OFFSET: usize = 0;
/// Offset where vector element data begins (after the `length` field).
/// Capacity is not stored; it is derived from the header's `size_in_bytes`
/// field: `capacity = (size_in_bytes - OBJECT_HEADER_SIZE - VEC_DATA_OFFSET) / elem_size`.
pub const VEC_DATA_OFFSET: usize = 8;

// Element 0 must land on a `MAX_ALIGN`-aligned offset so that any element
// type (whose alignment is ≤ `MAX_ALIGN`) is naturally aligned. Object
// pointers are `MAX_ALIGN`-aligned by the allocator ([§3.1] of
// `docs/memory_alignment.md`), so this assertion guards that
// `VEC_DATA_OFFSET` itself is also a `MAX_ALIGN` multiple. See
// `docs/memory_alignment.md` (§8.3).
const _: () = assert!(VEC_DATA_OFFSET.is_multiple_of(MAX_ALIGN));

/// Marker written into the `descriptor_id` field of a forwarded object during GC.
pub(crate) const FORWARDED_MARKER: u32 = u32::MAX;
