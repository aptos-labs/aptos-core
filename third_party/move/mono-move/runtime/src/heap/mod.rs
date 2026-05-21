// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Bump-allocated heap with a copying garbage collector (Cheney's algorithm).
//!
//! Exposed as free functions taking explicit parameters (heap, descriptors,
//! pinned roots, root-walk state). This keeps the borrow surface narrow —
//! callers in the interpreter can hold auxiliary borrows (e.g. a `PinGuard`
//! into [`PinnedRoots`]) while still invoking allocation paths that mutate
//! only the heap.

pub(crate) mod object_descriptor;
pub(crate) mod pinned_roots;

use crate::{
    error::{RuntimeError, RuntimeResult},
    heap::{
        object_descriptor::{ObjectDescriptor, ObjectDescriptorInner},
        pinned_roots::PinnedRoots,
    },
    invariant_violation,
    memory::{
        read_descriptor, read_forwarding, read_obj_size, read_ptr, read_u64, write_descriptor,
        write_forwarding, write_obj_size, write_ptr, write_u64, MemoryRegion,
    },
    types::{
        DEFAULT_HEAP_SIZE, FORWARDED_MARKER, META_SAVED_FP_OFFSET, META_SAVED_FUNC_PTR_OFFSET,
        VEC_DATA_OFFSET, VEC_LENGTH_OFFSET,
    },
};
use mono_move_core::{
    align_max, checked_align_max, DescriptorId, FrameOffset, Function, CAPTURED_DATA_VALUES_OFFSET,
    CLOSURE_CAPTURED_DATA_PTR_OFFSET, CLOSURE_DATA_SIZE, ENUM_DATA_OFFSET, ENUM_TAG_OFFSET,
    FRAME_METADATA_SIZE, OBJECT_HEADER_SIZE,
};
use std::ptr::NonNull;

// ---------------------------------------------------------------------------
// Macros
// ---------------------------------------------------------------------------

/// Call-site sugar for the allocation and GC free functions in this module.
///
/// Each macro forwards to an eponymous free function, unpacking the fields
/// of an `InterpreterContext` binding (`heap`, `descriptors`,
/// `pinned_roots`, `current_func`, `pc`, `frame_ptr`) as individual
/// arguments.
///
/// ```ignore
/// let ptr = heap::macros::alloc_obj!(self, fp, desc_id)?;
/// ```
///
/// # Why macros and not methods
///
/// Rust lacks partial-borrow syntax: a method on `InterpreterContext` that
/// mutates `heap` would borrow `self` as `&mut`, conflicting with any
/// outstanding borrow of an unrelated field (e.g. a `PinGuard` that holds
/// `&self.pinned_roots`). Spelling out the individual field borrows at the
/// call site lets the compiler see that the borrows are disjoint. The
/// macro hides that boilerplate while preserving the field-level borrow
/// granularity.
///
/// The macros live in a submodule so their `macro_rules!` names don't
/// collide with the free functions in the value namespace of this module
/// when re-exported.
pub(crate) mod macros {
    /// Forwards to [`super::alloc_obj`] via [`super::alloc_or_gc`].
    /// Arguments: (`$ctx`, `$fp`, `$descriptor_id`).
    macro_rules! alloc_obj {
        ($ctx:ident, $fp:expr, $descriptor_id:expr $(,)?) => {
            $crate::heap::alloc_or_gc(
                &mut $ctx.heap,
                $ctx.descriptors,
                |heap| $crate::heap::alloc_obj(heap, $ctx.descriptors, $descriptor_id),
                $crate::heap::make_scan_roots(&$ctx.pinned_roots, $fp, $ctx.current_func, $ctx.pc),
            )
        };
    }
    pub(crate) use alloc_obj;

    /// Forwards to [`super::alloc_vec`] via [`super::alloc_or_gc`].
    /// Arguments: (`$ctx`, `$fp`, `$descriptor_id`, `$elem_size`,
    /// `$capacity_in_elems`).
    macro_rules! alloc_vec {
        (
            $ctx:ident,
            $fp:expr,
            $descriptor_id:expr,
            $elem_size:expr,
            $capacity_in_elems:expr $(,)?
        ) => {
            $crate::heap::alloc_or_gc(
                &mut $ctx.heap,
                $ctx.descriptors,
                |heap| {
                    $crate::heap::alloc_vec(heap, $descriptor_id, $elem_size, $capacity_in_elems)
                },
                $crate::heap::make_scan_roots(&$ctx.pinned_roots, $fp, $ctx.current_func, $ctx.pc),
            )
        };
    }
    pub(crate) use alloc_vec;

