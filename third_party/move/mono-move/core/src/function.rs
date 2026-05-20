// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::instruction::{CodeOffset, FrameOffset, MicroOp, FRAME_METADATA_SIZE};
use arc_swap::ArcSwap;
use mono_move_alloc::{GlobalArenaPtr, LeakedBoxPtr};
use std::{ptr::NonNull, sync::Arc};

/// Function's micro-ops.
pub struct Code {
    inner: ArcSwap<Vec<MicroOp>>,
}

impl Code {
    /// Builds code from a vector of micro-ops.
    pub fn from_vec(ops: Vec<MicroOp>) -> Self {
        Self {
            inner: ArcSwap::from_pointee(ops),
        }
    }

    /// Snapshot of the current micro-ops.
    ///
    /// TODO: decide on if using ArcSwap is good enough for perf and
    ///   what is the best way to update code and any other relevant
    ///   information in the function.
    pub fn load(&self) -> arc_swap::Guard<Arc<Vec<MicroOp>>> {
        self.inner.load()
    }

    /// Replaces the micro-ops atomically.
    pub fn store(&self, ops: Vec<MicroOp>) {
        self.inner.store(Arc::new(ops));
    }
}

/// ---------------------------------------------------------------------------
// Function representation
// ---------------------------------------------------------------------------

/// A snapshot of frame layout information at a particular point in execution.
///
/// Currently tracks which frame offsets hold heap pointers (for GC root
/// scanning). Designed to be extended with additional per-slot type or
/// layout information in the future — e.g., slot type tags for stronger
/// runtime verification or debugging.
pub struct FrameLayoutInfo {
    /// Frame byte-offsets of slots that may hold pointers (GC roots).
    ///
    /// Each entry is the offset of an 8-byte slot that holds a pointer
    /// (heap pointer, or pointer to a stack local via a Move reference)
    /// or null. The GC scans these slots and safely ignores any pointer
    /// that does not point into the heap.
    ///
    /// For a 16-byte fat pointer `(base, offset)` at frame offset `X`,
    /// list `X` here — the base is the pointer; `X+8` is a scalar
    /// offset and is not listed.
    ///
    /// Offsets must not fall in the metadata segment
    /// (`param_and_local_sizes_sum..param_and_local_sizes_sum + FRAME_METADATA_SIZE`).
    pub heap_ptr_offsets: Vec<FrameOffset>,
}

impl FrameLayoutInfo {
    /// Creates frame layout information from a vector of pointer offsets.
    pub fn new(heap_ptr_offsets: Vec<FrameOffset>) -> Self {
        Self { heap_ptr_offsets }
    }

    /// Creates empty frame layout information.
    pub fn empty() -> Self {
        Self {
            heap_ptr_offsets: vec![],
        }
    }
}

/// Additional frame layout that applies at a specific safe point.
///
/// Safe points are instructions where GC may run:
///
/// - **Allocating instructions** (`HeapNew`, `VecPushBack`, `ForceGC`):
///   GC runs during the instruction, so the safe point is at that
///   instruction's own PC.
/// - **Call return sites**: when a callee triggers GC, the caller's
///   saved PC is `call_pc + 1`. The safe point for a caller frame is
///   the instruction *after* the call — at that point, the shared
///   arg/return region holds return values, not arguments.
pub struct SafePointEntry {
    pub code_offset: CodeOffset,
    pub layout: FrameLayoutInfo,
}

/// A sorted collection of per-safe-point frame layouts.
///
/// Provides O(log n) lookup by code offset. Entries must be strictly sorted by
/// `code_offset`.
pub struct SortedSafePointEntries {
    entries: Vec<SafePointEntry>,
}

impl SortedSafePointEntries {
    /// Creates safe point entries.
    ///
    /// The caller must ensure entries are strictly sorted by `code_offset`
    /// and that pointer offsets are disjoint from `frame_layout`.
    pub fn new(entries: Vec<SafePointEntry>) -> Self {
        Self { entries }
    }

    /// Creates empty safe point entries.
    pub fn empty() -> Self {
        Self { entries: vec![] }
    }

    /// Look up the safe-point layout for a given code offset, if one exists.
    pub fn layout_at(&self, pc: usize) -> Option<&FrameLayoutInfo> {
        if self.entries.is_empty() {
            return None;
        }
        let pc = pc as u32;
        self.entries
            .binary_search_by_key(&pc, |e| e.code_offset.0)
            .ok()
            .map(|idx| &self.entries[idx].layout)
    }

    /// Access the underlying entries slice.
    pub fn entries(&self) -> &[SafePointEntry] {
        &self.entries
    }
}

