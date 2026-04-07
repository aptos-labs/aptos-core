// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::instruction::{CodeOffset, FrameOffset, MicroOp, FRAME_METADATA_SIZE};
use mono_move_alloc::{ExecutableArenaPtr, GlobalArenaPtr};

/// ---------------------------------------------------------------------------
// Function representation
// ---------------------------------------------------------------------------

/// A snapshot of frame layout information at a particular point in execution.
///
/// Currently tracks which frame offsets hold heap pointers (for GC root
/// scanning). Designed to be extended with additional per-slot type or
/// layout information in the future — e.g., slot type tags for stronger
/// runtime verification or debugging.
pub struct FrameLayoutMap {
    /// Frame byte-offsets of slots that hold heap pointers (GC roots).
    ///
    /// Each entry is the offset of an 8-byte slot that holds a heap
    /// pointer (or null). For a 16-byte fat pointer `(base, offset)` at
    /// frame offset `X`, list `X` here — the base is the heap pointer;
    /// `X+8` is a scalar offset and is not listed.
    ///
    /// Offsets must not fall in the metadata segment
    /// (`args_and_locals_size..args_and_locals_size + FRAME_METADATA_SIZE`).
    pub pointer_offsets: ExecutableArenaPtr<[FrameOffset]>,
}

impl FrameLayoutMap {
    /// Create a `FrameLayoutMap` from an iterator of pointer offsets,
    /// allocating into the given arena.
    pub fn new<I>(arena: &mono_move_alloc::ExecutableArena, offsets: I) -> Self
    where
        I: IntoIterator<Item = FrameOffset>,
        I::IntoIter: ExactSizeIterator,
    {
        Self {
            pointer_offsets: arena.alloc_slice_fill_iter(offsets),
        }
    }

