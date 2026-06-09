// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    instruction::{CodeOffset, FrameOffset, MicroOp, SizedSlot, FRAME_METADATA_SIZE},
    interner::InternedModuleId,
};
use mono_move_alloc::{GlobalArenaPtr, LeakedBoxPtr};
use std::{fmt, ptr::NonNull};

/// Function's micro-ops.
pub struct Code {
    inner: Box<[MicroOp]>,
}

impl Code {
    /// Builds code from a vector of micro-ops.
    pub fn from_vec(ops: Vec<MicroOp>) -> Self {
        Self {
            inner: ops.into_boxed_slice(),
        }
    }

    /// The function's micro-ops.
    #[inline(always)]
    pub fn get(&self) -> &[MicroOp] {
        &self.inner
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

/// At specific safepoints, this entry supplements the base `frame_layout` with
/// additional pointer offsets that are only valid at that PC.
///
/// Top-frame-only contract: this entry is consulted by the GC only when (a)
/// this function is the top stack frame and (b) its current PC equals
/// `code_offset`.
///
/// `code_offset` must point at an op for which [`MicroOp::is_allocating`]
/// returns `true`.
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
///   [0 .. param_region_size)                                  parameters (written by caller as arguments)
///   [param_region_size .. param_and_local_sizes_sum)          locals
///   [param_and_local_sizes_sum .. param_and_local_sizes_sum+24)     metadata (saved_pc, saved_fp, saved_func_id)
///   [param_and_local_sizes_sum+24 .. extended_frame_size)   callee arg/return slots
/// ```
///
/// `extended_frame_size` == `param_and_local_sizes_sum + FRAME_METADATA_SIZE` for leaf
/// functions (no callee region).
pub struct Function {
    pub name: GlobalArenaPtr<str>,
    pub module_id: InternedModuleId,
    pub code: Code,
    /// Per-parameter (aligned) frame slot, in declaration order.
    pub param_slots: Vec<SizedSlot>,
    /// Byte size of the parameter region (includes padding in between parameters).
    pub param_region_size: usize,
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
    /// parameters (`param_region_size..extended_frame_size`) when a new
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
    ///   runtime zeroes `param_region_size..extended_frame_size` when a
    ///   frame is created, so all non-parameter pointer slots
    ///   (including the callee arg/return region) start as null.
    /// - **Pointer-only writes**: a pointer slot may only be
    ///   overwritten with another valid heap pointer (or null). The
    ///   specializer must guarantee this.
    ///
    /// Callee-region offsets (≥ `frame_size()`) should go in
    /// `safe_point_layouts` (top-frame-only), not here, unless the
    /// value is pointer-shaped at every PC.
    pub frame_layout: FrameLayoutInfo,
    /// Per-safe-point frame layouts.
    ///
    /// **Top-frame-only.** During GC, an entry at `code_offset = pc` is
    /// consulted only when this function is the *top* stack frame and
    /// its current PC equals `pc`. Caller frames below the top use
    /// `frame_layout` (always-on) alone — `gc_collect` does not query
    /// `safe_point_layouts` for any below-frame.
    ///
    /// Each entry's `code_offset` must point at an op for which
    /// [`MicroOp::is_allocating`] returns `true` — the only PCs at
    /// which a top-frame consultation can occur.
    ///
    /// The offsets in each safe-point entry must be disjoint from
    /// `frame_layout.heap_ptr_offsets` — a slot that is always a
    /// pointer belongs in `frame_layout`, not in individual safe-point
    /// entries.
    ///
    /// This supplements `frame_layout` for slots whose pointer status
    /// changes across the function — typically shared callee arg/ret
    /// region offsets that hold pointers only at specific allocating
    /// ops in this function's body.
    ///
    /// Empty when the function needs no per-PC distinction (all
    /// pointer slots are stable across the entire function body).
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

    /// The function's interned name.
    pub fn name(&self) -> &str {
        // SAFETY: any safe `&Function` borrow carries an upstream guarantee
        // that `name`'s pointee remains valid for the borrow's lifetime.
        unsafe { self.name.as_ref_unchecked() }
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "fun {}() {{", self.name())?;
        writeln!(f, "  frame_data_size: {}", self.param_and_local_sizes_sum)?;
        writeln!(f, "  code:")?;
        let code = self.code.get();
        for (i, op) in code.iter().enumerate() {
            writeln!(f, "    {}: {}", i, op)?;
        }
        let entries = self.safe_point_layouts.entries();
        if !entries.is_empty() {
            writeln!(f, "  safe_point_layouts:")?;
            for entry in entries {
                let offsets: Vec<String> = entry
                    .layout
                    .heap_ptr_offsets
                    .iter()
                    .map(|o| o.0.to_string())
                    .collect();
                writeln!(f, "    {}: [{}]", entry.code_offset.0, offsets.join(", "))?;
            }
        }
        writeln!(f, "}}")
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
