// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Core types and constants for the interpreter runtime.

use crate::FrameOffset;
pub use mono_move_micro_ops::DescriptorId;
use mono_move_micro_ops::MicroOp;

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
        /// Byte stride between consecutive elements.
        elem_size: u32,
        /// Byte offsets within each element that are heap pointers.
        elem_ref_offsets: Vec<u32>,
    },

    /// Fixed-size struct allocated on the heap.
    Struct {
        /// Total payload size in bytes (excluding the object header).
        size: u32,
        /// Byte offsets within the payload that hold owned heap pointers.
        /// Move forbids references inside structs, so these are always
        /// 8-byte pointers to other heap objects (vectors, structs, etc.).
        ref_offsets: Vec<u32>,
    },

    /// Enum (tagged union) allocated on the heap.
    /// Layout: [header(8)] [tag: u64(8)] [fields padded to max variant size]
    Enum {
        /// Total payload size in bytes (tag + max variant fields, excluding header).
        size: u32,
        /// Per-variant reference layouts. `variant_ref_offsets[tag]` gives
        /// byte offsets (relative to `ENUM_DATA_OFFSET`) that hold heap
        /// pointers for that variant.
        variant_ref_offsets: Vec<Vec<u32>>,
    },
}

// ---------------------------------------------------------------------------
// Function representation
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct Function {
    pub code: Vec<MicroOp>,
    /// Size of the argument region at the start of the data segment.
    /// Arguments are placed by the caller before `CallFunc`; when
    /// `zero_frame` is true, the runtime zeroes everything beyond args
    /// (`args_size..extended_frame_size`) at frame creation to ensure
    /// pointer slots start as null.
    pub args_size: usize,
    /// Size of the data segment (arguments + locals). Frame metadata is stored
    /// immediately after this region at offset `data_size`.
    pub data_size: usize,
    /// Total frame footprint including metadata. Must be >= `frame_size()`
    /// (i.e., `data_size + FRAME_METADATA_SIZE`). For non-calling leaf
    /// functions this equals `frame_size()`; for calling functions it
    /// additionally includes callee argument / return value slots.
    pub extended_frame_size: usize,
    /// Whether the runtime must zero-initialize the region beyond args
    /// (`args_size..extended_frame_size`) when a new frame is created.
    /// This is required when pointer slots exist so the GC sees null
    /// instead of garbage. Functions with no pointer slots (beyond args)
    /// can set this to `false` to skip the memset.
    pub zero_frame: bool,
    /// Frame byte-offsets that may hold heap pointers (GC roots).
    ///
    /// Offsets span `[0..extended_frame_size)` — they may reference the
    /// data segment, AND the callee argument/return region beyond the
    /// metadata. The GC scans these slots in every live frame — no
    /// per-PC stack maps are needed (see docs/gc_design.md).
    ///
    /// Invariants:
    ///
    /// - **Zeroed at frame creation**: when `zero_frame` is true, the
    ///   runtime zeroes `args_size..extended_frame_size` when a frame
    ///   is created, so all non-argument pointer slots (including the
    ///   callee arg/return region) start as null.
    /// - **Pointer-only writes**: a pointer slot may only be overwritten
    ///   with another valid heap pointer (or null). The re-compiler must
    ///   guarantee this.
    /// - **Fat pointers**: for a 16-byte `(base, offset)` fat pointer at
    ///   offset `X`, only `X` (the base) is listed here. The second word
    ///   (`X+8`, the offset) is a scalar.
    ///
    /// Callee arg region (`frame_size()..extended_frame_size`):
    ///
    /// This region overlaps with the callee's frame during GC
    /// traversal — both frames may scan the same memory. The
    /// forwarding markers in `gc_copy_object` handle double-scans
    /// correctly.
    pub pointer_slots: Vec<FrameOffset>,
}

impl Function {
    /// The frame size including metadata: `data_size + FRAME_METADATA_SIZE`.
    /// This is the offset where callee arguments begin.
    pub fn frame_size(&self) -> usize {
        self.data_size + FRAME_METADATA_SIZE
    }
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

// Re-export layout constants from the micro-ops crate (canonical definitions).
pub use mono_move_micro_ops::{
    ENUM_DATA_OFFSET, ENUM_TAG_OFFSET, FRAME_METADATA_SIZE, OBJECT_HEADER_SIZE, STRUCT_DATA_OFFSET,
};

/// Sentinel value written as `saved_func_id` for the outermost frame.
/// When `Return` reads this, it knows execution is complete.
pub(crate) const SENTINEL_FUNC_ID: u64 = u64::MAX;

/// Byte offset of the `descriptor_id` (u32) within any heap object header.
pub(crate) const HEADER_DESCRIPTOR_OFFSET: usize = 0;
/// Byte offset of `size_in_bytes` (u32) within any heap object header.
pub(crate) const HEADER_SIZE_OFFSET: usize = 4;

/// Byte offset of `saved_pc` within frame metadata.
pub(crate) const META_SAVED_PC_OFFSET: usize = 0;
/// Byte offset of `saved_fp` within frame metadata.
pub(crate) const META_SAVED_FP_OFFSET: usize = 8;
/// Byte offset of `saved_func_id` within frame metadata.
pub(crate) const META_SAVED_FUNC_ID_OFFSET: usize = 16;

/// Offset of the `length` field within a vector object (after the header).
pub const VEC_LENGTH_OFFSET: usize = OBJECT_HEADER_SIZE; // 8
/// Offset where vector element data begins (after header + length).
/// Capacity is not stored; it is derived from the header's `size_in_bytes`
/// field: `capacity = (size_in_bytes - VEC_DATA_OFFSET) / elem_size`.
pub const VEC_DATA_OFFSET: usize = OBJECT_HEADER_SIZE + 8; // 16

/// Marker written into the `descriptor_id` field of a forwarded object during GC.
pub(crate) const FORWARDED_MARKER: u32 = u32::MAX;
