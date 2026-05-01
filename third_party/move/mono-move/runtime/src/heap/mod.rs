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
    bail,
    error::ExecutionResult,
    heap::object_descriptor::{ObjectDescriptor, ObjectDescriptorInner},
    memory::{read_ptr, read_u32, read_u64, write_ptr, write_u32, write_u64, MemoryRegion},
    types::{
        DEFAULT_HEAP_SIZE, FORWARDED_MARKER, HEADER_DESCRIPTOR_OFFSET, HEADER_SIZE_OFFSET,
        META_SAVED_FP_OFFSET, META_SAVED_FUNC_PTR_OFFSET, META_SAVED_PC_OFFSET, VEC_DATA_OFFSET,
        VEC_LENGTH_OFFSET,
    },
};
use mono_move_core::{
    DescriptorId, FrameOffset, Function, CAPTURED_DATA_VALUES_OFFSET,
    CLOSURE_CAPTURED_DATA_PTR_OFFSET, CLOSURE_OBJECT_SIZE, ENUM_DATA_OFFSET, ENUM_TAG_OFFSET,
    FRAME_METADATA_SIZE, OBJECT_HEADER_SIZE, STRUCT_DATA_OFFSET,
};
use pinned_roots::PinnedRoots;
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
    /// Forwards to [`super::alloc_obj`]. Arguments: (`$ctx`, `$fp`,
    /// `$descriptor_id`).
    macro_rules! alloc_obj {
        ($ctx:ident, $fp:expr, $descriptor_id:expr $(,)?) => {
            $crate::heap::alloc_obj(
                &mut $ctx.heap,
                $ctx.descriptors,
                &$ctx.pinned_roots,
                $fp,
                $ctx.current_func,
                $ctx.pc,
                $descriptor_id,
            )
        };
    }
    pub(crate) use alloc_obj;

    /// Forwards to [`super::alloc_vec`]. Arguments: (`$ctx`, `$fp`,
    /// `$descriptor_id`, `$elem_size`, `$capacity`).
    macro_rules! alloc_vec {
        ($ctx:ident, $fp:expr, $descriptor_id:expr, $elem_size:expr, $capacity:expr $(,)?) => {
            $crate::heap::alloc_vec(
                &mut $ctx.heap,
                $ctx.descriptors,
                &$ctx.pinned_roots,
                $fp,
                $ctx.current_func,
                $ctx.pc,
                $descriptor_id,
                $elem_size,
                $capacity,
            )
        };
    }
    pub(crate) use alloc_vec;

    /// Forwards to [`super::grow_vec_ref`]. Arguments: (`$ctx`, `$fp`,
    /// `$vec_ref_offset`, `$elem_size`, `$required_cap`).
    macro_rules! grow_vec_ref {
        ($ctx:ident, $fp:expr, $vec_ref_offset:expr, $elem_size:expr, $required_cap:expr $(,)?) => {
            $crate::heap::grow_vec_ref(
                &mut $ctx.heap,
                $ctx.descriptors,
                &$ctx.pinned_roots,
                $ctx.current_func,
                $ctx.pc,
                $fp,
                $vec_ref_offset,
                $elem_size,
                $required_cap,
            )
        };
    }
    pub(crate) use grow_vec_ref;

    /// Forwards to [`super::gc_collect`]. Arguments: (`$ctx`,).
    macro_rules! gc_collect {
        ($ctx:ident $(,)?) => {
            $crate::heap::gc_collect(
                &mut $ctx.heap,
                $ctx.descriptors,
                &$ctx.pinned_roots,
                $ctx.frame_ptr,
                $ctx.current_func,
                $ctx.pc,
            )
        };
    }
    pub(crate) use gc_collect;
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

