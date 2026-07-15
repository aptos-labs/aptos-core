// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Bump-allocated heap with a copying garbage collector (Cheney's algorithm).
//!
//! Exposed as free functions taking explicit parameters (heap, descriptors,
//! root pool, root-walk state). This keeps the borrow surface narrow —
//! callers in the interpreter can hold auxiliary borrows (e.g. a handle into
//! the [`RootPool`]) while still invoking allocation paths that mutate only the
//! heap.

mod deep_copy;

use crate::{
    error::{RuntimeError, RuntimeInvariantViolation, RuntimeResult},
    global_storage::ResourceReadWriteSet,
    invariant_violation,
    memory::{
        read_descriptor, read_forwarding, read_obj_size, read_ptr, read_u64, write_descriptor,
        write_forwarding, write_object_header, write_ptr, write_u64, MemoryRegion,
    },
    types::{
        DEFAULT_HEAP_SIZE, FORWARDED_MARKER, META_SAVED_FP_OFFSET, META_SAVED_FUNC_PTR_OFFSET,
        VEC_DATA_OFFSET, VEC_LENGTH_OFFSET,
    },
};
use mono_move_core::{
    align_max, checked_align_max,
    native::{NativeABI, NativeExtensions},
    types::InternedType,
    DescriptorId, DescriptorProvider, FrameOffset, Function, LayoutProvider, ObjectDescriptorInner,
    RootPool, CAPTURED_DATA_VALUES_OFFSET, CLOSURE_CAPTURED_DATA_PTR_OFFSET, CLOSURE_DATA_SIZE,
    ENUM_DATA_OFFSET, ENUM_TAG_OFFSET, FRAME_METADATA_SIZE, OBJECT_HEADER_SIZE,
};
use std::ptr::NonNull;

// ---------------------------------------------------------------------------
// Macros
// ---------------------------------------------------------------------------

/// Call-site sugar for the allocation and GC free functions in this module.
///
/// Each macro forwards to an eponymous free function, unpacking the fields of an
/// `InterpreterContext`.
///
/// ```ignore
/// let ptr = heap::macros::alloc_obj!(self, fp, pc, func, desc_id)?;
/// ```
///
/// # Why macros and not methods
///
/// Rust lacks partial-borrow syntax: a method on `InterpreterContext` that
/// mutates a field would borrow `self` as `&mut`, conflicting with any
/// outstanding borrow of an unrelated field.
///
/// The macros live in a submodule so their `macro_rules!` names don't
/// collide with the free functions in the value namespace of this module
/// when re-exported.
pub(crate) mod macros {
    /// Forwards to [`super::alloc_obj`]. Arguments: (`$ctx`, `$fp`,
    /// `$descriptor_id`).
    macro_rules! alloc_obj {
        ($ctx:ident, $fp:expr, $pc:expr, $func:expr, $descriptor_id:expr $(,)?) => {
            $crate::heap::alloc_obj(
                &mut $ctx.heap,
                $ctx.loader.guard(),
                &mut $ctx.read_write_set,
                &$ctx.root_pool,
                &$ctx.extensions,
                $fp,
                $crate::heap::TopFrame::Function {
                    func: $func,
                    pc: $pc,
                },
                $descriptor_id,
            )
        };
    }
    pub(crate) use alloc_obj;

    /// Forwards to [`super::alloc_vec`]. Arguments: (`$ctx`, `$fp`,
    /// `$descriptor_id`, `$elem_size`, `$capacity_in_elems`).
    macro_rules! alloc_vec {
        (
            $ctx:ident,
            $fp:expr,
            $pc:expr,
            $func:expr,
            $descriptor_id:expr,
            $elem_size:expr,
            $capacity_in_elems:expr $(,)?
        ) => {
            $crate::heap::alloc_vec(
                &mut $ctx.heap,
                $ctx.loader.guard(),
                &mut $ctx.read_write_set,
                &$ctx.root_pool,
                &$ctx.extensions,
                $fp,
                $crate::heap::TopFrame::Function {
                    func: $func,
                    pc: $pc,
                },
                $descriptor_id,
                $elem_size,
                $capacity_in_elems,
            )
        };
    }
    pub(crate) use alloc_vec;

