// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Core types and constants for the interpreter runtime.

pub(crate) use mono_move_core::OBJECT_HEADER_SIZE;

// ---------------------------------------------------------------------------
// Object descriptors (for GC tracing)
// ---------------------------------------------------------------------------

/// Describes the reference layout of a heap object so the GC knows how to
/// trace internal pointers. Only one level of indirection is described;
/// pointed-to objects are self-describing via their own headers.
#[derive(Debug)]
pub enum ObjectDescriptor {
    /// No internal heap references. GC copies the blob and moves on.
    Trivial,

    /// Vector whose elements may contain heap pointers at known offsets.
    Vector {
        /// Size of each element in bytes.
        /// The address of element `i` is `data_start + i * elem_size`.
        elem_size: u32,
        /// Byte offsets within each element that are heap pointers.
        elem_pointer_offsets: Vec<u32>,
    },

    /// Fixed-size struct allocated on the heap.
    Struct {
        /// Total payload size in bytes (excluding the object header).
        size: u32,
        /// Byte offsets within the payload that hold owned heap pointers.
        /// Move forbids references inside structs, so these are always
        /// 8-byte pointers to other heap objects (vectors, structs, etc.).
        pointer_offsets: Vec<u32>,
    },

    /// Enum (tagged union) allocated on the heap.
    /// Layout: [header(8)] [tag: u64(8)] [fields padded to max variant size]
    Enum {
        /// Total payload size in bytes (tag + max variant fields, excluding header).
        size: u32,
        /// Per-variant pointer layouts. `variant_pointer_offsets[tag]` gives
        /// byte offsets (relative to `ENUM_DATA_OFFSET`) that hold heap
        /// pointers for that variant.
        variant_pointer_offsets: Vec<Vec<u32>>,
    },

    /// Closure object — fixed runtime layout shared by every closure.
    ///
    /// Payload layout (`size = CLOSURE_OBJECT_SIZE - OBJECT_HEADER_SIZE = 32`):
    /// `[func_ref(16)] [mask(8)] [captured_data_ptr(8)]`. The single heap
    /// pointer is `captured_data_ptr` at payload offset
    /// `CLOSURE_CAPTURED_DATA_PTR_OFFSET - OBJECT_HEADER_SIZE = 24`. Both
    /// the size and the pointer offset are constants of the runtime
    /// layout — there is nothing per-instance to store here.
    Closure,

    /// `ClosureCapturedData` (Materialized) object.
    ///
    /// Object layout: `[header(8)] [tag(1) + padding(7)] [values...]`.
    /// `size` and `pointer_offsets` are interpreted relative to the
    /// values region (i.e., excluding both the header and the
    /// tag+padding prefix), so an offset of `0` names the first byte of
    /// the first captured value. The 8-byte tag prefix is added
    /// internally by the GC.
    CapturedData {
        /// Byte size of the values region (sum of captured value sizes).
        size: u32,
        /// Byte offsets within the values region that hold heap pointers.
        pointer_offsets: Vec<u32>,
    },
}

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
