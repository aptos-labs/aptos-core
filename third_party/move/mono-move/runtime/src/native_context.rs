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
    memory::{read_ptr, write_u64},
    types::{META_SAVED_FP_OFFSET, META_SAVED_FUNC_PTR_OFFSET, VEC_DATA_OFFSET, VEC_LENGTH_OFFSET},
};
use mono_move_core::{
    interner::InternedModuleId,
    native::{
        BcsError, NativeABI, NativeContext, NativeContextFamily, NativeExtension, NativeExtensions,
        NativeFunction, NativeRegistry, RootPool, VMInternalError, VMValue, Vector,
    },
    types::InternedType,
    DescriptorProvider, Function, GasMeter, LayoutProvider, FRAME_METADATA_SIZE,
    TRIVIAL_DESCRIPTOR_ID,
};
use std::cell::{Cell, RefMut, UnsafeCell};

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
    /// Value layouts, used by natives that serialize, deserialize, or compare
    /// values driven by their types.
    layouts: &'a dyn LayoutProvider,
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
    /// Per-transaction native extensions, shared across native calls. Accessed
    /// sharedly — each extension's own [`RefCell`](std::cell::RefCell) provides
    /// the interior mutability.
    extensions: &'a NativeExtensions,
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
        layouts: &'a dyn LayoutProvider,
        heap: &'a mut Heap,
        rws: &'a mut ResourceReadWriteSet,
        extensions: &'a NativeExtensions,
    ) -> Self {
        Self {
            abi,
            ty_args,
            desc_provider,
            layouts,
            frame_ptr,
            gas: UnsafeCell::new(gas_meter),
            heap: UnsafeCell::new(heap),
            rws: UnsafeCell::new(rws),
            extensions,
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
            return Err(VMInternalError::invariant_violation(format!(
                "arg({}) called after a return value was written",
                i,
            )));
        }
        let slot = self.abi.args().get(i).copied().ok_or_else(|| {
            VMInternalError::invariant_violation(format!(
                "arg index {} out of bounds (num_args={})",
                i,
                self.abi.args().len(),
            ))
        })?;
        if T::FRAME_SLOT_SIZE as u32 != slot.size {
            return Err(VMInternalError::invariant_violation(format!(
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
        // `T` is responsible for the correctness of its own `read_from_frame` impl.
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
            VMInternalError::invariant_violation(format!(
                "return index {} out of bounds (num_returns={})",
                i,
                self.abi.returns().len(),
            ))
        })?;
        if T::FRAME_SLOT_SIZE as u32 != slot.size {
            return Err(VMInternalError::invariant_violation(format!(
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

    unsafe fn set_return_raw(&self, i: usize, bytes: &[u8]) -> Result<(), VMInternalError> {
        let slot = self.abi.returns().get(i).copied().ok_or_else(|| {
            VMInternalError::invariant_violation(format!(
                "return index {} out of bounds (num_returns={})",
                i,
                self.abi.returns().len(),
            ))
        })?;
        if bytes.len() != slot.size as usize {
            return Err(VMInternalError::invariant_violation(format!(
                "set_return_raw: return slot {} is {} bytes but got {}",
                i,
                slot.size,
                bytes.len(),
            )));
        }
        // SAFETY: the ABI keeps `[offset, offset + size)` within the frame, and
        // the length check makes the copy in-bounds. `bytes` is a valid
        // representation of the slot's type, per this method's contract.
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                self.frame_ptr.add(slot.offset as usize),
                bytes.len(),
            );
        }
        self.returns_started.set(true);
        Ok(())
    }

    fn num_ty_args(&self) -> usize {
        self.ty_args.len()
    }

    fn ty_arg(&self, i: usize) -> Result<InternedType, VMInternalError> {
        self.ty_args.get(i).copied().ok_or_else(|| {
            VMInternalError::invariant_violation(format!(
                "ty_arg index {} out of bounds (num_ty_args={})",
                i,
                self.ty_args.len(),
            ))
        })
    }

    fn arg_raw(&self, i: usize) -> Result<Vec<u8>, VMInternalError> {
        if self.returns_started.get() {
            return Err(VMInternalError::invariant_violation(format!(
                "arg_raw({i}) called after a return value was written",
            )));
        }
        let slot = self.abi.args().get(i).copied().ok_or_else(|| {
            VMInternalError::invariant_violation(format!(
                "arg index {} out of bounds (num_args={})",
                i,
                self.abi.args().len(),
            ))
        })?;
        // SAFETY: the ABI keeps `[offset, offset + size)` within the frame, and
        // the caller wrote the argument's bytes there before the native ran.
        let bytes = unsafe {
            std::slice::from_raw_parts(self.frame_ptr.add(slot.offset as usize), slot.size as usize)
        };
        Ok(bytes.to_vec())
    }

    fn arg_ptr_offsets(&self, i: usize) -> Result<Vec<u32>, VMInternalError> {
        let slot = self.abi.args().get(i).copied().ok_or_else(|| {
            VMInternalError::invariant_violation(format!(
                "arg index {} out of bounds (num_args={})",
                i,
                self.abi.args().len(),
            ))
        })?;
        // The ABI's heap-pointer offsets are frame-relative and span all args;
        // keep the ones inside this arg's slot and rebase them to the arg start.
        Ok(self
            .abi
            .heap_ptr_offsets()
            .iter()
            .map(|o| o.0)
            .filter(|&o| slot.offset <= o && o < slot.offset + slot.size)
            .map(|o| o - slot.offset)
            .collect())
    }

    fn caller_module(&self) -> Option<InternedModuleId> {
        // Walk two frames up: the native's metadata records its immediate
        // caller's frame pointer, and that caller's metadata records *its*
        // caller. A null saved-function pointer marks the entry frame, which
        // has no caller.
        unsafe {
            let caller_fp = read_ptr(
                self.frame_ptr.sub(FRAME_METADATA_SIZE),
                META_SAVED_FP_OFFSET,
            );
            let caller_caller = read_ptr(
                caller_fp.sub(FRAME_METADATA_SIZE),
                META_SAVED_FUNC_PTR_OFFSET,
            ) as *const Function;
            caller_caller.as_ref().map(|f| f.module_id)
        }
    }

    fn new_byte_vector<'a>(&'a self, bytes: &[u8]) -> Result<Vector<'a, u8>, VMInternalError> {
        if self.returns_started.get() {
            return Err(VMInternalError::invariant_violation(
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
            return Err(VMInternalError::invariant_violation(
                "new_byte_vector: bytes must not alias the VM heap".into(),
            ));
        }
        // A `vector<u8>` has no inner pointers, so it uses the trivial descriptor.
        let ptr = alloc_vec(
            heap,
            self.desc_provider,
            rws,
            &self.pool,
            self.extensions,
            self.frame_ptr,
            TopFrame::Native(self.abi),
            TRIVIAL_DESCRIPTOR_ID,
            1,
            len,
        )
        .map_err(VMInternalError::from)?;
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

    unsafe fn bcs_serialize_value(
        &self,
        base: *const u8,
        ty: InternedType,
    ) -> Result<Result<Vec<u8>, BcsError>, VMInternalError> {
        // SAFETY: forwarded from this method's contract; serialization performs
        // no VM-heap allocation, so `base` stays valid throughout.
        match unsafe { crate::value_utils::serialize(self.layouts, base, ty) } {
            Ok(bytes) => Ok(Ok(bytes)),
            Err(e) => classify_serde_error(e).map(Err),
        }
    }

    fn bcs_deserialize_value(
        &self,
        ty: InternedType,
        bytes: &[u8],
    ) -> Result<Result<Vec<u8>, BcsError>, VMInternalError> {
        let layout = self.layouts.layout_by_ty(ty).ok_or_else(|| {
            VMInternalError::invariant_violation("bcs deserialize: no layout for type".into())
        })?;
        let mut out = vec![0u8; layout.size as usize];
        // SAFETY: heap and rws are distinct fields (see the type-level aliasing
        // rule), so reborrowing both through `&self` at once is sound.
        let heap = unsafe { &mut **self.heap.get() };
        let rws = unsafe { &mut **self.rws.get() };
        // `bytes` is off-heap (the native copied it), so it survives the GC the
        // retry may run.
        // SAFETY: `out` is `layout.size` writable bytes.
        let result = unsafe {
            crate::value_utils::deserialize_or_gc(
                self.layouts,
                heap,
                ty,
                bytes,
                out.as_mut_ptr(),
                self.desc_provider,
                rws,
                &self.pool,
                self.extensions,
                self.frame_ptr,
                TopFrame::Native(self.abi),
            )
        };
        match result {
            Ok(()) => Ok(Ok(out)),
            Err(e) => classify_serde_error(e).map(Err),
        }
    }

    fn get_extension<T: NativeExtension>(&self) -> Result<RefMut<'_, T>, VMInternalError> {
        self.extensions.get_mut::<T>()
    }
}

/// Splits a value-walk error into a malformed-data `BcsError` and a VM error.
fn classify_serde_error(err: RuntimeError) -> Result<BcsError, VMInternalError> {
    match err {
        RuntimeError::BCSEof => Ok(BcsError::UnexpectedEof),
        RuntimeError::BCSInvalidUleb => Ok(BcsError::MalformedLength),
        RuntimeError::BCSSequenceTooLong { len } => Ok(BcsError::SequenceTooLong { len }),
        RuntimeError::BCSRemainingInput { remaining } => Ok(BcsError::TrailingBytes { remaining }),
        other => Err(VMInternalError::from(other)),
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