    /// Forwards to [`super::alloc_captured_data`]. Arguments: (`$ctx`, `$fp`,
    /// `$values_size`, `$descriptor_id`).
    macro_rules! alloc_captured_data {
        (
            $ctx:ident,
            $fp:expr,
            $pc:expr,
            $func:expr,
            $values_size:expr,
            $descriptor_id:expr $(,)?
        ) => {
            $crate::heap::alloc_captured_data(
                &mut $ctx.heap,
                $ctx.loader.guard(),
                &mut $ctx.read_write_set,
                &$ctx.root_pool,
                &$ctx.extensions,
                $fp,
                $crate::heap::TopFrame::Function {
                    func: $func,
                    pc: $pc,
                },
                $values_size,
                $descriptor_id,
            )
        };
    }
    pub(crate) use alloc_captured_data;

    /// Forwards to [`super::grow_vec_ref`]. Arguments: (`$ctx`, `$fp`,
    /// `$vec_ref_offset`, `$elem_size`, `$required_cap_in_elems`).
    macro_rules! grow_vec_ref {
        (
            $ctx:ident,
            $fp:expr,
            $pc:expr,
            $func:expr,
            $vec_ref_offset:expr,
            $elem_size:expr,
            $required_cap_in_elems:expr $(,)?
        ) => {
            $crate::heap::grow_vec_ref(
                &mut $ctx.heap,
                $ctx.loader.guard(),
                &mut $ctx.read_write_set,
                &$ctx.root_pool,
                &$ctx.extensions,
                $crate::heap::TopFrame::Function {
                    func: $func,
                    pc: $pc,
                },
                $fp,
                $vec_ref_offset,
                $elem_size,
                $required_cap_in_elems,
            )
        };
    }
    pub(crate) use grow_vec_ref;