/// Round `size` up to the next multiple of 8.
//
// TODO: the 8-byte alignment here is a placeholder that matches the
// current assumption that every field is 8-byte aligned. Once the object
// layout admits other widths (u32 / u16 / u128), this needs to take the
// object's max-alignment into account — either from the descriptor or
// from a per-op alignment hint.
fn align8(size: usize) -> usize {
    (size + 7) & !7
}

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
    /// Next free byte in `buffer`. Invariant: always within
    /// `[buffer.as_ptr(), buffer.as_ptr() + buffer.len()]`.
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

/// Allocate `size` bytes (8-byte aligned) on the heap.
/// Triggers GC if the bump allocator is full; fails on OOM after GC.
///
/// `fp`, `current_func`, `pc` describe the top of the call stack so that GC
/// (if triggered) can walk frames to find roots. `pinned_roots` is the
/// auxiliary root set.
pub(crate) fn heap_alloc(
    heap: &mut Heap,
    descriptors: &[ObjectDescriptor],
    pinned_roots: &PinnedRoots,
    fp: *mut u8,
    current_func: NonNull<Function>,
    pc: usize,
    size: usize,
) -> ExecutionResult<*mut u8> {
    if size == 0 {
        bail!("heap_alloc: size must not be zero");
    }
    if size > MAX_SINGLE_ALLOCATION_SIZE {
        bail!(
            "heap_alloc: size {} exceeds maximum single allocation size",
            size
        );
    }
    let aligned = align8(size);

    // Bound check uses integer arithmetic on addresses rather than
    // `bump_ptr.add(aligned)` against `buffer.as_ptr().add(buffer.len())`:
    // pointer arithmetic that produces a result more than one byte past
    // the end of the buffer is UB, so we cannot legally form the
    // out-of-bounds pointer just to compare it.
    if !fits_in_buffer(heap, aligned) {
        gc_collect(heap, descriptors, pinned_roots, fp, current_func, pc)?;
        if !fits_in_buffer(heap, aligned) {
            bail!("out of heap memory after GC (requested {} bytes)", size);
        }
    }

    unsafe {
        let ptr = heap.bump_ptr;
        heap.bump_ptr = ptr.add(aligned);
        std::ptr::write_bytes(ptr, 0, aligned);
        Ok(ptr)
    }
}

/// Allocate a new vector object on the heap with the given parameters.
pub(crate) fn alloc_vec(
    heap: &mut Heap,
    descriptors: &[ObjectDescriptor],
    pinned_roots: &PinnedRoots,
    fp: *mut u8,
    current_func: NonNull<Function>,
    pc: usize,
    descriptor_id: DescriptorId,
    elem_size: u32,
    capacity: u64,
) -> ExecutionResult<*mut u8> {
    let total_size = (capacity as usize)
        .checked_mul(elem_size as usize)
        .and_then(|v| v.checked_add(VEC_DATA_OFFSET))
        .ok_or_else(|| anyhow::anyhow!("alloc_vec: size overflow"))?;
    let aligned_size = align8(total_size);
    let ptr = heap_alloc(
        heap,
        descriptors,
        pinned_roots,
        fp,
        current_func,
        pc,
        total_size,
    )?;
    unsafe {
        write_u32(ptr, HEADER_DESCRIPTOR_OFFSET, descriptor_id.as_u32());
        write_u32(ptr, HEADER_SIZE_OFFSET, aligned_size as u32);
        write_u64(ptr, VEC_LENGTH_OFFSET, 0);
    }
    Ok(ptr)
}