    /// Forwards to [`super::grow_vec_ref`] via
    /// [`super::alloc_or_gc`]. Arguments: (`$ctx`, `$fp`,
    /// `$vec_ref_offset`, `$elem_size`, `$required_cap_in_elems`).
    macro_rules! grow_vec_ref {
        (
            $ctx:ident,
            $fp:expr,
            $vec_ref_offset:expr,
            $elem_size:expr,
            $required_cap_in_elems:expr $(,)?
        ) => {
            $crate::heap::alloc_or_gc(
                &mut $ctx.heap,
                $ctx.descriptors,
                |heap| {
                    $crate::heap::grow_vec_ref(
                        heap,
                        $fp,
                        $vec_ref_offset,
                        $elem_size,
                        $required_cap_in_elems,
                    )
                },
                $crate::heap::make_scan_roots(&$ctx.pinned_roots, $fp, $ctx.current_func, $ctx.pc),
            )
        };
    }
    pub(crate) use grow_vec_ref;
}

#[derive(Debug)]
pub(crate) enum AllocationError {
    /// Heap is full and the requested size did not fit. GC will be triggered
    /// to continue. If after GC there is still not enough space, converted to
    /// runtime error.
    OutOfHeapMemory { requested: usize },
    /// Runtime error, not recoverable.
    RuntimeError(RuntimeError),
}

type AllocationResult<T> = Result<T, AllocationError>;

impl From<RuntimeError> for AllocationError {
    fn from(err: RuntimeError) -> Self {
        Self::RuntimeError(err)
    }
}

impl AllocationError {
    pub fn into_runtime_error(self) -> RuntimeError {
        match self {
            AllocationError::OutOfHeapMemory { requested } => {
                RuntimeError::OutOfHeapMemory { requested }
            },
            AllocationError::RuntimeError(err) => err,
        }
    }
}

// ---------------------------------------------------------------------------
// Allocation orchestration
// ---------------------------------------------------------------------------

