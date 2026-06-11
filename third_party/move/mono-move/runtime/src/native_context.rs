// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Concrete native-function types used by the production VM.
//!
//! These are internal to the VM; native functions depend only on the
//! [`NativeContext`] trait, never on these types directly.

use crate::{
    error::RuntimeError,
    global_storage::ResourceReadWriteSet,
    heap::{alloc_vec, is_heap_ptr, Heap, TopFrame},
    memory::{read_descriptor, read_obj_size, read_ptr, read_u64, write_ptr, write_u64},
    types::{VEC_DATA_OFFSET, VEC_LENGTH_OFFSET},
};
use mono_move_core::{
    native::{
        NativeABI, NativeContext, NativeContextFamily, NativeFunction, NativeRegistry, Opaque, Ref,
        RootPool, VMInternalError, VMValue, Vector,
    },
    types::InternedType,
    DescriptorId, DescriptorProvider, GasMeter, OBJECT_HEADER_SIZE, TRIVIAL_DESCRIPTOR_ID,
};
use std::cell::{Cell, UnsafeCell};

/// Concrete [`NativeContext`] used by the production runtime.
///
/// Constructed inline by the interpreter at the dispatch site (one instance per
/// native call) and exposed to native functions only through the
/// [`NativeContext`] trait.
///
/// # Interior mutability & safety invariants
///
/// Trait methods take `&self`, so the mutable sub-components sit behind interior
/// mutability and these invariants MUST be upheld by the implementation rather than
/// the borrow checker:
///  - Together, all public entry points must ensure that **at most one** mutable
///    borrow of any field is live at any time. If one methods needs to call another,
///    it must ensure all borrows are all disjoint -- if the callee needs
///    to borrow the same field, it must be passed in from the caller. Another way to
///    phrase this is that no reentrancies should be allowed.
///  - Exclusivity against the rest of the interpreter is not a manual burden:
///    access to other VM components are passed in as `&mut T`, allowing us to have
///    exclusive access to those here.
pub struct ProductionNativeContext<'a> {
    /// ABI of the native being invoked, describing the native's frame layout.
    abi: &'a NativeABI,
    /// Type arguments to the native.
    ty_args: &'a [InternedType],
    /// Descriptor provider, used by any GC the native triggers while allocating.
    desc_provider: &'a dyn DescriptorProvider,
    /// Start of the native's slot region within the caller's frame. Args are
    /// read and returns written here, within the ABI-verified bounds.
    frame_ptr: *mut u8,
    /// Gas meter for the current transaction.
    ///
    /// TODO: Expose to native functions.
    #[allow(dead_code)]
    gas: UnsafeCell<&'a mut GasMeter>,
    /// The VM's heap -- used by the natives to allocate new heap objects.
    heap: UnsafeCell<&'a mut Heap>,
    /// The transaction's read write set -- provides global storage access.
    rws: UnsafeCell<&'a mut ResourceReadWriteSet>,
    /// GC roots backing the references and heap objects the native holds.
    pool: RootPool,
    /// Set after the first [`Self::set_return`]; blocks further `arg` /
    /// allocation calls.
    returns_started: Cell<bool>,
}

impl<'a> ProductionNativeContext<'a> {
    // TODO: revisit this lint.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        frame_ptr: *mut u8,
        abi: &'a NativeABI,
        ty_args: &'a [InternedType],
        gas_meter: &'a mut GasMeter,
        desc_provider: &'a dyn DescriptorProvider,
        heap: &'a mut Heap,
        rws: &'a mut ResourceReadWriteSet,
    ) -> Self {
        Self {
            abi,
            ty_args,
            desc_provider,
            frame_ptr,
            gas: UnsafeCell::new(gas_meter),
            heap: UnsafeCell::new(heap),
            rws: UnsafeCell::new(rws),
            pool: RootPool::new(),
            returns_started: Cell::new(false),
        }
    }
}