/// Frame layout (fp-relative):
///
/// ```text
///   [0 .. param_sizes_sum)                    parameters (written by caller as arguments)
///   [param_sizes_sum .. param_and_local_sizes_sum)          locals
///   [param_and_local_sizes_sum .. param_and_local_sizes_sum+24)     metadata (saved_pc, saved_fp, saved_func_id)
///   [param_and_local_sizes_sum+24 .. extended_frame_size)  callee arg/return slots
/// ```
///
/// `extended_frame_size` == `param_and_local_sizes_sum + FRAME_METADATA_SIZE` for leaf
/// functions (no callee region).
pub struct Function {
    pub name: GlobalArenaPtr<str>,
    pub code: Code,
    /// Byte size of each parameter, in declaration order.
    ///
    /// Used by `CallClosure` (together with the closure's `ClosureMask`) to
    /// compute each parameter's offset in the callee's parameter region and
    /// to advance through the packed captured values. The sum of these sizes
    /// must equal `param_sizes_sum`.
    //
    // TODO: this only captures sizes, not alignment. Once the layout admits
    // non-8-byte fields, the closure-call interleaver will need per-param
    // alignment (either encoded alongside `size` here, or as a sibling
    // `param_alignments` slice).
    pub param_sizes: Vec<u32>,
    /// Size of the parameter region at the start of the frame.
    /// The caller writes the corresponding arguments into this region
    /// before the call instruction; when `zero_frame` is true, the runtime
    /// zeroes everything beyond the parameter region
    /// (`param_sizes_sum..extended_frame_size`) at frame creation to
    /// ensure pointer slots start as null.
    pub param_sizes_sum: usize,
    /// Size of the parameters + locals region. Frame metadata is stored
    /// immediately after this region at offset `param_and_local_sizes_sum`.
    pub param_and_local_sizes_sum: usize,
    /// Total frame footprint including metadata and callee slots.
    /// Must be >= `frame_size()` (i.e., `param_and_local_sizes_sum + FRAME_METADATA_SIZE`).
    /// For leaf functions this equals `frame_size()`; for calling functions
    /// it additionally includes callee argument / return value slots
    /// (sized to fit the largest callee's arguments or return values).
    pub extended_frame_size: usize,
    /// Whether the runtime must zero-initialize the region beyond
    /// parameters (`param_sizes_sum..extended_frame_size`) when a new
    /// frame is created. This is required when `frame_layout` has
    /// pointer slots so the GC sees null instead of garbage. Functions
    /// with no heap pointer slots in `frame_layout` (beyond parameters)
    /// can set this to `false` to skip the memset. Not needed if the
    /// function uses only per-PC layouts and the specializer ensures
    /// slots are written before becoming visible as pointers.
    //
    // TODO: derive from `frame_layout` instead of taking as input.
    // `safe_point_layouts` doesn't need zeroing — each entry already
    // pins which slots hold valid pointers at that PC.
    pub zero_frame: bool,
    /// Base frame layout — pointer offsets that are valid at every point
    /// in the function's execution. The GC always scans these.
    ///
    /// Offsets span `[0..extended_frame_size)` — they may reference the
    /// data segment AND the callee arg/return region beyond the
    /// metadata, but must NOT fall in the metadata segment itself.
    ///
    /// Invariants:
    ///
    /// - **Zeroed at frame creation**: when `zero_frame` is true, the
    ///   runtime zeroes `param_sizes_sum..extended_frame_size` when a frame
    ///   is created, so all non-parameter pointer slots (including the
    ///   callee arg/return region) start as null.
    /// - **Pointer-only writes**: a pointer slot may only be
    ///   overwritten with another valid heap pointer (or null). The
    ///   specializer must guarantee this.
    ///
    /// The callee arg region (`frame_size()..extended_frame_size`)
    /// overlaps with the callee's frame during GC traversal — both
    /// frames may scan the same memory. The forwarding markers in
    /// `gc_copy_object` handle double-scans correctly.
    pub frame_layout: FrameLayoutInfo,
    /// Per-safe-point frame layouts.
    ///
    /// During GC, for each frame on the call stack, the GC scans the
    /// union of `frame_layout.heap_ptr_offsets` (always) and the
    /// matching safe-point entry's `heap_ptr_offsets` (if the frame's
    /// current PC has a corresponding entry).
    ///
    /// The offsets in each safe-point entry must be disjoint from
    /// `frame_layout.heap_ptr_offsets` — a slot that is always a pointer
    /// belongs in `frame_layout`, not in individual safe-point entries.
    ///
    /// This supplements `frame_layout` for slots whose pointer status
    /// changes across the function — e.g., shared arg/return regions
    /// that hold a pointer argument before a call but a scalar return
    /// value after, or callee arg slots used by different callees.
    ///
    /// Empty when the function needs no per-PC distinction (all pointer
    /// slots are stable across the entire function body).
    pub safe_point_layouts: SortedSafePointEntries,
}

impl Function {
    /// The frame size including metadata: `param_and_local_sizes_sum + FRAME_METADATA_SIZE`.
    /// This is the offset where the callee arg region begins.
    pub fn frame_size(&self) -> usize {
        self.param_and_local_sizes_sum + FRAME_METADATA_SIZE
    }

    /// Look up the safe-point layout for a given code offset, if one exists.
    ///
    /// Returns `None` if there is no entry for this exact code offset.
    pub fn safe_point_layout_at(&self, pc: usize) -> Option<&FrameLayoutInfo> {
        self.safe_point_layouts.layout_at(pc)
    }
}

/// Pointer to lowered function. See [`LeakedBoxPtr`] for safety contract.
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct FunctionPtr(LeakedBoxPtr<Function>);

impl FunctionPtr {
    /// Leaks the box and returns a stable pointer to the function.
    pub fn new(function: Box<Function>) -> Self {
        Self(LeakedBoxPtr::from_box(function))
    }

    /// Returns the underlying non-null pointer.
    pub fn as_non_null(&self) -> NonNull<Function> {
        self.0.as_non_null()
    }

    /// Frees allocated data.
    ///
    /// # Safety
    ///
    /// The caller must ensure that no other references to the data exist and
    /// that this method is called at most once per pointer.
    pub unsafe fn free_unchecked(self) {
        unsafe { self.0.free_unchecked() }
    }

    /// Returns a shared reference to the pointee with an explicit lifetime.
    ///
    /// # Safety
    ///
    /// The caller must ensure the pointer has not been freed and that the
    /// returned reference does not outlive the actual allocation.
    pub unsafe fn as_ref_unchecked<'a>(&self) -> &'a Function {
        // SAFETY: The caller guarantees the pointer is still valid.
        unsafe { self.0.as_ref_unchecked() }
    }
}
