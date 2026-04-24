// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    instruction::{CodeOffset, FrameOffset, MicroOp, FRAME_METADATA_SIZE},
    transaction_context::FunctionResolver,
};
use mono_move_alloc::{ExecutableArena, ExecutableArenaPtr, GlobalArenaPtr};

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
    /// (`args_and_locals_size..args_and_locals_size + FRAME_METADATA_SIZE`).
    pub heap_ptr_offsets: ExecutableArenaPtr<[FrameOffset]>,
}

impl FrameLayoutInfo {
    /// Create a `FrameLayoutInfo` from an iterator of pointer offsets,
    /// allocating into the given arena.
    pub fn new<I>(arena: &ExecutableArena, offsets: I) -> Self
    where
        I: IntoIterator<Item = FrameOffset>,
        I::IntoIter: ExactSizeIterator,
    {
        Self {
            heap_ptr_offsets: arena.alloc_slice_fill_iter(offsets),
        }
    }

    /// Create an empty `FrameLayoutInfo` (no pointer offsets).
    pub fn empty(arena: &ExecutableArena) -> Self {
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
    pub layout: FrameLayoutInfo,
}

/// A sorted collection of per-safe-point frame layouts.
///
/// Wraps `ExecutableArenaPtr<[SafePointEntry]>` and provides O(log n)
/// lookup by code offset. Entries must be strictly sorted by
/// `code_offset`.
pub struct SortedSafePointEntries {
    entries: ExecutableArenaPtr<[SafePointEntry]>,
}

impl SortedSafePointEntries {
    /// Create a `SortedSafePointEntries` from an iterator of entries,
    /// allocating into the given arena.
    ///
    /// The caller must ensure entries are strictly sorted by `code_offset`
    /// and that pointer offsets are disjoint from `frame_layout`.
    pub fn new<I>(arena: &ExecutableArena, entries: I) -> Self
    where
        I: IntoIterator<Item = SafePointEntry>,
        I::IntoIter: ExactSizeIterator,
    {
        Self {
            entries: arena.alloc_slice_fill_iter(entries),
        }
    }

    /// Create an empty `SortedSafePointEntries`.
    pub fn empty(arena: &ExecutableArena) -> Self {
        Self::new(arena, std::iter::empty::<SafePointEntry>())
    }

