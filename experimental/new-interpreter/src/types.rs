// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Core types and constants for the interpreter runtime.

use crate::Instruction;
use std::collections::HashMap;

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
}

// ---------------------------------------------------------------------------
// Function representation
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct Function {
    pub code: Vec<Instruction>,
    /// Size of the data segment (locals, arguments). Frame metadata is stored
    /// immediately after this region at offset `data_size`.
    pub data_size: u32,
    /// Total frame footprint including metadata. Must be >= `frame_size()`
    /// (i.e., `data_size + FRAME_METADATA_SIZE`). For non-calling leaf
    /// functions this equals `frame_size()`; for calling functions it
    /// additionally includes callee argument / return value slots.
    pub extended_frame_size: u32,
    /// Stack maps for GC root scanning.
    /// Maps PC -> list of frame byte-offsets that hold live heap references.
    /// Only PCs that are safe points (allocation sites, call return addresses)
    /// need entries; missing PCs are treated as "no refs".
    pub stack_maps: HashMap<usize, Vec<u32>>,
}

impl Function {
    /// The frame size including metadata: `data_size + FRAME_METADATA_SIZE`.
    /// This is the offset where callee arguments begin.
    pub fn frame_size(&self) -> u32 {
        self.data_size + FRAME_METADATA_SIZE as u32
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

pub const DEFAULT_STACK_SIZE: usize = 1024 * 1024; // 1 MiB

pub const DEFAULT_HEAP_SIZE: usize = 10 * 1024 * 1024; // 10 MiB

/// Size of the frame metadata saved on the unified stack.
/// Layout: [saved_pc: u64 | saved_fp: u64 | saved_func_id: u64]
pub const FRAME_METADATA_SIZE: usize = 24;

/// Sentinel value written as `saved_func_id` for the outermost frame.
/// When `Return` reads this, it knows execution is complete.
pub const SENTINEL_FUNC_ID: u64 = u64::MAX;

/// Size of the object header: [descriptor_id: u32 | size_in_bytes: u32].
pub const OBJECT_HEADER_SIZE: usize = 8;

/// Offset of the `length` field within a vector object (after the header).
pub const VEC_LENGTH_OFFSET: usize = OBJECT_HEADER_SIZE; // 8
/// Offset of the `capacity` field within a vector object.
pub const VEC_CAPACITY_OFFSET: usize = OBJECT_HEADER_SIZE + 8; // 16
/// Offset where vector element data begins.
pub const VEC_DATA_OFFSET: usize = OBJECT_HEADER_SIZE + 16; // 24

/// Offset where struct field data begins (same as OBJECT_HEADER_SIZE).
pub const STRUCT_DATA_OFFSET: usize = OBJECT_HEADER_SIZE; // 8

/// Marker written into the `descriptor_id` field of a forwarded object during GC.
pub const FORWARDED_MARKER: u32 = u32::MAX;
