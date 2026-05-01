// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Misc runtime types and layout constants.
//!
//! Object descriptors live in [`crate::heap::object_descriptor`].

pub(crate) use mono_move_core::OBJECT_HEADER_SIZE;

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

/// Byte offset of the `descriptor_id` (u32) within any heap object header.
pub(crate) const HEADER_DESCRIPTOR_OFFSET: usize = 0;
/// Byte offset of `size_in_bytes` (u32) within any heap object header.
pub(crate) const HEADER_SIZE_OFFSET: usize = 4;

/// Byte offset of `saved_pc` within frame metadata.
pub(crate) const META_SAVED_PC_OFFSET: usize = 0;
/// Byte offset of `saved_fp` within frame metadata.
pub(crate) const META_SAVED_FP_OFFSET: usize = 8;
/// Byte offset of `saved_func_ptr` within frame metadata.
pub(crate) const META_SAVED_FUNC_PTR_OFFSET: usize = 16;

/// Offset of the `length` field within a vector object (after the header).
pub const VEC_LENGTH_OFFSET: usize = OBJECT_HEADER_SIZE; // 8
/// Offset where vector element data begins (after header + length).
/// Capacity is not stored; it is derived from the header's `size_in_bytes`
/// field: `capacity = (size_in_bytes - VEC_DATA_OFFSET) / elem_size`.
pub const VEC_DATA_OFFSET: usize = OBJECT_HEADER_SIZE + 8; // 16

/// Marker written into the `descriptor_id` field of a forwarded object during GC.
pub(crate) const FORWARDED_MARKER: u32 = u32::MAX;