/// Allocate a new zeroed heap object (struct or enum). Size comes from the
/// descriptor at `descriptor_id`.
pub(crate) fn alloc_obj(
    heap: &mut Heap,
    descriptors: &[ObjectDescriptor],
    pinned_roots: &PinnedRoots,
    fp: *mut u8,
    current_func: NonNull<Function>,
    pc: usize,
    descriptor_id: DescriptorId,
) -> ExecutionResult<*mut u8> {
    let payload_size = match descriptors[descriptor_id.as_usize()].inner() {
        ObjectDescriptorInner::Struct { size, .. } => *size as usize,
        ObjectDescriptorInner::Enum { size, .. } => *size as usize,
        ObjectDescriptorInner::Closure => CLOSURE_OBJECT_SIZE - OBJECT_HEADER_SIZE,
        ObjectDescriptorInner::CapturedData { size, .. } => {
            // Add the 8-byte tag+padding prefix to the values-region size.
            (CAPTURED_DATA_VALUES_OFFSET - OBJECT_HEADER_SIZE) + *size as usize
        },
        ObjectDescriptorInner::Trivial | ObjectDescriptorInner::Vector { .. } => bail!(
            "alloc_obj called with non-allocatable descriptor {}",
            descriptor_id
        ),
    };
    let total_size = OBJECT_HEADER_SIZE + payload_size;
    let aligned_size = align8(total_size);
    let ptr = heap_alloc(
        heap,
        descriptors,
        pinned_roots,
        fp,
        current_func,
        pc,
        total_size,
    )?;
    unsafe {
        write_u32(ptr, HEADER_DESCRIPTOR_OFFSET, descriptor_id.as_u32());
        write_u32(ptr, HEADER_SIZE_OFFSET, aligned_size as u32);
    }
    Ok(ptr)
}