impl NativeContext for ProductionNativeContext<'_> {
    fn num_args(&self) -> usize {
        self.abi.args().len()
    }

    unsafe fn arg<'a, T: VMValue<'a>>(&'a self, i: usize) -> Result<T, VMInternalError> {
        if self.returns_started.get() {
            return Err(VMInternalError::InvariantViolation(format!(
                "arg({}) called after a return value was written",
                i,
            )));
        }
        let slot = self.abi.args().get(i).copied().ok_or_else(|| {
            VMInternalError::InvariantViolation(format!(
                "arg index {} out of bounds (num_args={})",
                i,
                self.abi.args().len(),
            ))
        })?;
        if T::FRAME_SLOT_SIZE as u32 != slot.size {
            return Err(VMInternalError::InvariantViolation(format!(
                "VMValue size mismatch: ABI says {} bytes for arg {}, T::FRAME_SLOT_SIZE is {}",
                slot.size,
                i,
                T::FRAME_SLOT_SIZE,
            )));
        }
        // SAFETY: the ABI was verified at module load to keep slot.offset+slot.size
        // inside the native's slot region; the interpreter sets `frame_ptr` to the
        // base of that region. Referenced/allocated memory is rooted in `pool`.
        //
        // `T` is responsible for the correctness of its own `write_to_frame` impl.
        Ok(unsafe { T::read_from_frame(&self.pool, self.frame_ptr, slot.offset as usize) })
    }

    fn num_returns(&self) -> usize {
        self.abi.returns().len()
    }

    unsafe fn set_return<'a, T: VMValue<'a>>(
        &'a self,
        i: usize,
        value: T,
    ) -> Result<(), VMInternalError> {
        let slot = self.abi.returns().get(i).copied().ok_or_else(|| {
            VMInternalError::InvariantViolation(format!(
                "return index {} out of bounds (num_returns={})",
                i,
                self.abi.returns().len(),
            ))
        })?;
        if T::FRAME_SLOT_SIZE as u32 != slot.size {
            return Err(VMInternalError::InvariantViolation(format!(
                "VMValue size mismatch: ABI says {} bytes for return {}, T::FRAME_SLOT_SIZE is {}",
                slot.size,
                i,
                T::FRAME_SLOT_SIZE,
            )));
        }
        // SAFETY: same as `arg`, frame_ptr & offset guaranteed by the interpreter and verified
        // by the ABI. `T` is responsible for the correctness of its own `write_to_frame` impl.
        unsafe { value.write_to_frame(self.frame_ptr, slot.offset as usize) };
        self.returns_started.set(true);
        Ok(())
    }

    fn num_ty_args(&self) -> usize {
        self.ty_args.len()
    }

    fn ty_arg(&self, i: usize) -> Result<InternedType, VMInternalError> {
        self.ty_args.get(i).copied().ok_or_else(|| {
            VMInternalError::InvariantViolation(format!(
                "ty_arg index {} out of bounds (num_ty_args={})",
                i,
                self.ty_args.len(),
            ))
        })
    }

    fn new_byte_vector<'a>(&'a self, bytes: &[u8]) -> Result<Vector<'a, u8>, VMInternalError> {
        if self.returns_started.get() {
            return Err(VMInternalError::InvariantViolation(
                "new_byte_vector called after a return value was written".into(),
            ));
        }
        let len = bytes.len() as u64;
        // SAFETY: `heap` and `rws` are distinct fields, so reborrowing both
        // through `&self` at once is sound — at most one `&mut` per field is
        // live (see the type-level aliasing rule).
        let heap = unsafe { &mut **self.heap.get() };
        let rws = unsafe { &mut **self.rws.get() };
        // A heap-aliasing `bytes` would be invalidated by the GC `alloc_vec` may
        // trigger, before the copy below.
        if is_heap_ptr(heap, bytes.as_ptr()) {
            return Err(VMInternalError::InvariantViolation(
                "new_byte_vector: bytes must not alias the VM heap".into(),
            ));
        }
        // A `vector<u8>` has no inner pointers, so it uses the trivial descriptor.
        let ptr = alloc_vec(
            heap,
            self.desc_provider,
            rws,
            &self.pool,
            self.frame_ptr,
            TopFrame::Native(self.abi),
            TRIVIAL_DESCRIPTOR_ID,
            1,
            len,
        )
        // Allocation failures are resource-limit conditions, not VM bugs.
        .map_err(map_alloc_err)?;
        // SAFETY: `ptr` is a fresh vector with room for `len` bytes; no GC runs
        // between here and these writes, so the raw pointer is valid.
        unsafe {
            write_u64(ptr, VEC_LENGTH_OFFSET, len);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(VEC_DATA_OFFSET), bytes.len());
        }
        // Root it so it survives later allocations and is GC-relocated.
        // SAFETY: `ptr` is the data pointer of the freshly allocated vector.
        Ok(Vector::from_handle(unsafe { self.pool.root_object(ptr) }))
    }

    unsafe fn grow_vector<'a>(
        &self,
        target: &Ref<'a, Vector<'a, Opaque>>,
        donor: &Ref<'a, Vector<'a, Opaque>>,
        elem_size: usize,
        required_cap: usize,
    ) -> Result<(), VMInternalError> {
        // A zero-sized element type would divide-by-zero in the capacity math
        // below; the caller is contracted to pass the byte size of a real `T`.
        debug_assert!(elem_size != 0, "grow_vector: elem_size must be non-zero");

        // SAFETY: `heap` and `rws` are distinct fields, so reborrowing both
        // through `&self` at once is sound (see the type-level aliasing rule).
        let heap = unsafe { &mut **self.heap.get() };
        let rws = unsafe { &mut **self.rws.get() };

        // The reference handles are rooted, so `target.ptr()` / `donor.ptr()`
        // stay current across the GC `alloc_vec` may trigger.
        // SAFETY: `target` references a live `vector<T>`; its slot holds the
        // current heap pointer (null for an empty vector).
        let old_ptr = unsafe { read_ptr(target.ptr(), 0usize) };

        if old_ptr.is_null() {
            // `target` is empty: allocate a fresh vector, copying the GC
            // descriptor from the (non-empty) donor of the same element type.
            // SAFETY: `donor` is a non-empty `vector<T>` (caller's contract).
            let src_ptr = unsafe { read_ptr(donor.ptr(), 0usize) };
            let descriptor_id = DescriptorId(unsafe { read_descriptor(src_ptr) });
            let new_ptr = alloc_vec(
                heap,
                self.desc_provider,
                rws,
                &self.pool,
                self.frame_ptr,
                TopFrame::Native(self.abi),
                descriptor_id,
                elem_size as u32,
                (required_cap as u64).max(4),
            )
            .map_err(map_alloc_err)?;
            // `alloc_vec` zero-inits the length; just publish the new pointer
            // through the (post-GC) reference.
            // SAFETY: `target.ptr()` is re-read after the allocation's GC.
            unsafe { write_ptr(target.ptr(), 0usize, new_ptr) };
            return Ok(());
        }

        // `target` is non-empty: grow only if it can't already hold `required_cap`.
        // SAFETY: `old_ptr` is a live vector object.
        let old_len = unsafe { read_u64(old_ptr, VEC_LENGTH_OFFSET) };
        let old_total = unsafe { read_obj_size(old_ptr) } as usize;
        let old_cap = (old_total - OBJECT_HEADER_SIZE - VEC_DATA_OFFSET) / elem_size;
        if required_cap <= old_cap {
            return Ok(());
        }

        let descriptor_id = DescriptorId(unsafe { read_descriptor(old_ptr) });
        let doubled = if old_cap == 0 { 4 } else { old_cap * 2 };
        let new_cap = doubled.max(required_cap);
        let new_ptr = alloc_vec(
            heap,
            self.desc_provider,
            rws,
            &self.pool,
            self.frame_ptr,
            TopFrame::Native(self.abi),
            descriptor_id,
            elem_size as u32,
            new_cap as u64,
        )
        .map_err(map_alloc_err)?;

        // `alloc_vec` may have run a GC; re-read the old pointer through the
        // rooted reference, copy the live elements over, then publish the new
        // pointer through the reference.
        // SAFETY: `target.ptr()` is re-read after the GC; both objects own at
        // least `byte_count` data bytes and are distinct allocations.
        unsafe {
            let old_ptr = read_ptr(target.ptr(), 0usize);
            let byte_count = old_len as usize * elem_size;
            if byte_count > 0 {
                std::ptr::copy_nonoverlapping(
                    old_ptr.add(VEC_DATA_OFFSET),
                    new_ptr.add(VEC_DATA_OFFSET),
                    byte_count,
                );
            }
            write_u64(new_ptr, VEC_LENGTH_OFFSET, old_len);
            write_ptr(target.ptr(), 0usize, new_ptr);
        }
        Ok(())
    }
}

/// Maps an allocation failure to the native-facing error. Resource-limit
/// conditions surface as such; anything else is a VM invariant violation.
fn map_alloc_err(e: RuntimeError) -> VMInternalError {
    match e {
        RuntimeError::OutOfHeapMemory { requested } => {
            VMInternalError::OutOfHeapMemory { requested }
        },
        RuntimeError::AllocationTooLarge { requested } => {
            VMInternalError::AllocationTooLarge { requested }
        },
        RuntimeError::VecAllocSizeOverflow => VMInternalError::VecAllocSizeOverflow,
        other => VMInternalError::InvariantViolation(format!("vector allocation failed: {other}")),
    }
}

/// A family of [`ProductionNativeContext`] types indexed by a lifetime.
pub struct ProductionContextFamily;

impl NativeContextFamily for ProductionContextFamily {
    type Of<'a> = ProductionNativeContext<'a>;
}

/// Shorthand for the [`NativeRegistry`] used by the production VM.
pub type ProductionNativeRegistry = NativeRegistry<ProductionContextFamily>;

/// Shorthand for the [`NativeFunction`] used by the production VM.
pub type ProductionNativeFunction = NativeFunction<ProductionContextFamily>;