/// Run `try_alloc`; on [`AllocationError::OutOfHeapMemory`] run GC with
/// `scan_roots` and retry once. GC is deterministic over the root set so
/// the retry is single-shot.
pub(crate) fn alloc_or_gc(
    heap: &mut Heap,
    descriptors: &[ObjectDescriptor],
    try_alloc: impl Fn(&mut Heap) -> AllocationResult<*mut u8>,
    scan_roots: impl FnOnce(&mut RootScanner<'_>),
) -> RuntimeResult<*mut u8> {
    match try_alloc(heap) {
        Ok(p) => Ok(p),
        Err(AllocationError::OutOfHeapMemory { .. }) => {
            gc_collect(heap, descriptors, scan_roots)?;
            try_alloc(heap).map_err(AllocationError::into_runtime_error)
        },
        Err(e) => Err(e.into_runtime_error()),
    }
}

/// Build the standard root-scanning closure for an interpreter context.
/// Captures the field borrows the GC needs (pinned roots + frame walker)
/// without taking `&self`, so callers can still pass `&mut self.heap`
/// alongside the returned closure.
pub(crate) fn make_scan_roots<'a>(
    pinned_roots: &'a PinnedRoots,
    fp: *mut u8,
    current_func: NonNull<Function>,
    pc: usize,
) -> impl Fn(&mut RootScanner<'_>) + 'a {
    move |scanner| {
        scan_call_stack(scanner, fp, current_func, pc);
        pinned_roots.scan(scanner);
    }
}

// ---------------------------------------------------------------------------
// Heap
// ---------------------------------------------------------------------------

/// Upper bound on a single `heap_alloc` request. Guards against pathological
/// allocation sizes (e.g. a corrupted descriptor or attacker-controlled
/// capacity in `alloc_vec`) before we compute a bump-pointer offset.
///
/// Tied to [`DEFAULT_HEAP_SIZE`] — anything larger could never succeed
/// regardless. The constant exists so the bound can be detached from
/// `DEFAULT_HEAP_SIZE` once heaps become per-context configurable
/// (likely driven by gas limits).
const MAX_SINGLE_ALLOCATION_SIZE: usize = DEFAULT_HEAP_SIZE;

/// True if `bump_ptr + size` still lies within the heap buffer (or one
/// byte past the end). Compares integer addresses to avoid forming an
/// out-of-bounds raw pointer through `add`.
fn fits_in_buffer(heap: &Heap, size: usize) -> bool {
    let bump_addr = heap.bump_ptr as usize;
    let end_addr = heap.buffer.as_ptr() as usize + heap.buffer.len();
    bump_addr
        .checked_add(size)
        .is_some_and(|end| end <= end_addr)
}

/// Heap state for a single interpreter context.
pub struct Heap {
    /// Backing memory. Swapped with a fresh `MemoryRegion` on every GC
    /// (`to_space` becomes the new `buffer`).
    pub(crate) buffer: MemoryRegion,
    /// Next free byte in `buffer` — i.e., the start of the next header
    /// reservation. Object pointers returned to callers are
    /// `bump_ptr + OBJECT_HEADER_SIZE` (data-region start). Invariant:
    /// always within `[buffer.as_ptr(), buffer.as_ptr() + buffer.len()]`
    /// and `MAX_ALIGN`-aligned.
    pub(crate) bump_ptr: *mut u8,
    /// Number of times the GC has run. Exposed for tests / diagnostics.
    pub(crate) gc_count: usize,
}

impl Heap {
    pub fn new(size: usize) -> Self {
        let buffer = MemoryRegion::new(size);
        Self {
            bump_ptr: buffer.as_ptr(),
            buffer,
            gc_count: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Allocation
// ---------------------------------------------------------------------------

/// Allocate `total_size` bytes on the heap and initialize the header.
/// `total_size` covers header + payload; the function rounds it up to
/// `MAX_ALIGN`, allocates, and writes both header fields (`descriptor_id`
/// and the aligned size) before returning the **object pointer** — the
/// data-region start at `header_start + OBJECT_HEADER_SIZE`. Callers
/// only need to populate the data region (e.g. vector length, captured
/// values).
///
/// Returns [`AllocationError::OutOfHeapMemory`] when the heap is full so the
/// caller can run GC and retry.
fn heap_alloc(
    heap: &mut Heap,
    total_size: usize,
    descriptor_id: DescriptorId,
) -> AllocationResult<*mut u8> {
    // Round up to MAX_ALIGN with overflow protection; the
    // `MAX_SINGLE_ALLOCATION_SIZE` check below then applies to the
    // *aligned* size so the rounding can't smuggle an oversize allocation
    // past the bound.
    let aligned_size = checked_align_max(total_size).ok_or_else(|| {
        AllocationError::from(RuntimeError::AllocationTooLarge {
            requested: total_size,
        })
    })?;
    debug_assert!(aligned_size >= OBJECT_HEADER_SIZE);
    if aligned_size > MAX_SINGLE_ALLOCATION_SIZE {
        return Err(AllocationError::from(RuntimeError::AllocationTooLarge {
            requested: aligned_size,
        }));
    }

    // Bound check uses integer arithmetic on addresses rather than
    // `bump_ptr.add(aligned_size)` against `buffer.as_ptr().add(buffer.len())`:
    // pointer arithmetic that produces a result more than one byte past
    // the end of the buffer is UB, so we cannot legally form the
    // out-of-bounds pointer just to compare it.
    if !fits_in_buffer(heap, aligned_size) {
        return Err(AllocationError::OutOfHeapMemory {
            requested: aligned_size,
        });
    }

    unsafe {
        let header_start = heap.bump_ptr;
        heap.bump_ptr = header_start.add(aligned_size);
        std::ptr::write_bytes(header_start, 0, aligned_size);
        let obj_ptr = header_start.add(OBJECT_HEADER_SIZE);
        write_descriptor(obj_ptr, descriptor_id.as_u32());
        write_obj_size(obj_ptr, aligned_size as u32);
        Ok(obj_ptr)
    }
}

/// Allocate a new vector object on the heap.
///
/// `capacity_in_elems` is the number of elements the vector can hold
/// before needing to grow — *not* a byte size. Total bytes allocated
/// are roughly `capacity_in_elems * elem_size + OBJECT_HEADER_SIZE +
/// VEC_DATA_OFFSET`, rounded up to `MAX_ALIGN`.
pub(crate) fn alloc_vec(
    heap: &mut Heap,
    descriptor_id: DescriptorId,
    elem_size: u32,
    capacity_in_elems: u64,
) -> AllocationResult<*mut u8> {
    let total_size = (capacity_in_elems as usize)
        .checked_mul(elem_size as usize)
        .and_then(|v| v.checked_add(OBJECT_HEADER_SIZE + VEC_DATA_OFFSET))
        .ok_or_else(|| {
            AllocationError::from(RuntimeError::AllocationTooLarge {
                requested: usize::MAX,
            })
        })?;
    heap_alloc(heap, total_size, descriptor_id)
    // `length` defaults to 0 via heap_alloc's zero-init.
}

/// Allocate a new zeroed heap object (struct or enum). Size comes from the
/// descriptor at `descriptor_id`.
pub(crate) fn alloc_obj(
    heap: &mut Heap,
    descriptors: &[ObjectDescriptor],
    descriptor_id: DescriptorId,
) -> AllocationResult<*mut u8> {
    let payload_size = match descriptors[descriptor_id.as_usize()].inner() {
        ObjectDescriptorInner::Struct { size, .. } => *size as usize,
        ObjectDescriptorInner::Enum { size, .. } => *size as usize,
        ObjectDescriptorInner::Closure => CLOSURE_DATA_SIZE,
        ObjectDescriptorInner::CapturedData { size, .. } => {
            // Add the 8-byte tag+padding prefix to the values-region size.
            CAPTURED_DATA_VALUES_OFFSET + *size as usize
        },
        ObjectDescriptorInner::Trivial | ObjectDescriptorInner::Vector { .. } => {
            return Err(AllocationError::from(RuntimeError::InvariantViolation(
                crate::error::RuntimeInvariantViolation::NonAllocatableDescriptor {
                    descriptor_id: descriptor_id.as_u32(),
                },
            )));
        },
    };
    heap_alloc(heap, OBJECT_HEADER_SIZE + payload_size, descriptor_id)
}

/// Grow a vector to at least `required_cap_in_elems` elements, accessed
/// through a fat pointer reference at `fp + vec_ref_offset`. The vector
/// pointer is written back through the reference; returns the new object
/// pointer.
///
/// Returns [`AllocationError::OutOfHeapMemory`] when the heap is full so the
/// caller can run GC and retry.
///
/// # Safety
///
/// `fp` must point to a valid frame. `vec_ref_offset` must be the byte
/// offset of a 16-byte fat pointer `(base, offset)` whose target holds
/// the current vector heap pointer.
pub(crate) fn grow_vec_ref(
    heap: &mut Heap,
    fp: *mut u8,
    vec_ref_offset: usize,
    elem_size: u32,
    required_cap_in_elems: u64,
) -> AllocationResult<*mut u8> {
    unsafe {
        let base = read_ptr(fp, vec_ref_offset);
        let off = read_u64(fp, vec_ref_offset + 8) as usize;
        let old_ptr = read_ptr(base, off);

        let old_len = read_u64(old_ptr, VEC_LENGTH_OFFSET);
        let old_total = read_obj_size(old_ptr) as usize;
        let old_cap_in_elems =
            ((old_total - OBJECT_HEADER_SIZE - VEC_DATA_OFFSET) / elem_size as usize) as u64;
        let descriptor_id = DescriptorId(read_descriptor(old_ptr));

        let mut new_cap_in_elems = if old_cap_in_elems == 0 {
            4
        } else {
            old_cap_in_elems * 2
        };
        if new_cap_in_elems < required_cap_in_elems {
            new_cap_in_elems = required_cap_in_elems;
        }

        let new_ptr = alloc_vec(heap, descriptor_id, elem_size, new_cap_in_elems)?;

        let byte_count = old_len as usize * elem_size as usize;
        if byte_count > 0 {
            std::ptr::copy_nonoverlapping(
                old_ptr.add(VEC_DATA_OFFSET),
                new_ptr.add(VEC_DATA_OFFSET),
                byte_count,
            );
        }
        write_u64(new_ptr, VEC_LENGTH_OFFSET, old_len);

        // Write new pointer back through the reference.
        write_ptr(base, off, new_ptr);
        Ok(new_ptr)
    }
}

// ---------------------------------------------------------------------------
// Root scanner
// ---------------------------------------------------------------------------

/// Phase-1 root-walking interface for the GC. Built by [`gc_collect`]
/// and passed to the caller's `scan_roots` closure. Each root source
/// iterates its slots and calls [`Self::relocate`] per pointer,
/// writing the returned value back into the slot.
pub struct RootScanner<'a> {
    /// Used by [`is_heap_ptr`] to decide whether a pointer is a
    /// from-space heap pointer (and thus needs relocation) or an
    /// external pointer (passed through unchanged).
    heap: &'a Heap,
    /// Bump cursor in to-space. Advances by `obj_size` per relocation.
    free_ptr: &'a mut *mut u8,
}

impl<'a> RootScanner<'a> {
    /// Relocate a possibly-from-space pointer to its to-space
    /// address. If `ptr` is null or not a heap pointer (e.g. an
    /// external resource pointer), it is returned unchanged.
    ///
    /// Callers must uphold the object-header-integrity invariant
    /// documented on [`gc_collect`]: if `ptr` is in-heap, its
    /// header at `ptr - OBJECT_HEADER_SIZE` must be valid.
    pub fn relocate(&mut self, ptr: *mut u8) -> *mut u8 {
        if ptr.is_null() || !is_heap_ptr(self.heap, ptr) {
            return ptr;
        }
        gc_copy_object(ptr, self.free_ptr)
    }

    /// Test-only constructor. Production code only sees a
    /// `RootScanner` handed in by [`gc_collect`].
    #[cfg(test)]
    pub(crate) fn for_test(heap: &'a Heap, free_ptr: &'a mut *mut u8) -> RootScanner<'a> {
        RootScanner { heap, free_ptr }
    }
}

/// Scan call-stack roots: the top frame's `frame_layout` plus the
/// matching `safe_point_layouts` entry (if any), then every caller
/// frame's `frame_layout`. Each pointer offset is read, relocated
/// via the scanner, and written back.
pub fn scan_call_stack(
    scanner: &mut RootScanner<'_>,
    fp: *mut u8,
    current_func: NonNull<Function>,
    pc_top: usize,
) {
    // Two sets of pointer offsets are scanned for the top frame:
    //   1. `frame_layout.heap_ptr_offsets` — always applies, to
    //      every frame on the stack.
    //   2. The matching `safe_point_layouts` entry for the *top*
    //      frame's `pc_top`, if any — provides additional pointer
    //      offsets that are only valid at that specific top-frame
    //      safe point (an allocating op currently executing in the
    //      top frame).
    //
    // Per `SafePointEntry`'s top-frame-only contract,
    // `safe_point_layouts` is consulted only for the top frame;
    // caller-below frames use `frame_layout` alone.
    // SAFETY: current_func is valid (caller's invariant).
    let top_func = unsafe { current_func.as_ref() };
    unsafe {
        gc_scan_frame_roots(scanner, fp, &top_func.frame_layout.heap_ptr_offsets);
        if let Some(sp_layout) = top_func.safe_point_layout_at(pc_top) {
            gc_scan_frame_roots(scanner, fp, &sp_layout.heap_ptr_offsets);
        }
    }

    // Walk caller frames with frame_layout only.
    let mut fp = fp;
    loop {
        // SAFETY: `fp.sub(FRAME_METADATA_SIZE)` reads metadata written
        // by the call protocol when this frame's callee was created.
        // Arena pointers are valid for the executable's lifetime.
        let meta = unsafe { fp.sub(FRAME_METADATA_SIZE) };
        let saved_func_ptr =
            unsafe { read_ptr(meta, META_SAVED_FUNC_PTR_OFFSET) } as *const Function;
        if saved_func_ptr.is_null() {
            break;
        }
        fp = unsafe { read_ptr(meta, META_SAVED_FP_OFFSET) };
        // SAFETY: non-null checked above.
        let caller_func_ptr = unsafe { NonNull::new_unchecked(saved_func_ptr as *mut Function) };
        let caller_func = unsafe { caller_func_ptr.as_ref() };
        unsafe {
            gc_scan_frame_roots(scanner, fp, &caller_func.frame_layout.heap_ptr_offsets);
        }
    }
}

// ---------------------------------------------------------------------------
// Garbage collection
// ---------------------------------------------------------------------------

/// Run Cheney's copying GC. The caller's `scan_roots` closure walks
/// every root source (call stack, pinned roots, etc.) and routes each
/// pointer through [`RootScanner::relocate`]. The GC then does a
/// breadth-first copy of all reachable objects.
///
/// # Safety assumptions
///
/// Correctness relies on the following invariants maintained by the
/// interpreter and the micro-op verifier:
///
/// - **Frame metadata integrity**: each frame's saved `fp`, `func_ptr`,
///   and `pc` are written by [`MicroOp::Return`], [`MicroOp::CallDirect`],
///   [`MicroOp::CallIndirect`] and never modified by user-visible micro-ops.
///   A corrupted saved `fp` leads to out-of-bounds stack reads (UB).
/// - **Pointer-slot accuracy**: `Function::frame_layout` (and the
///   matching `safe_point_layouts` entry, if any) together list every
///   frame offset that may hold a live heap pointer, and *only* those
///   offsets. Missing entries → dangling pointers after GC; extra
///   entries → non-pointer data reinterpreted as a pointer (UB).
/// - **Object header integrity**: the `descriptor_id` and `size` fields
///   in every heap object header are set by the allocator and never
///   overwritten by user code. Corrupted headers → wrong copy size or
///   wrong reference tracing (UB).
pub(crate) fn gc_collect(
    heap: &mut Heap,
    descriptors: &[ObjectDescriptor],
    scan_roots: impl FnOnce(&mut RootScanner<'_>),
) -> RuntimeResult<()> {
    heap.gc_count += 1;

    let to_space = MemoryRegion::new(heap.buffer.len());
    // `free_ptr` is a raw bump cursor — it points at the start of the
    // next *header* reservation, advancing by each object's total size.
    // Treating it as a raw cursor (rather than as an "object pointer"
    // biased by OBJECT_HEADER_SIZE) keeps its maximum value at
    // `to_space.end()`, which is the one-past-end address that pointer
    // arithmetic permits. A `+ H` bias would let it overshoot when all
    // of from-space survives, producing UB on `.add` / `.sub`.
    let mut free_ptr = to_space.as_ptr();

    // Phase 1: hand the scanner to the caller's closure. The
    // closure walks every root source (call stack, pinned roots,
    // and future ones like global-storage writes) and routes each
    // pointer through `RootScanner::relocate`.
    {
        let mut scanner = RootScanner {
            heap,
            free_ptr: &mut free_ptr,
        };
        scan_roots(&mut scanner);
    }

    // Phase 2: Cheney-style breadth-first scan of copied objects.
    // `scan_ptr` is a raw cursor — header start of the next object to
    // scan. Object pointers (data starts) are `scan_ptr + H`.
    let mut scan_ptr = to_space.as_ptr();
    while (scan_ptr as usize) < (free_ptr as usize) {
        // SAFETY: scan_ptr advances through to-space by each object's
        // aligned size. Object headers were copied verbatim by
        // gc_copy_object, so descriptor_id and size are valid as long
        // as the object-header-integrity invariant holds (see above).
        unsafe {
            let obj_ptr = scan_ptr.add(OBJECT_HEADER_SIZE);
            let descriptor_id = read_descriptor(obj_ptr);
            let obj_size = read_obj_size(obj_ptr) as usize;

            if obj_size == 0 || obj_size != align_max(obj_size) {
                invariant_violation!(GcInvalidObjectSize { size: obj_size });
            }

            if descriptor_id == FORWARDED_MARKER {
                invariant_violation!(GcForwardingMarkerInToSpace);
            }
            gc_scan_object(descriptors, heap, obj_ptr, descriptor_id, &mut free_ptr);

            scan_ptr = scan_ptr.add(obj_size);
        }
    }

    // Phase 3: swap — drop old heap, adopt new one. The bump cursor
    // semantics match `free_ptr` directly (both are raw header-start
    // cursors).
    heap.buffer = to_space;
    heap.bump_ptr = free_ptr;
    Ok(())
}

/// Returns true if `ptr` is a valid object pointer (data-region start)
/// in the from-space heap buffer.
///
/// Valid obj_ptrs lie in `[start + OBJECT_HEADER_SIZE, end]`:
/// - The header at `obj_ptr - OBJECT_HEADER_SIZE` must be in-bounds
///   (lower bound).
/// - The data region `[obj_ptr, obj_ptr + payload)` must be in-bounds;
///   `obj_ptr == end` is permitted for a zero-payload object whose
///   header occupies the very last bytes of the buffer (upper bound).
///
/// Current descriptor validation enforces `payload_size > 0`, so
/// `obj_ptr == end` is unreachable in practice. The bound is written
/// to match the semantic invariant so it stays correct under future
/// changes.
fn is_heap_ptr(heap: &Heap, ptr: *const u8) -> bool {
    let start = heap.buffer.as_ptr() as usize;
    let end = start + heap.buffer.len();
    let p = ptr as usize;
    p >= start + OBJECT_HEADER_SIZE && p <= end
}

/// Scan a set of pointer offsets in a frame, copying any heap objects
/// they reference into to-space and updating the frame slots.
///
/// # Safety
///
/// - `fp` must point to a valid frame.
/// - Each entry in `offsets` must be a valid 8-byte-aligned offset within
///   the frame's extended size.
unsafe fn gc_scan_frame_roots(scanner: &mut RootScanner<'_>, fp: *mut u8, offsets: &[FrameOffset]) {
    unsafe {
        for &offset in offsets {
            let old_ptr = read_ptr(fp, offset);
            if !old_ptr.is_null() {
                let new_ptr = scanner.relocate(old_ptr);
                write_ptr(fp, offset, new_ptr);
            }
        }
    }
}

/// Copy a single object from the old heap into to-space at `*free_ptr`,
/// writing a forwarding pointer in the old location. If the object is
/// already forwarded, just return the forwarding address.
///
/// `old_ptr` and the returned pointer are *object pointers* (data-region
/// starts); the header lives at `obj_ptr - OBJECT_HEADER_SIZE`. The
/// forwarding pointer is parked at offset 0 of the data region (over the
/// old object's first 8 payload bytes), and the descriptor field at
/// `obj_ptr - 8` is overwritten with `FORWARDED_MARKER`.
///
/// `*free_ptr` is a raw bump cursor in to-space (header start of the
/// next slot); after this call it advances by `obj_size`.
///
/// # Safety
///
/// - `old_ptr` must point to the data region of a live object in
///   from-space. Its `descriptor_id` and `size` header fields must be
///   valid (see object-header-integrity invariant on [`gc_collect`]).
/// - `*free_ptr` must point into to-space with at least `obj_size` bytes
///   (header + payload) of room ahead.
/// - The from-space object must not have been partially overwritten
///   except by a prior call to this function (which installs a
///   forwarding marker).
fn gc_copy_object(old_ptr: *mut u8, free_ptr: &mut *mut u8) -> *mut u8 {
    unsafe {
        let descriptor_id = read_descriptor(old_ptr);

        if descriptor_id == FORWARDED_MARKER {
            return read_forwarding(old_ptr);
        }

        let obj_size = read_obj_size(old_ptr) as usize;
        debug_assert!(
            obj_size >= OBJECT_HEADER_SIZE && obj_size == align_max(obj_size),
            "gc_copy_object: invalid object size {}",
            obj_size
        );

        // Move the `[header | payload]` block from from-space to
        // to-space, then advance the raw bump cursor by `obj_size`. The
        // returned obj_ptr is `new_header_start + H`.
        let new_header_start = *free_ptr;
        std::ptr::copy_nonoverlapping(old_ptr.sub(OBJECT_HEADER_SIZE), new_header_start, obj_size);
        *free_ptr = new_header_start.add(obj_size);
        let new_ptr = new_header_start.add(OBJECT_HEADER_SIZE);

        write_descriptor(old_ptr, FORWARDED_MARKER);
        write_forwarding(old_ptr, new_ptr);

        new_ptr
    }
}

/// Scan a copied object in to-space for internal heap references and
/// copy the referenced objects as well (Cheney forwarding).
///
/// # Safety
///
/// - `obj_ptr` must point to a valid, fully-copied object in to-space.
/// - `descriptor_id` must match the object's actual type.
/// - The `ObjectDescriptor` at `descriptor_id` must accurately describe
///   the reference layout of the object.
/// - `free_ptr` must point to the next free byte in to-space with enough
///   room for any objects that will be copied.
fn gc_scan_object(
    descriptors: &[ObjectDescriptor],
    heap: &Heap,
    obj_ptr: *mut u8,
    descriptor_id: u32,
    free_ptr: &mut *mut u8,
) {
    debug_assert!(
        (descriptor_id as usize) < descriptors.len(),
        "gc_scan_object: descriptor_id {} out of bounds (have {} descriptors)",
        descriptor_id,
        descriptors.len()
    );
    let desc = if (descriptor_id as usize) < descriptors.len() {
        &descriptors[descriptor_id as usize]
    } else {
        return;
    };

    // All offsets below are relative to `obj_ptr` (the data-region
    // start). Struct field offsets are used directly as obj_ptr-relative
    // byte offsets; `ENUM_DATA_OFFSET` / `VEC_DATA_OFFSET` /
    // `CAPTURED_DATA_VALUES_OFFSET` are the small fixed prefixes for the
    // tag / length / tag+padding.
    match desc.inner() {
        ObjectDescriptorInner::Trivial => {},
        ObjectDescriptorInner::Vector {
            elem_size,
            elem_pointer_offsets,
        } => {
            debug_assert!(*elem_size > 0, "elem_size must not be zero");
            if elem_pointer_offsets.is_empty() {
                return;
            }
            unsafe {
                let length = read_u64(obj_ptr, VEC_LENGTH_OFFSET) as usize;
                let data_start = obj_ptr.add(VEC_DATA_OFFSET);

                for i in 0..length {
                    let elem_base = data_start.add(i * (*elem_size as usize));
                    for &ptr_off in elem_pointer_offsets {
                        let old_ptr = read_ptr(elem_base, ptr_off as usize);
                        if !old_ptr.is_null() && is_heap_ptr(heap, old_ptr) {
                            let new_ptr = gc_copy_object(old_ptr, free_ptr);
                            write_ptr(elem_base, ptr_off as usize, new_ptr);
                        }
                    }
                }
            }
        },
        ObjectDescriptorInner::Struct {
            pointer_offsets, ..
        } => {
            if pointer_offsets.is_empty() {
                return;
            }
            unsafe {
                for &off in pointer_offsets {
                    let old_ptr = read_ptr(obj_ptr, off as usize);
                    if !old_ptr.is_null() && is_heap_ptr(heap, old_ptr) {
                        let new_ptr = gc_copy_object(old_ptr, free_ptr);
                        write_ptr(obj_ptr, off as usize, new_ptr);
                    }
                }
            }
        },
        ObjectDescriptorInner::Enum {
            variant_pointer_offsets,
            ..
        } => unsafe {
            let tag = read_u64(obj_ptr, ENUM_TAG_OFFSET) as usize;
            if tag >= variant_pointer_offsets.len() {
                return;
            }
            let pointer_offsets = &variant_pointer_offsets[tag];
            if pointer_offsets.is_empty() {
                return;
            }
            for &off in pointer_offsets {
                let old_ptr = read_ptr(obj_ptr, ENUM_DATA_OFFSET + off as usize);
                if !old_ptr.is_null() && is_heap_ptr(heap, old_ptr) {
                    let new_ptr = gc_copy_object(old_ptr, free_ptr);
                    write_ptr(obj_ptr, ENUM_DATA_OFFSET + off as usize, new_ptr);
                }
            }
        },
        ObjectDescriptorInner::Closure => unsafe {
            // The closure's only heap pointer is `captured_data_ptr` at a
            // fixed data-region offset.
            let off = CLOSURE_CAPTURED_DATA_PTR_OFFSET;
            let old_ptr = read_ptr(obj_ptr, off);
            if !old_ptr.is_null() && is_heap_ptr(heap, old_ptr) {
                let new_ptr = gc_copy_object(old_ptr, free_ptr);
                write_ptr(obj_ptr, off, new_ptr);
            }
        },
        ObjectDescriptorInner::CapturedData {
            pointer_offsets, ..
        } => {
            if pointer_offsets.is_empty() {
                return;
            }
            unsafe {
                for &off in pointer_offsets {
                    let abs_off = CAPTURED_DATA_VALUES_OFFSET + off as usize;
                    let old_ptr = read_ptr(obj_ptr, abs_off);
                    if !old_ptr.is_null() && is_heap_ptr(heap, old_ptr) {
                        let new_ptr = gc_copy_object(old_ptr, free_ptr);
                        write_ptr(obj_ptr, abs_off, new_ptr);
                    }
                }
            }
        },
    }
}