    /// Create an empty `FrameLayoutMap` (no pointer offsets).
    pub fn empty(arena: &mono_move_alloc::ExecutableArena) -> Self {
        Self::new(arena, std::iter::empty::<FrameOffset>())
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
///   arg/return region holds return values, not args.
pub struct SafePointEntry {
    pub code_offset: CodeOffset,
    pub layout: FrameLayoutMap,
}

/// A sorted collection of per-safe-point frame layouts.
///
/// Wraps `ExecutableArenaPtr<[SafePointEntry]>` and provides O(log n)
/// lookup by code offset. Entries must be strictly sorted by
/// `code_offset`.
pub struct SafePointMap {
    entries: ExecutableArenaPtr<[SafePointEntry]>,
}

impl SafePointMap {
    /// Create a `SafePointMap` from an iterator of entries, allocating
    /// into the given arena.
    ///
    /// The caller must ensure entries are strictly sorted by `code_offset`
    /// and that pointer offsets are disjoint from `frame_layout`.
    pub fn new<I>(arena: &mono_move_alloc::ExecutableArena, entries: I) -> Self
    where
        I: IntoIterator<Item = SafePointEntry>,
        I::IntoIter: ExactSizeIterator,
    {
        Self {
            entries: arena.alloc_slice_fill_iter(entries),
        }
    }

    /// Create an empty `SafePointMap`.
    pub fn empty(arena: &mono_move_alloc::ExecutableArena) -> Self {
        Self::new(arena, std::iter::empty::<SafePointEntry>())
    }

    /// Look up the safe-point layout for a given code offset, if one exists.
    ///
    /// # Safety
    ///
    /// `entries` must be a valid arena pointer.
    pub unsafe fn layout_at(&self, pc: usize) -> Option<&FrameLayoutMap> {
        let entries = unsafe { self.entries.as_ref_unchecked() };
        if entries.is_empty() {
            return None;
        }
        let pc = pc as u32;
        entries
            .binary_search_by_key(&pc, |e| e.code_offset.0)
            .ok()
            .map(|idx| &entries[idx].layout)
    }

    /// Access the underlying entries slice.
    ///
    /// # Safety
    ///
    /// `entries` must be a valid arena pointer.
    pub unsafe fn entries(&self) -> &[SafePointEntry] {
        unsafe { self.entries.as_ref_unchecked() }
    }
}

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
    pub code: ExecutableArenaPtr<[MicroOp]>,
    /// Size of the argument region at the start of the frame.
    /// Arguments are placed by the caller before `CallFunc`; when
    /// `zero_frame` is true, the runtime zeroes everything beyond args
    /// (`args_size..extended_frame_size`) at frame creation to ensure
    /// pointer slots start as null.
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
    /// This is required when pointer slots exist so the GC sees null
    /// instead of garbage. Functions with no heap pointer slots (beyond
    /// args) can set this to `false` to skip the memset.
    pub zero_frame: bool,
    /// Base frame layout — pointer offsets that are valid at every point
    /// in the function's execution. The GC always scans these.
    ///
    /// Offsets span `[0..extended_frame_size)` — they may reference the
    /// data segment AND the callee argument/return region beyond the
    /// metadata, but must NOT fall in the metadata segment itself.
    ///
    /// Invariants:
    ///
    /// - **Zeroed at frame creation**: when `zero_frame` is true, the
    ///   runtime zeroes `args_size..extended_frame_size` when a frame
    ///   is created, so all non-argument pointer slots (including the
    ///   callee arg/return region) start as null.
    /// - **Pointer-only writes**: a pointer slot may only be
    ///   overwritten with another valid heap pointer (or null). The
    ///   specializer must guarantee this.
    ///
    /// The callee arg region (`frame_size()..extended_frame_size`)
    /// overlaps with the callee's frame during GC traversal — both
    /// frames may scan the same memory. The forwarding markers in
    /// `gc_copy_object` handle double-scans correctly.
    pub frame_layout: FrameLayoutMap,
    /// Per-safe-point frame layouts.
    ///
    /// At a given safe point, the GC scans the union of
    /// `frame_layout.pointer_offsets` and the matching entry's
    /// `pointer_offsets`. If no entry matches the current PC, only
    /// `frame_layout` is used.
    ///
    /// The offsets in each safe-point entry must be disjoint from
    /// `frame_layout.pointer_offsets` — a slot that is always a pointer
    /// belongs in `frame_layout`, not in individual safe-point entries.
    ///
    /// This supplements `frame_layout` for slots whose pointer status
    /// changes across the function — e.g., shared arg/return regions
    /// that hold a pointer argument before a call but a scalar return
    /// value after, or callee arg slots used by different callees.
    ///
    /// Empty when the function needs no per-PC distinction (all pointer
    /// slots are stable across the entire function body).
    pub safe_point_layouts: SafePointMap,
}

impl Function {
    /// The frame size including metadata: `args_and_locals_size + FRAME_METADATA_SIZE`.
    /// This is the offset where callee arguments begin.
    pub fn frame_size(&self) -> usize {
        self.args_and_locals_size + FRAME_METADATA_SIZE
    }

    /// Look up the safe-point layout for a given code offset, if one exists.
    ///
    /// Returns `None` if there is no entry for this exact code offset.
    ///
    /// # Safety
    ///
    /// Arena pointers in `safe_point_layouts` must be valid.
    pub unsafe fn safe_point_layout_at(&self, pc: usize) -> Option<&FrameLayoutMap> {
        unsafe { self.safe_point_layouts.layout_at(pc) }
    }

    /// Replaces every [`MicroOp::CallFunc`] (index-based dispatch) with
    /// [`MicroOp::CallLocalFunc`] (direct pointer dispatch).
    ///
    /// `func_ptrs` is indexed by definition index and may contain `None`
    /// for functions that were not lowered (e.g. generic functions).
    ///
    /// # Safety
    ///
    /// The caller must have exclusive access to the functions and their
    /// arena-allocated code. The arena must outlive all uses of the patched
    /// code.
    pub unsafe fn resolve_calls(func_ptrs: &[Option<ExecutableArenaPtr<Function>>]) {
        for func_ptr in func_ptrs {
            let Some(mut func_ptr) = *func_ptr else {
                continue;
            };
            // SAFETY: We have exclusive access during build — no concurrent
            // readers exist yet. The arena is alive because the caller owns it.
            let func = unsafe { func_ptr.as_mut_unchecked() };
            let code = unsafe { func.code.as_mut_unchecked() };
            for op in code.iter_mut() {
                if let MicroOp::CallFunc { func_id } = *op {
                    *op = MicroOp::CallLocalFunc {
                        ptr: func_ptrs[func_id as usize]
                            .expect("CallFunc target must be a lowered function"),
                    };
                }
            }
        }
    }
}