    /// Look up the safe-point layout for a given code offset, if one exists.
    ///
    /// # Safety
    ///
    /// `entries` must be a valid arena pointer.
    pub unsafe fn layout_at(&self, pc: usize) -> Option<&FrameLayoutInfo> {
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
    /// This is required when `frame_layout` has pointer slots so the GC
    /// sees null instead of garbage. Functions with no heap pointer slots
    /// in `frame_layout` (beyond args) can set this to `false` to skip
    /// the memset. Not needed if the function uses only per-PC layouts
    /// and the specializer ensures slots are written before becoming
    /// visible as pointers.
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
    pub unsafe fn safe_point_layout_at(&self, pc: usize) -> Option<&FrameLayoutInfo> {
        unsafe { self.safe_point_layouts.layout_at(pc) }
    }

    /// Replaces every [`MicroOp::CallFunc`] (index-based dispatch) with
    /// [`MicroOp::CallDirect`] (direct pointer dispatch).
    ///
    /// Only used by hand-built test programs. The executable builder uses
    /// its own rewrite pass that handles both local and cross-module calls.
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
                    if let Some(ptr) = func_ptrs[func_id as usize] {
                        *op = MicroOp::CallDirect { ptr };
                    }
                }
            }
        }
    }

    /// Replaces every [`MicroOp::CallIndirect`] (name-based dispatch) with
    /// [`MicroOp::CallDirect`] (direct pointer dispatch) using the
    /// provided function resolver.
    ///
    /// `func_ptrs` yields the functions whose code should be patched.
    ///
    /// # Safety
    ///
    /// The caller must have exclusive access to the functions and their
    /// arena-allocated code.
    pub unsafe fn resolve_module_calls(
        func_ptrs: impl IntoIterator<Item = ExecutableArenaPtr<Function>>,
        resolver: &impl FunctionResolver,
    ) {
        for mut func_ptr in func_ptrs {
            // SAFETY: We have exclusive access during build — no concurrent
            // readers exist yet. The arena is alive because the caller owns it.
            let func = unsafe { func_ptr.as_mut_unchecked() };
            let code = unsafe { func.code.as_mut_unchecked() };
            for op in code.iter_mut() {
                if let MicroOp::CallIndirect {
                    executable_id,
                    func_name,
                } = *op
                {
                    if let Some(ptr) = resolver.resolve_function(executable_id, func_name) {
                        *op = MicroOp::CallDirect { ptr };
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{transaction_context::FunctionResolver, ExecutableId};
    use mono_move_alloc::{ExecutableArena, GlobalArenaPool, GlobalArenaShard};
    use move_core_types::account_address::AccountAddress;

    /// Minimal helper: build a [`Function`] with the given code into `arena`.
    fn make_function(
        arena: &ExecutableArena,
        global: &GlobalArenaShard<'_>,
        name: &str,
        code: Vec<MicroOp>,
    ) -> ExecutableArenaPtr<Function> {
        let name_ptr = global.alloc_str(name);
        let code_ptr = arena.alloc_slice_fill_iter(code);
        let empty_layout = FrameLayoutInfo::new(arena, std::iter::empty::<FrameOffset>());
        let empty_safe_points =
            SortedSafePointEntries::new(arena, std::iter::empty::<SafePointEntry>());
        arena.alloc(Function {
            name: name_ptr,
            code: code_ptr,
            args_size: 0,
            args_and_locals_size: 8,
            extended_frame_size: 8 + FRAME_METADATA_SIZE,
            zero_frame: false,
            frame_layout: empty_layout,
            safe_point_layouts: empty_safe_points,
        })
    }

    /// A test [`FunctionResolver`] that resolves everything to a fixed target.
    struct FixedResolver {
        target: ExecutableArenaPtr<Function>,
    }

    impl FunctionResolver for FixedResolver {
        fn resolve_function(
            &self,
            _executable_id: GlobalArenaPtr<ExecutableId>,
            _name: GlobalArenaPtr<str>,
        ) -> Option<ExecutableArenaPtr<Function>> {
            Some(self.target)
        }
    }

    #[test]
    fn resolve_module_calls_patches_to_call_local_func() {
        let arena = ExecutableArena::new();
        let global_pool = GlobalArenaPool::with_num_arenas(1);
        let global = global_pool.lock_arena(0);

        // Build the target function (just a Return).
        let target = make_function(&arena, &global, "target", vec![MicroOp::Return]);

        // Build interned pointers for the cross-module call.
        let addr = AccountAddress::ONE;
        let exe_name = global.alloc_str("mod_b");
        let exe_id_ptr = global.alloc(unsafe { ExecutableId::new(addr, exe_name) });
        let func_name = global.alloc_str("target");

        // Build the caller with one CallIndirect → Return.
        let caller = make_function(&arena, &global, "caller", vec![
            MicroOp::CallIndirect {
                executable_id: exe_id_ptr,
                func_name,
            },
            MicroOp::Return,
        ]);

        let func_ptrs = [caller];
        let resolver = FixedResolver { target };

        unsafe {
            Function::resolve_module_calls(func_ptrs.iter().copied(), &resolver);
        }

        // Should now be a CallDirect pointing at target.
        let resolved_code = unsafe { func_ptrs[0].as_ref_unchecked().code.as_ref_unchecked() };
        assert!(
            matches!(resolved_code[0], MicroOp::CallDirect { ptr } if ptr.as_non_null() == target.as_non_null()),
            "expected CallDirect pointing to target, got {:?}",
            resolved_code[0]
        );
        assert!(matches!(resolved_code[1], MicroOp::Return));
    }
}