    /// Forwards to [`super::gc_collect`]. Arguments: (`$ctx`,).
    macro_rules! gc_collect {
        ($ctx:ident, $fp:expr, $pc:expr, $func:expr $(,)?) => {
            $crate::heap::gc_collect(
                &mut $ctx.heap,
                $ctx.loader.guard(),
                &mut $ctx.read_write_set,
                &$ctx.root_pool,
                &$ctx.extensions,
                $fp,
                $crate::heap::TopFrame::Function {
                    func: $func,
                    pc: $pc,
                },
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

    /// Rewinds the bump pointer to the start of the buffer, discarding all allocations. The buffer
    /// is retained (no reallocation), so it's the cheap way to reuse one heap across runs.
    ///
    /// The caller must ensure no live references into the heap remain (reset the interpreter's frame
    /// and root set first, via [`InterpreterContext::reset`](crate::InterpreterContext::reset)).
    pub fn reset(&mut self) {
        self.bump_ptr = self.buffer.as_ptr();
        self.gc_count = 0;
    }

    /// Bump-allocates one object of `total_size` bytes (header + payload), stamps the header with
    /// `descriptor_id`, and returns the data pointer (payload start; the header occupies the
    /// preceding [`OBJECT_HEADER_SIZE`](mono_move_core::OBJECT_HEADER_SIZE) bytes). Returns `None` if
    /// the heap is full. Does not run GC.
    pub fn alloc_object(
        &mut self,
        total_size: usize,
        descriptor_id: DescriptorId,
    ) -> Option<std::ptr::NonNull<u8>> {
        let ptr = heap_alloc(self, total_size, descriptor_id).ok()?;
        std::ptr::NonNull::new(ptr)
    }
}

/// Outcome of a bump-allocation attempt.
#[derive(Debug)]
pub(crate) enum AllocationError {
    /// Heap is full and the requested size did not fit. GC can be triggered to
    /// free up space and continue. If after GC there is still no space for the
    /// allocation, converted to [`RuntimeError::OutOfHeapMemory`].
    OutOfHeapMemory { requested: usize },
    /// Runtime error, not recoverable.
    RuntimeError(RuntimeError),
}

pub(crate) type AllocationResult<T> = Result<T, AllocationError>;

impl From<RuntimeError> for AllocationError {
    fn from(err: RuntimeError) -> Self {
        Self::RuntimeError(err)
    }
}

impl AllocationError {
    /// Converts allocation error into non-recoverable [`RuntimeError`].
    pub(crate) fn into_runtime_error(self) -> RuntimeError {
        match self {
            AllocationError::OutOfHeapMemory { requested } => {
                RuntimeError::OutOfHeapMemory { requested }
            },
            AllocationError::RuntimeError(err) => err,
        }
    }
}

/// Runs allocation function; on [`AllocationError::OutOfHeapMemory`] runs GC
/// once and retries again. If allocation fails on retry, returns a runtime
/// error.
pub(crate) fn alloc_or_gc<P: DescriptorProvider + ?Sized>(
    heap: &mut Heap,
    provider: &P,
    rws: &mut ResourceReadWriteSet,
    extra_roots: &RootPool,
    extensions: &NativeExtensions,
    fp: *mut u8,
    top_frame: TopFrame<'_>,
    try_alloc: impl Fn(&mut Heap) -> AllocationResult<*mut u8>,
) -> RuntimeResult<*mut u8> {
    match try_alloc(heap) {
        Ok(ptr) => Ok(ptr),
        Err(AllocationError::OutOfHeapMemory { .. }) => {
            gc_collect(heap, provider, rws, extra_roots, extensions, fp, top_frame)?;
            try_alloc(heap).map_err(AllocationError::into_runtime_error)
        },
        Err(e) => Err(e.into_runtime_error()),
    }
}

/// Deep-copies the value tree at `root`.
/// Automatically runs GC and retries when out of memory.
///
/// # Safety
///
/// `root` must point to the data region of a live object whose header is at
/// `root - OBJECT_HEADER_SIZE`.
pub(crate) unsafe fn deep_copy_or_gc<P: DescriptorProvider + ?Sized>(
    heap: &mut Heap,
    provider: &P,
    rws: &mut ResourceReadWriteSet,
    extra_roots: &RootPool,
    extensions: &NativeExtensions,
    fp: *mut u8,
    top_frame: TopFrame<'_>,
    root: NonNull<u8>,
) -> RuntimeResult<NonNull<u8>> {
    // SAFETY: `root` is a live object (this function's contract).
    let root_guard = unsafe { extra_roots.root_object(root.as_ptr()) };
    // SAFETY: the rooted object is live, so its (possibly relocated) pointer is
    // non-null.
    match unsafe { heap.try_deep_copy(provider, NonNull::new_unchecked(root_guard.ptr())) } {
        Ok(ptr) => Ok(ptr),
        Err(AllocationError::RuntimeError(err)) => Err(err),
        Err(AllocationError::OutOfHeapMemory { .. }) => {
            gc_collect(heap, provider, rws, extra_roots, extensions, fp, top_frame)?;
            // SAFETY: the handle kept the source live across GC; re-read its
            // relocated, still-non-null pointer.
            unsafe {
                heap.try_deep_copy(provider, NonNull::new_unchecked(root_guard.ptr()))
                    .map_err(AllocationError::into_runtime_error)
            }
        },
    }
}

/// Deserializes `bytes` into `dst` as a value of type `ty`. If the first attempt
/// runs out of heap, collects garbage once (with the given roots) and retries.
/// The deserialize counterpart to [`alloc_or_gc`] / [`deep_copy_or_gc`].
///
/// # Safety
///
/// `dst` must be writable for `ty`'s layout size, and `bytes` must not alias the
/// heap, since a GC may relocate heap objects.
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe fn deserialize_or_gc<
    L: LayoutProvider + ?Sized,
    P: DescriptorProvider + ?Sized,
>(
    layouts: &L,
    heap: &mut Heap,
    ty: InternedType,
    bytes: &[u8],
    dst: *mut u8,
    provider: &P,
    rws: &mut ResourceReadWriteSet,
    extra_roots: &RootPool,
    extensions: &NativeExtensions,
    frame_ptr: *mut u8,
    top_frame: TopFrame<'_>,
) -> RuntimeResult<()> {
    // SAFETY: forwarded from this function's contract.
    match unsafe { crate::value_utils::deserialize(layouts, heap, ty, bytes, dst) } {
        Ok(()) => Ok(()),
        Err(AllocationError::RuntimeError(err)) => Err(err),
        Err(AllocationError::OutOfHeapMemory { .. }) => {
            gc_collect(
                heap,
                provider,
                rws,
                extra_roots,
                extensions,
                frame_ptr,
                top_frame,
            )?;
            // SAFETY: as above.
            unsafe { crate::value_utils::deserialize(layouts, heap, ty, bytes, dst) }
                .map_err(AllocationError::into_runtime_error)
        },
    }
}

/// Root-walking interface for the GC. Passed to each root source that iterates
/// its allocations and calls [`Self::relocate`] per pointer, writing the
/// returned value back into the slot.
pub(crate) struct RootScanner<'a> {
    /// From-space heap.
    heap: &'a Heap,
    /// Bump cursor in to-space. Advances during relocation.
    free_ptr: *mut u8,
}

impl<'a> RootScanner<'a> {
    /// Test-only constructor. Production code only sees a
    /// `RootScanner` handed in by [`gc_collect`].
    #[cfg(test)]
    pub(crate) fn for_test(heap: &'a Heap, free_ptr: *mut u8) -> RootScanner<'a> {
        RootScanner { heap, free_ptr }
    }

    /// Current to-space bump cursor.
    pub(crate) fn cursor(&self) -> usize {
        self.free_ptr as usize
    }

    /// Relocates a possibly-from-space pointer to its to-space address.
    /// address. Returns [`None`] when pointer is null or does not belong to
    /// the heap. Otherwise, returns the copy.
    ///
    /// Callers must uphold the object-header-integrity invariant
    /// documented on [`gc_collect`]: if `ptr` is in-heap, its
    /// header at `ptr - OBJECT_HEADER_SIZE` must be valid.
    pub(crate) fn relocate(&mut self, ptr: *mut u8) -> Option<*mut u8> {
        if ptr.is_null() || !is_heap_ptr(self.heap, ptr) {
            return None;
        }
        Some(gc_copy_object(ptr, &mut self.free_ptr))
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
/// Returns [`AllocationError::OutOfHeapMemory`] when the heap is full
/// so the caller can trigger GC and retry.
pub(crate) fn heap_alloc(
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
        // TODO(perf): this O(size) memset is redundant for `EnumNew` /
        // `PackVariant`, which immediately overwrite the tag and every field of
        // the active variant. A pack-aware non-zeroing path could skip it (a
        // material win for large/wide-variant enums). It is not a drop-in
        // change: `gc_copy_object` / `deep_copy` copy the full object image
        // including dead-variant tail and inter-field padding, which is
        // deterministically zero today; leaving it uninitialized makes that
        // image carry stale heap bytes. Prefer zeroing only the
        // tail/padding the active variant does not write (still skipping the
        // large active body), or audit that no byte-image consumer
        // (state commit / hashing) depends on those bytes first.
        std::ptr::write_bytes(header_start, 0, aligned_size);
        let obj_ptr = header_start.add(OBJECT_HEADER_SIZE);
        write_object_header(obj_ptr, descriptor_id, aligned_size as u32);
        Ok(obj_ptr)
    }
}

/// Reserve `total_size` bytes (object header + payload) on the heap, running a
/// GC and retrying once on out-of-memory. Shared tail of the `alloc_*` entry
/// points below, which differ only in how `total_size` is computed.
fn alloc_sized<P: DescriptorProvider + ?Sized>(
    heap: &mut Heap,
    provider: &P,
    rws: &mut ResourceReadWriteSet,
    extra_roots: &RootPool,
    extensions: &NativeExtensions,
    fp: *mut u8,
    top_frame: TopFrame<'_>,
    total_size: usize,
    descriptor_id: DescriptorId,
) -> RuntimeResult<*mut u8> {
    alloc_or_gc(
        heap,
        provider,
        rws,
        extra_roots,
        extensions,
        fp,
        top_frame,
        |h| heap_alloc(h, total_size, descriptor_id),
    )
}

/// Allocate a new vector object on the heap.
///
/// `capacity_in_elems` is the number of elements the vector can hold
/// before needing to grow — *not* a byte size. Total bytes allocated
/// are roughly `capacity_in_elems * elem_size + OBJECT_HEADER_SIZE +
/// VEC_DATA_OFFSET`, rounded up to `MAX_ALIGN`.
pub(crate) fn alloc_vec<P: DescriptorProvider + ?Sized>(
    heap: &mut Heap,
    provider: &P,
    rws: &mut ResourceReadWriteSet,
    extra_roots: &RootPool,
    extensions: &NativeExtensions,
    fp: *mut u8,
    top_frame: TopFrame<'_>,
    descriptor_id: DescriptorId,
    elem_size: u32,
    capacity_in_elems: u64,
) -> RuntimeResult<*mut u8> {
    let total_size = (capacity_in_elems as usize)
        .checked_mul(elem_size as usize)
        .and_then(|v| v.checked_add(OBJECT_HEADER_SIZE + VEC_DATA_OFFSET))
        .ok_or(RuntimeError::VecAllocSizeOverflow)?;
    // `length` defaults to 0 via heap_alloc's zero-init.
    alloc_sized(
        heap,
        provider,
        rws,
        extra_roots,
        extensions,
        fp,
        top_frame,
        total_size,
        descriptor_id,
    )
}

/// Allocate a new zeroed heap object (struct or enum). Size comes from the
/// descriptor at `descriptor_id`.
pub(crate) fn alloc_obj<P: DescriptorProvider + ?Sized>(
    heap: &mut Heap,
    provider: &P,
    rws: &mut ResourceReadWriteSet,
    extra_roots: &RootPool,
    extensions: &NativeExtensions,
    fp: *mut u8,
    top_frame: TopFrame<'_>,
    descriptor_id: DescriptorId,
) -> RuntimeResult<*mut u8> {
    let desc = match provider.descriptor(descriptor_id) {
        Some(desc) => desc,
        None => invariant_violation!(DescriptorNotFound {
            descriptor_id: descriptor_id.as_u32(),
        }),
    };
    let payload_size = match desc.inner() {
        ObjectDescriptorInner::Struct { size, .. } => *size as usize,
        ObjectDescriptorInner::Enum { size, .. } => *size as usize,
        ObjectDescriptorInner::Closure => CLOSURE_DATA_SIZE,
        // These descriptors carry no intrinsic size, so their objects are
        // allocated through explicit-size entry points, not this path.
        ObjectDescriptorInner::Trivial
        | ObjectDescriptorInner::Vector { .. }
        | ObjectDescriptorInner::CapturedData { .. } => {
            invariant_violation!(NonAllocatableDescriptor {
                descriptor_id: descriptor_id.as_u32(),
            });
        },
    };
    let total_size = OBJECT_HEADER_SIZE + payload_size;
    alloc_sized(
        heap,
        provider,
        rws,
        extra_roots,
        extensions,
        fp,
        top_frame,
        total_size,
        descriptor_id,
    )
}

/// Allocate a closure's captured-data object. The values-region `values_size`
/// is a parameter, not read from the descriptor, so layouts sharing a trace
/// shape share one `descriptor_id`: `Trivial` when pointer-free, else a
/// `CapturedData` descriptor.
pub(crate) fn alloc_captured_data<P: DescriptorProvider + ?Sized>(
    heap: &mut Heap,
    provider: &P,
    rws: &mut ResourceReadWriteSet,
    extra_roots: &RootPool,
    extensions: &NativeExtensions,
    fp: *mut u8,
    top_frame: TopFrame<'_>,
    values_size: u32,
    descriptor_id: DescriptorId,
) -> RuntimeResult<*mut u8> {
    let total_size = OBJECT_HEADER_SIZE + CAPTURED_DATA_VALUES_OFFSET + values_size as usize;
    alloc_sized(
        heap,
        provider,
        rws,
        extra_roots,
        extensions,
        fp,
        top_frame,
        total_size,
        descriptor_id,
    )
}

/// Grow a vector to at least `required_cap_in_elems` elements, accessed
/// through a fat pointer reference at `fp + vec_ref_offset`. The vector
/// pointer is written back through the reference; returns the new object
/// pointer.
///
/// # Safety
///
/// `fp` must point to a valid frame. `vec_ref_offset` must be the byte
/// offset of a 16-byte fat pointer `(base, offset)` whose target holds
/// the current vector heap pointer. `alloc_vec` may trigger GC which
/// relocates objects; the fat pointer's base in `heap_ptr_offsets` and the
/// vector pointer in the struct's `pointer_offsets` are updated by the GC.
/// We re-read through the fat pointer after allocation.
pub(crate) fn grow_vec_ref<P: DescriptorProvider + ?Sized>(
    heap: &mut Heap,
    provider: &P,
    rws: &mut ResourceReadWriteSet,
    extra_roots: &RootPool,
    extensions: &NativeExtensions,
    top_frame: TopFrame<'_>,
    fp: *mut u8,
    vec_ref_offset: usize,
    elem_size: u32,
    required_cap_in_elems: u64,
) -> RuntimeResult<*mut u8> {
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

        // alloc_vec may trigger GC. Re-read through the fat pointer afterward.
        let new_ptr = alloc_vec(
            heap,
            provider,
            rws,
            extra_roots,
            extensions,
            fp,
            top_frame,
            descriptor_id,
            elem_size,
            new_cap_in_elems,
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

/// Identifies the top stack frame for GC root scanning.
//
// TODO(cleanup): revisit whether this enum is the cleanest representation.
#[derive(Clone, Copy)]
pub(crate) enum TopFrame<'a> {
    /// A regular Move function frame; `pc` selects the safe-point supplement.
    Function { func: NonNull<Function>, pc: usize },
    /// A native function frame; its arg slots hold the roots.
    Native(&'a NativeABI),
}

/// Run Cheney's copying GC. Walks the call stack to find roots via
/// per-function pointer slot lists, scans the [`RootPool`], then does a
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
pub(crate) fn gc_collect<P: DescriptorProvider + ?Sized>(
    heap: &mut Heap,
    provider: &P,
    rws: &mut ResourceReadWriteSet,
    extra_roots: &RootPool,
    extensions: &NativeExtensions,
    frame_ptr: *mut u8,
    top_frame: TopFrame<'_>,
) -> Result<(), RuntimeInvariantViolation> {
    heap.gc_count += 1;

    let to_space = MemoryRegion::new(heap.buffer.len());
    // `free_ptr` is a raw bump cursor — it points at the start of the
    // next *header* reservation, advancing by each object's total size.
    // Treating it as a raw cursor (rather than as an "object pointer"
    // biased by OBJECT_HEADER_SIZE) keeps its maximum value at
    // `to_space.end()`, which is the one-past-end address that pointer
    // arithmetic permits. A `+ H` bias would let it overshoot when all
    // of from-space survives, producing UB on `.add` / `.sub`.
    let mut scanner = RootScanner {
        heap,
        free_ptr: to_space.as_ptr(),
    };

    // Phase 1a: scan roots from the call stack. The top frame is either a
    // regular function (scanned via `frame_layout` plus the `safe_point_layouts`
    // entry for its current pc, per `SafePointEntry`'s top-frame-only contract)
    // or a native (scanned via the arg pointer slots in its ABI). Caller-below
    // frames are always regular functions and use `frame_layout` alone.
    match top_frame {
        TopFrame::Function { func, pc } => {
            // SAFETY: func is valid (caller's invariant).
            let top_func = unsafe { func.as_ref() };
            unsafe {
                gc_scan_frame_roots(
                    &mut scanner,
                    frame_ptr,
                    &top_func.frame_layout.heap_ptr_offsets,
                );
                if let Some(sp_layout) = top_func.safe_point_layout_at(pc) {
                    gc_scan_frame_roots(&mut scanner, frame_ptr, &sp_layout.heap_ptr_offsets);
                }
            }
        },
        TopFrame::Native(abi) => {
            // SAFETY: a native's arg slots hold the references passed to it.
            unsafe {
                gc_scan_frame_roots(&mut scanner, frame_ptr, abi.heap_ptr_offsets());
            }
        },
    }

    // Walk caller frames with frame_layout only.
    let mut fp = frame_ptr;
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
            gc_scan_frame_roots(&mut scanner, fp, &caller_func.frame_layout.heap_ptr_offsets);
        }
    }

    // Phase 1b: scan the extra roots (the interpreter's pins or a native's
    // rooted handles).
    // SAFETY: GC is stop-the-world, so no handle read overlaps; the closure only
    // relocates via the scanner and never re-enters the pool.
    unsafe { extra_roots.relocate_each(|base| scanner.relocate(base)) };

    // Phase 1c: resource read-write set. Working-map and journal writes
    // point into the local heap and must be relocated alongside the call
    // stack and pinned roots.
    rws.scan(&mut scanner);

    // Phase 1d: native extension roots.
    //
    // TODO(correctness): a native holding an extension borrow across an
    // allocation makes this a hard error; revisit how to guarantee exclusive
    // access here (e.g. relocating only the disjoint root set).
    unsafe {
        extensions
            .relocate_all_roots(&mut |base| scanner.relocate(base))
            .map_err(|_| RuntimeInvariantViolation::ExtensionBorrowedDuringGC)?;
    }

    // Phase 2: Cheney-style breadth-first scan of copied objects.
    // `scan_ptr` is a raw cursor — header start of the next object to
    // scan. Object pointers (data starts) are `scan_ptr + H`.
    let mut scan_ptr = to_space.as_ptr();
    while (scan_ptr as usize) < scanner.cursor() {
        // SAFETY: scan_ptr advances through to-space by each object's
        // aligned size. Object headers were copied verbatim by
        // gc_copy_object, so descriptor_id and size are valid as long
        // as the object-header-integrity invariant holds (see above).
        unsafe {
            let obj_ptr = scan_ptr.add(OBJECT_HEADER_SIZE);
            let descriptor_id = read_descriptor(obj_ptr);
            let obj_size = read_obj_size(obj_ptr) as usize;

            if obj_size == 0 || obj_size != align_max(obj_size) {
                return Err(RuntimeInvariantViolation::GcInvalidObjectSize { size: obj_size });
            }

            if descriptor_id == FORWARDED_MARKER {
                return Err(RuntimeInvariantViolation::GcForwardingMarkerInToSpace);
            }
            gc_scan_object(provider, &mut scanner, obj_ptr, DescriptorId(descriptor_id))?;

            scan_ptr = scan_ptr.add(obj_size);
        }
    }

    // Phase 3: swap — drop old heap, adopt new one. The bump cursor
    // semantics match `free_ptr` directly (both are raw header-start
    // cursors).
    let RootScanner { free_ptr, .. } = scanner;
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
pub(crate) fn is_heap_ptr(heap: &Heap, ptr: *const u8) -> bool {
    let start = heap.buffer.as_ptr() as usize;
    let end = start + heap.buffer.len();
    let p = ptr as usize;
    p >= start + OBJECT_HEADER_SIZE && p <= end
}

/// Scan a set of pointer offsets in a frame, copying any heap objects
/// they reference into to-space (via `scanner`) and updating the frame
/// slots.
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
            if let Some(new_ptr) = scanner.relocate(old_ptr) {
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
fn gc_scan_object<P: DescriptorProvider + ?Sized>(
    provider: &P,
    scanner: &mut RootScanner<'_>,
    obj_ptr: *mut u8,
    descriptor_id: DescriptorId,
) -> Result<(), RuntimeInvariantViolation> {
    let desc = match provider.descriptor(descriptor_id) {
        Some(d) => d,
        None => {
            return Err(RuntimeInvariantViolation::DescriptorNotFound {
                descriptor_id: descriptor_id.as_u32(),
            })
        },
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
                return Ok(());
            }
            unsafe {
                let length = read_u64(obj_ptr, VEC_LENGTH_OFFSET) as usize;
                let data_start = obj_ptr.add(VEC_DATA_OFFSET);

                for i in 0..length {
                    let elem_base = data_start.add(i * (*elem_size as usize));
                    for &ptr_off in elem_pointer_offsets {
                        let old_ptr = read_ptr(elem_base, ptr_off as usize);
                        if let Some(new_ptr) = scanner.relocate(old_ptr) {
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
                return Ok(());
            }
            unsafe {
                for &off in pointer_offsets {
                    let old_ptr = read_ptr(obj_ptr, off as usize);
                    if let Some(new_ptr) = scanner.relocate(old_ptr) {
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
                // A tag past the variant count means a corrupt object; surface
                // it as an invariant violation.
                return Err(RuntimeInvariantViolation::EnumTagOutOfRange {
                    tag: tag as u64,
                    variant_count: variant_pointer_offsets.len(),
                });
            }
            let pointer_offsets = &variant_pointer_offsets[tag];
            if pointer_offsets.is_empty() {
                return Ok(());
            }
            for &off in pointer_offsets {
                let abs_off = ENUM_DATA_OFFSET + off as usize;
                let old_ptr = read_ptr(obj_ptr, abs_off);
                if let Some(new_ptr) = scanner.relocate(old_ptr) {
                    write_ptr(obj_ptr, abs_off, new_ptr);
                }
            }
        },
        ObjectDescriptorInner::Closure => unsafe {
            // The closure's only heap pointer is `captured_data_ptr` at a
            // fixed data-region offset.
            let off = CLOSURE_CAPTURED_DATA_PTR_OFFSET;
            let old_ptr = read_ptr(obj_ptr, off);
            if let Some(new_ptr) = scanner.relocate(old_ptr) {
                write_ptr(obj_ptr, off, new_ptr);
            }
        },
        ObjectDescriptorInner::CapturedData { pointer_offsets } => {
            if pointer_offsets.is_empty() {
                return Ok(());
            }
            unsafe {
                for &off in pointer_offsets {
                    let abs_off = CAPTURED_DATA_VALUES_OFFSET + off as usize;
                    // Shared across objects of different `values_size`
                    // (offset-shape dedup), so `off` isn't bounded by this
                    // object's size; the layout guarantees `values_size >=
                    // off + 8`. Assert it for this object.
                    debug_assert!(
                        abs_off + 8 <= read_obj_size(obj_ptr) as usize - OBJECT_HEADER_SIZE,
                        "captured-data pointer offset {off} exceeds object payload"
                    );
                    let old_ptr = read_ptr(obj_ptr, abs_off);
                    if let Some(new_ptr) = scanner.relocate(old_ptr) {
                        write_ptr(obj_ptr, abs_off, new_ptr);
                    }
                }
            }
        },
    }
    Ok(())
}
