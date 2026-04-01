// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::instruction::{FrameOffset, MicroOp, FRAME_METADATA_SIZE};
use mono_move_alloc::GlobalArenaPtr;

/// ---------------------------------------------------------------------------
// Function representation
// ---------------------------------------------------------------------------

/// Frame layout (fp-relative):
///
/// ```text
///   [0 .. args_size)                    arguments (written by caller)
///   [args_size .. args_and_locals_size)          locals
///   [args_and_locals_size .. args_and_locals_size+24)     metadata (saved_pc, saved_fp, saved_func_id)
///   [args_and_locals_size+24 .. extended_frame_size)  callee arg/return slots
/// ```
///
/// `extended_frame_size` == `args_and_locals_size + FRAME_METADATA_SIZE` for leaf
/// functions (no callee region).
pub struct Function {
    pub name: GlobalArenaPtr<str>,
    pub code: Vec<MicroOp>,
    /// Size of the argument region at the start of the frame.
    /// Arguments are placed by the caller before `CallFunc`; when
    /// `zero_frame` is true, the runtime zeroes everything beyond args
    /// (`args_size..extended_frame_size`) at frame creation to ensure
    /// pointer_offsets start as null.
    pub args_size: usize,
    /// Size of the arguments + locals region. Frame metadata is stored
    /// immediately after this region at offset `args_and_locals_size`.
    pub args_and_locals_size: usize,
    /// Total frame footprint including metadata and callee slots.
    /// Must be >= `frame_size()` (i.e., `args_and_locals_size + FRAME_METADATA_SIZE`).
    /// For leaf functions this equals `frame_size()`; for calling functions
    /// it additionally includes callee argument / return value slots
    /// (sized to fit the largest callee's args or return values).
    pub extended_frame_size: usize,
    /// Whether the runtime must zero-initialize the region beyond args
    /// (`args_size..extended_frame_size`) when a new frame is created.
    /// This is required when pointer_offsets exist so the GC sees null
    /// instead of garbage. Functions with no heap pointer slots (beyond
    /// args) can set this to `false` to skip the memset.
    pub zero_frame: bool,
    /// Frame byte-offsets of slots that may hold heap pointers (GC roots).
    ///
    /// Offsets span `[0..extended_frame_size)` — they may reference the
    /// data segment AND the callee argument/return region beyond the
    /// metadata. The GC scans these slots in every live frame — no
    /// per-PC stack maps are needed (see docs/heap_and_gc.md).
    ///
    /// Each entry is the offset of an 8-byte slot that holds a heap
    /// pointer (or null). For a 16-byte fat pointer `(base, offset)` at
    /// frame offset `X`, list `X` here — the base is the heap pointer;
    /// `X+8` is a scalar offset and is not listed.
    ///
    /// Invariants:
    ///
    /// - **Zeroed at frame creation**: when `zero_frame` is true, the
    ///   runtime zeroes `args_size..extended_frame_size` when a frame
    ///   is created, so all non-argument pointer slots (including the
    ///   callee arg/return region) start as null.
    /// - **Pointer-only writes**: a pointer_offset slot may only be
    ///   overwritten with another valid heap pointer (or null). The
    ///   re-compiler must guarantee this.
    ///
    /// The callee arg region (`frame_size()..extended_frame_size`)
    /// overlaps with the callee's frame during GC traversal — both
    /// frames may scan the same memory. The forwarding markers in
    /// `gc_copy_object` handle double-scans correctly.
    pub pointer_offsets: Vec<FrameOffset>,
}

impl Function {
    /// The frame size including metadata: `args_and_locals_size + FRAME_METADATA_SIZE`.
    /// This is the offset where callee arguments begin.
    pub fn frame_size(&self) -> usize {
        self.args_and_locals_size + FRAME_METADATA_SIZE
    }
}