/// Grow a vector to at least `required_cap` elements, accessed through
/// a fat pointer reference at `fp + vec_ref_offset`. The vector pointer
/// is written back through the reference; returns the new object pointer.
///
/// # Safety
///
/// `fp` must point to a valid frame. `vec_ref_offset` must be the byte
/// offset of a 16-byte fat pointer `(base, offset)` whose target holds
/// the current vector heap pointer. `alloc_vec` may trigger GC which
/// relocates objects; the fat pointer's base in `heap_ptr_offsets` and the
/// vector pointer in the struct's `pointer_offsets` are updated by the GC.
/// We re-read through the fat pointer after allocation.
pub(crate) fn grow_vec_ref(
    heap: &mut Heap,
    descriptors: &[ObjectDescriptor],
    pinned_roots: &PinnedRoots,
    current_func: NonNull<Function>,
    pc: usize,
    fp: *mut u8,
    vec_ref_offset: usize,
    elem_size: u32,
    required_cap: u64,
) -> ExecutionResult<*mut u8> {
    unsafe {
        let base = read_ptr(fp, vec_ref_offset);
        let off = read_u64(fp, vec_ref_offset + 8) as usize;
        let old_ptr = read_ptr(base, off);

        let old_len = read_u64(old_ptr, VEC_LENGTH_OFFSET);
        let old_size = read_u32(old_ptr, HEADER_SIZE_OFFSET) as usize;
        let old_cap = ((old_size - VEC_DATA_OFFSET) / elem_size as usize) as u64;
        let descriptor_id = DescriptorId(read_u32(old_ptr, HEADER_DESCRIPTOR_OFFSET));

        let mut new_cap = if old_cap == 0 { 4 } else { old_cap * 2 };
        if new_cap < required_cap {
            new_cap = required_cap;
        }

        // alloc_vec may trigger GC. Re-read through the fat pointer afterward.
        let new_ptr = alloc_vec(
            heap,
            descriptors,
            pinned_roots,
            fp,
            current_func,
            pc,
            descriptor_id,
            elem_size,
            new_cap,
        )?;
        let base = read_ptr(fp, vec_ref_offset);
        let off = read_u64(fp, vec_ref_offset + 8) as usize;
        let old_ptr = read_ptr(base, off);

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
// Garbage collection
// ---------------------------------------------------------------------------

/// Run Cheney's copying GC. Walks the call stack to find roots via
/// per-function pointer slot lists, scans [`PinnedRoots`], then does a
/// breadth-first copy of all reachable objects.
///
/// # Safety assumptions
///
/// Correctness relies on the following invariants maintained by the
/// interpreter and the micro-op verifier:
///
/// - **Frame metadata integrity**: each frame's saved `fp`, `func_ptr`,
///   and `pc` are written by `CallFunc`/`Return` and never modified by
///   user-visible micro-ops. A corrupted saved `fp` leads to
///   out-of-bounds stack reads (UB).
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
    pinned_roots: &PinnedRoots,
    frame_ptr: *mut u8,
    current_func: NonNull<Function>,
    pc_top: usize,
) -> ExecutionResult<()> {
    heap.gc_count += 1;

    let to_space = MemoryRegion::new(heap.buffer.len());
    let mut free_ptr = to_space.as_ptr();

    // Phase 1: scan roots from the call stack.
    //
    // For each frame we scan two sets of pointer offsets:
    //   1. `frame_layout.heap_ptr_offsets` — always applies.
    //   2. The matching `safe_point_layouts` entry for the frame's
    //      current PC, if any — provides additional pointer offsets
    //      that are only valid at that specific safe point.
    //
    // PC tracking: the topmost frame uses `pc_top`. For caller frames,
    // the saved PC is read from the callee's frame metadata (the return
    // address stored by `CallFunc`).
    let mut fp = frame_ptr;
    let mut func_ptr = current_func;
    let mut pc = pc_top;

    loop {
        // SAFETY: func_ptr is a valid, non-null pointer — set from
        // `self.functions[]` or from `CallLocalFunc`, and saved/restored
        // via frame metadata. Arena pointers are valid for the lifetime
        // of the executable. `fp.sub` retrieves saved metadata written
        // by the call protocol.
        let func = unsafe { func_ptr.as_ref() };
        unsafe {
            // Scan base pointer offsets (always active).
            let base_offsets = func.frame_layout.heap_ptr_offsets.as_ref_unchecked();
            gc_scan_frame_roots(heap, fp, base_offsets, &mut free_ptr);

            // Scan safe-point-specific pointer offsets, if any.
            if let Some(sp_layout) = func.safe_point_layout_at(pc) {
                let sp_offsets = sp_layout.heap_ptr_offsets.as_ref_unchecked();
                gc_scan_frame_roots(heap, fp, sp_offsets, &mut free_ptr);
            }

            let meta = fp.sub(FRAME_METADATA_SIZE);
            let saved_func_ptr = read_ptr(meta, META_SAVED_FUNC_PTR_OFFSET) as *const Function;
            if saved_func_ptr.is_null() {
                break;
            }
            pc = read_u64(meta, META_SAVED_PC_OFFSET) as usize;
            fp = read_ptr(meta, META_SAVED_FP_OFFSET);
            // SAFETY: saved_func_ptr is non-null — we checked for the
            // null sentinel above and would have broken out of the loop.
            func_ptr = NonNull::new_unchecked(saved_func_ptr as *mut Function);
        }
    }

    // Phase 1b: scan the auxiliary pinned-roots set.
    //
    // SAFETY: `update_in_place` requires that its callback neither pin
    // new objects nor drop `PinGuard`s tied to this `PinnedRoots`. Our
    // callback only calls `gc_copy_object` and `is_heap_ptr`, which don't
    // touch the pinned-roots set.
    unsafe {
        pinned_roots.update_in_place(|old_ptr| {
            if is_heap_ptr(heap, old_ptr) {
                gc_copy_object(old_ptr, &mut free_ptr)
            } else {
                old_ptr
            }
        });
    }

    // Phase 2: Cheney-style breadth-first scan of copied objects.
    let mut scan_ptr = to_space.as_ptr();
    while (scan_ptr as usize) < (free_ptr as usize) {
        // SAFETY: scan_ptr advances through to-space by each object's
        // aligned size. Object headers were copied verbatim by
        // gc_copy_object, so descriptor_id and size are valid as long
        // as the object-header-integrity invariant holds (see above).
        unsafe {
            let descriptor_id = read_u32(scan_ptr, HEADER_DESCRIPTOR_OFFSET);
            let obj_size = read_u32(scan_ptr, HEADER_SIZE_OFFSET) as usize;

            if obj_size == 0 || obj_size != align8(obj_size) {
                bail!(
                    "GC scan: invalid object size {} (expected non-zero, 8-byte aligned)",
                    obj_size
                );
            }

            if descriptor_id == FORWARDED_MARKER {
                bail!("GC found forwarding marker in to-space (invariant violation)");
            }
            gc_scan_object(descriptors, heap, scan_ptr, descriptor_id, &mut free_ptr);

            scan_ptr = scan_ptr.add(obj_size);
        }
    }

    // Phase 3: swap — drop old heap, adopt new one.
    heap.buffer = to_space;
    heap.bump_ptr = free_ptr;
    Ok(())
}

/// Returns true if `ptr` falls within the (from-space) heap buffer.
fn is_heap_ptr(heap: &Heap, ptr: *const u8) -> bool {
    let start = heap.buffer.as_ptr() as usize;
    let end = start + heap.buffer.len();
    let p = ptr as usize;
    p >= start && p < end
}

/// Scan a set of pointer offsets in a frame, copying any heap objects
/// they reference into to-space and updating the frame slots.
///
/// # Safety
///
/// - `fp` must point to a valid frame.
/// - Each entry in `offsets` must be a valid 8-byte-aligned offset within
///   the frame's extended size.
/// - `free_ptr` must point into to-space with sufficient room.
unsafe fn gc_scan_frame_roots(
    heap: &Heap,
    fp: *mut u8,
    offsets: &[FrameOffset],
    free_ptr: &mut *mut u8,
) {
    unsafe {
        for &offset in offsets {
            let old_ptr = read_ptr(fp, offset);
            if !old_ptr.is_null() && is_heap_ptr(heap, old_ptr) {
                let new_ptr = gc_copy_object(old_ptr, free_ptr);
                write_ptr(fp, offset, new_ptr);
            }
        }
    }
}

/// Copy a single object from the old heap into to-space (at `*free_ptr`),
/// writing a forwarding pointer in the old location. If the object is
/// already forwarded, just return the forwarding address.
///
/// # Safety
///
/// - `old_ptr` must point to the header of a live object in from-space.
///   Its `descriptor_id` and `size` header fields must be valid (see
///   object-header-integrity invariant on [`gc_collect`]).
/// - `free_ptr` must point into to-space with at least `obj_size` bytes
///   remaining (already 8-byte aligned).
/// - The from-space object must not have been partially overwritten
///   except by a prior call to this function (which installs a
///   forwarding marker).
fn gc_copy_object(old_ptr: *mut u8, free_ptr: &mut *mut u8) -> *mut u8 {
    unsafe {
        let descriptor_id = read_u32(old_ptr, HEADER_DESCRIPTOR_OFFSET);

        if descriptor_id == FORWARDED_MARKER {
            return read_ptr(old_ptr, OBJECT_HEADER_SIZE);
        }

        let obj_size = read_u32(old_ptr, HEADER_SIZE_OFFSET) as usize;
        debug_assert!(
            obj_size > 0 && obj_size == align8(obj_size),
            "gc_copy_object: invalid object size {}",
            obj_size
        );
        let new_ptr = *free_ptr;

        std::ptr::copy_nonoverlapping(old_ptr, new_ptr, obj_size);
        *free_ptr = new_ptr.add(obj_size);

        write_u32(old_ptr, HEADER_DESCRIPTOR_OFFSET, FORWARDED_MARKER);
        write_ptr(old_ptr, OBJECT_HEADER_SIZE, new_ptr);

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
                    let old_ptr = read_ptr(obj_ptr, STRUCT_DATA_OFFSET + off as usize);
                    if !old_ptr.is_null() && is_heap_ptr(heap, old_ptr) {
                        let new_ptr = gc_copy_object(old_ptr, free_ptr);
                        write_ptr(obj_ptr, STRUCT_DATA_OFFSET + off as usize, new_ptr);
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
            // fixed payload offset.
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
