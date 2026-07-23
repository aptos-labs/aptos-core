// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Concrete native-function types used by the production VM.
//!
//! These are internal to the VM; native functions depend only on the
//! [`NativeContext`] trait, never on these types directly.

use crate::{
    error::RuntimeError,
    global_storage::{EntryPtr, ResourceReadWriteSet},
    heap::{
        alloc_or_gc, alloc_vec, deep_copy_or_gc, deserialize_or_gc, heap_alloc, is_heap_ptr, Heap,
        TopFrame,
    },
    memory::{read_ptr, write_enum_tag, write_u64},
    types::{META_SAVED_FP_OFFSET, META_SAVED_FUNC_PTR_OFFSET, VEC_DATA_OFFSET, VEC_LENGTH_OFFSET},
};
use mono_move_core::{
    interner::InternedModuleId,
    native::{
        native_invariant_violation, Boxed, NativeABI, NativeContext, NativeContextFamily,
        NativeExtension, NativeExtensions, NativeFunction, NativeRegistry, Opaque, Ref, RootPool,
        TableHandle, VMValue, Vector,
    },
    storage::resource_provider::InMemoryStorageKey,
    types::InternedType,
    DescriptorId, DescriptorProvider, Function, GasMeter, LayoutProvider, ResourceProvider,
    VMResult, ENUM_DATA_OFFSET, FRAME_METADATA_SIZE, OBJECT_HEADER_SIZE, TRIVIAL_DESCRIPTOR_ID,
};
use move_core_types::account_address::AccountAddress;
use std::{
    cell::{Cell, RefMut, UnsafeCell},
    cmp::Ordering,
    ptr::NonNull,
};

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
    /// TODO(completeness): Expose to native functions.
    #[allow(dead_code)]
    gas: UnsafeCell<&'a mut GasMeter>,
    /// The VM's heap -- used by the natives to allocate new heap objects.
    heap: UnsafeCell<&'a mut Heap>,
    /// The transaction's read write set -- provides global storage access.
    rws: UnsafeCell<&'a mut ResourceReadWriteSet>,
    /// Resource provider backing global-storage reads on a read-set cache miss.
    resource_provider: &'a dyn ResourceProvider,
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
    // TODO(cleanup): revisit this lint.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        frame_ptr: *mut u8,
        abi: &'a NativeABI,
        ty_args: &'a [InternedType],
        gas_meter: &'a mut GasMeter,
        desc_provider: &'a dyn DescriptorProvider,
        layouts: &'a dyn LayoutProvider,
        resource_provider: &'a dyn ResourceProvider,
        heap: &'a mut Heap,
        rws: &'a mut ResourceReadWriteSet,
        extensions: &'a NativeExtensions,
    ) -> Self {
        Self {
            abi,
            ty_args,
            desc_provider,
            layouts,
            resource_provider,
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

    unsafe fn arg<'a, T: VMValue<'a>>(&'a self, i: usize) -> VMResult<T> {
        if self.returns_started.get() {
            return Err(native_invariant_violation(format!(
                "arg({}) called after a return value was written",
                i,
            )));
        }
        let slot = self.abi.args().get(i).copied().ok_or_else(|| {
            native_invariant_violation(format!(
                "arg index {} out of bounds (num_args={})",
                i,
                self.abi.args().len(),
            ))
        })?;
        if T::FRAME_SLOT_SIZE as u32 != slot.size {
            return Err(native_invariant_violation(format!(
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

    fn return_size(&self, i: usize) -> VMResult<usize> {
        self.abi
            .returns()
            .get(i)
            .map(|slot| slot.size as usize)
            .ok_or_else(|| {
                native_invariant_violation(format!(
                    "return index {} out of bounds (num_returns={})",
                    i,
                    self.abi.returns().len(),
                ))
            })
    }

    unsafe fn set_return<'a, T: VMValue<'a>>(&'a self, i: usize, value: T) -> VMResult<()> {
        let slot = self.abi.returns().get(i).copied().ok_or_else(|| {
            native_invariant_violation(format!(
                "return index {} out of bounds (num_returns={})",
                i,
                self.abi.returns().len(),
            ))
        })?;
        if T::FRAME_SLOT_SIZE as u32 != slot.size {
            return Err(native_invariant_violation(format!(
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

    unsafe fn set_return_raw(&self, i: usize, bytes: &[u8]) -> VMResult<()> {
        let slot = self.abi.returns().get(i).copied().ok_or_else(|| {
            native_invariant_violation(format!(
                "return index {} out of bounds (num_returns={})",
                i,
                self.abi.returns().len(),
            ))
        })?;
        if bytes.len() != slot.size as usize {
            return Err(native_invariant_violation(format!(
                "set_return_raw: return slot {} is {} bytes but got {}",
                i,
                slot.size,
                bytes.len(),
            )));
        }
        // SAFETY: the ABI keeps `[offset, offset + size)` within the frame, and
        // the length check makes the copy in-bounds. `bytes` is a valid
        // representation of the slot's type, per this method's contract.
        //
        // TODO(correctness): `bytes` must NOT alias the return slot it is written to.
        // This is currently ensured by the `return_started` flag and the absence
        // of value types that reference the frame.
        // Re-audit this if new value APIs are added.
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

    fn ty_arg(&self, i: usize) -> VMResult<InternedType> {
        self.ty_args.get(i).copied().ok_or_else(|| {
            native_invariant_violation(format!(
                "ty_arg index {} out of bounds (num_ty_args={})",
                i,
                self.ty_args.len(),
            ))
        })
    }

    fn value_size(&self, ty: InternedType) -> VMResult<u32> {
        self.layouts
            .size_and_align(ty)
            .map(|(size, _)| size)
            .ok_or_else(|| {
                native_invariant_violation("value_size: type has no known layout".into())
            })
    }

    fn arg_raw(&self, i: usize) -> VMResult<Vec<u8>> {
        if self.returns_started.get() {
            return Err(native_invariant_violation(format!(
                "arg_raw({i}) called after a return value was written",
            )));
        }
        let slot = self.abi.args().get(i).copied().ok_or_else(|| {
            native_invariant_violation(format!(
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

    fn arg_ptr_offsets(&self, i: usize) -> VMResult<Vec<u32>> {
        let slot = self.abi.args().get(i).copied().ok_or_else(|| {
            native_invariant_violation(format!(
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

    fn new_byte_vector<'a>(&'a self, bytes: &[u8]) -> VMResult<Vector<'a, u8>> {
        if self.returns_started.get() {
            return Err(native_invariant_violation(
                "new_byte_vector called after a return value was written".into(),
            ));
        }
        let len = bytes.len() as u64;
        if len == 0 {
            // TODO(correctness): audit empty <=> null vector invariant
            // SAFETY: passing `null` is always safe.
            let handle = unsafe { self.pool.root_object(std::ptr::null_mut()) };
            return Ok(Vector::from_handle(handle));
        }

        // SAFETY: `heap` and `rws` are distinct fields, so reborrowing both
        // through `&self` at once is sound — at most one `&mut` per field is
        // live (see the type-level aliasing rule).
        let heap = unsafe { &mut **self.heap.get() };
        let rws = unsafe { &mut **self.rws.get() };
        // A heap-aliasing `bytes` would be invalidated by the GC `alloc_vec` may
        // trigger, before the copy below.
        if is_heap_ptr(heap, bytes.as_ptr()) {
            return Err(native_invariant_violation(
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
        )?;
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

    unsafe fn bcs_serialize_value(&self, base: *const u8, ty: InternedType) -> VMResult<Vec<u8>> {
        // SAFETY: forwarded from this method's contract; serialization performs
        // no VM-heap allocation, so `base` stays valid throughout.
        unsafe { crate::value_utils::serialize(self.layouts, base, ty) }
    }

    unsafe fn bcs_serialized_size(&self, base: *const u8, ty: InternedType) -> VMResult<usize> {
        // SAFETY: forwarded from this method's contract.
        unsafe { crate::value_utils::serialized_size(self.layouts, base, ty) }
    }

    fn bcs_deserialize_value(&self, ty: InternedType, bytes: &[u8]) -> VMResult<Vec<u8>> {
        let layout = self.layouts.layout_by_ty(ty).ok_or_else(|| {
            native_invariant_violation("bcs deserialize: no layout for type".into())
        })?;
        let mut out = vec![0u8; layout.size as usize];
        // SAFETY: heap and rws are distinct fields (see the type-level aliasing
        // rule), so reborrowing both through `&self` at once is sound.
        let heap = unsafe { &mut **self.heap.get() };
        let rws = unsafe { &mut **self.rws.get() };
        // `bytes` is off-heap (the native copied it), so it survives the GC the
        // retry may run.
        // SAFETY: `out` is `layout.size` writable bytes.
        unsafe {
            deserialize_or_gc(
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
        }
        .map(|()| out)
    }

    fn resource_exists(&self, address: AccountAddress, ty: InternedType) -> VMResult<bool> {
        // SAFETY: `rws` is reborrowed exclusively here; no other borrow is live.
        let rws = unsafe { &mut **self.rws.get() };
        let key = InMemoryStorageKey::resource(address, ty);
        Ok(rws.exists(self.resource_provider, &key)?)
    }

    fn bcs_serialize_arg(&self, i: usize, ty: InternedType) -> VMResult<Vec<u8>> {
        let slot =
            self.abi.args().get(i).copied().ok_or_else(|| {
                native_invariant_violation(format!("arg index {i} out of bounds"))
            })?;
        // SAFETY: the ABI keeps arg `i` within the frame, holding a value of type
        // `ty` that stays live for the call.
        let base = unsafe { self.frame_ptr.add(slot.offset as usize) };
        unsafe { self.bcs_serialize_value(base, ty) }
    }

    fn required_descriptor(&self, i: usize) -> Option<DescriptorId> {
        self.abi.required_descriptor(i)
    }

    fn constant_serialized_size(&self, ty: InternedType) -> VMResult<Option<u64>> {
        let size = crate::value_utils::fixed_serialized_size(self.layouts, ty)?;
        Ok(size.map(|n| n as u64))
    }

    unsafe fn compare(&self, a: *const u8, b: *const u8, ty: InternedType) -> VMResult<Ordering> {
        // SAFETY: forwarded from this method's contract.
        unsafe { crate::value_utils::compare(self.layouts, a, b, ty) }
    }

    unsafe fn new_enum<'a, V: VMValue<'a>>(
        &'a self,
        descriptor: DescriptorId,
        tag: u64,
        value: V,
    ) -> VMResult<Boxed<'a, Opaque>> {
        let payload = ENUM_DATA_OFFSET + V::FRAME_SLOT_SIZE;
        // SAFETY: heap and rws are distinct fields (see the aliasing rule).
        let heap = unsafe { &mut **self.heap.get() };
        let rws = unsafe { &mut **self.rws.get() };
        let obj = alloc_or_gc(
            heap,
            self.desc_provider,
            rws,
            &self.pool,
            self.extensions,
            self.frame_ptr,
            TopFrame::Native(self.abi),
            |h| heap_alloc(h, OBJECT_HEADER_SIZE + payload, descriptor),
        )?;
        // SAFETY: `obj` has `payload` bytes; no GC runs before these writes.
        unsafe {
            write_enum_tag(obj, tag);
            value.write_to_frame(obj, ENUM_DATA_OFFSET);
        }
        // SAFETY: `obj` is a freshly allocated, live heap object.
        Ok(Boxed::from_handle(unsafe { self.pool.root_object(obj) }))
    }

    fn table_contains(
        &self,
        handle: &TableHandle,
        key: &[u8],
        value_ty: InternedType,
    ) -> VMResult<bool> {
        // SAFETY: `rws` is reborrowed exclusively here.
        let rws = unsafe { &mut **self.rws.get() };
        let storage_key = InMemoryStorageKey::table_item(*handle, key.into(), value_ty);
        Ok(rws.exists(self.resource_provider, &storage_key)?)
    }

    fn table_borrow(
        &self,
        handle: &TableHandle,
        key: &[u8],
        mutable: bool,
        value_ty: InternedType,
    ) -> VMResult<Option<Ref<'_, Opaque>>> {
        let storage_key = InMemoryStorageKey::table_item(*handle, key.into(), value_ty);
        // SAFETY: heap and rws are distinct fields (see the aliasing rule).
        let rws = unsafe { &mut **self.rws.get() };
        let ptr = if mutable {
            match rws.try_borrow_global_mut(self.resource_provider, &storage_key) {
                Ok(EntryPtr::Writable(ptr)) => ptr,
                Ok(EntryPtr::NonWritable(ptr)) => {
                    // Copy-on-write: an external or stale value must be copied
                    // into the local heap before it can be mutated.
                    let heap = unsafe { &mut **self.heap.get() };
                    // SAFETY: `ptr` is a live object (provider- or older-epoch-owned).
                    let copied = unsafe {
                        deep_copy_or_gc(
                            heap,
                            self.desc_provider,
                            rws,
                            &self.pool,
                            self.extensions,
                            self.frame_ptr,
                            TopFrame::Native(self.abi),
                            ptr,
                        )
                    }?;
                    rws.commit_borrow_global_mut(&storage_key, copied);
                    copied
                },
                Err(RuntimeError::ResourceDoesNotExist { .. }) => return Ok(None),
                Err(e) => return Err(e.into()),
            }
        } else {
            match rws.borrow_global(self.resource_provider, &storage_key) {
                Ok(ptr) => ptr,
                Err(RuntimeError::ResourceDoesNotExist { .. }) => return Ok(None),
                Err(e) => return Err(e.into()),
            }
        };
        // SAFETY: `ptr` is the live entry value; the reference points at its
        // start, so the offset is 0. The pool roots it for the rest of the call.
        let handle = unsafe { self.pool.root_reference(ptr.as_ptr(), 0) };
        Ok(Some(Ref::from_handle(handle)))
    }

    // TODO(cleanup): See if there's a way to separate out argument-reading from boxing.
    //       Currently they are both handled here for GC-safety.
    fn box_arg<'a>(
        &'a self,
        value_arg: usize,
        descriptor: DescriptorId,
    ) -> VMResult<Boxed<'a, Opaque>> {
        if self.returns_started.get() {
            return Err(native_invariant_violation(format!(
                "box_arg({value_arg}) called after a return value was written"
            )));
        }
        let slot = self.abi.args().get(value_arg).copied().ok_or_else(|| {
            native_invariant_violation(format!("box_arg: arg index {value_arg} out of bounds"))
        })?;
        let size = slot.size as usize;
        // SAFETY: heap and rws are distinct fields (see the aliasing rule).
        let heap = unsafe { &mut **self.heap.get() };
        let rws = unsafe { &mut **self.rws.get() };

        // Allocate first, then copy the value out of the frame, so to be
        // GC-safe.
        let obj = alloc_or_gc(
            heap,
            self.desc_provider,
            rws,
            &self.pool,
            self.extensions,
            self.frame_ptr,
            TopFrame::Native(self.abi),
            |h| heap_alloc(h, OBJECT_HEADER_SIZE + size, descriptor),
        )?;
        // SAFETY: the arg slot lies within the frame and `obj` has `size` payload
        // bytes; no GC runs between the allocation and this copy.
        unsafe {
            std::ptr::copy_nonoverlapping(self.frame_ptr.add(slot.offset as usize), obj, size);
        }
        // Root the boxed object so it survives later allocations and is
        // GC-relocated.
        // SAFETY: `obj` is a freshly allocated, live heap object.
        Ok(Boxed::from_handle(unsafe { self.pool.root_object(obj) }))
    }

    fn table_add(
        &self,
        handle: &TableHandle,
        key: &[u8],
        value: Boxed<'_, Opaque>,
        value_ty: InternedType,
    ) -> VMResult<bool> {
        let storage_key = InMemoryStorageKey::table_item(*handle, key.into(), value_ty);
        let obj = NonNull::new(value.ptr())
            .ok_or_else(|| native_invariant_violation("table_add: null boxed value".into()))?;
        // SAFETY: `rws` is reborrowed exclusively here.
        let rws = unsafe { &mut **self.rws.get() };
        match rws.move_to(self.resource_provider, &storage_key, obj) {
            Ok(()) => Ok(true),
            Err(RuntimeError::ResourceAlreadyExists { .. }) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    fn table_remove(
        &self,
        handle: &TableHandle,
        key: &[u8],
        value_ty: InternedType,
    ) -> VMResult<Option<Boxed<'_, Opaque>>> {
        let storage_key = InMemoryStorageKey::table_item(*handle, key.into(), value_ty);
        // SAFETY: heap and rws are distinct fields (see the aliasing rule).
        let rws = unsafe { &mut **self.rws.get() };
        let ptr = match rws.try_move_from(self.resource_provider, &storage_key) {
            Ok(EntryPtr::Writable(ptr)) => ptr,
            Ok(EntryPtr::NonWritable(ptr)) => {
                // Copy-on-write: an external or older-epoch value must be copied
                // into the local heap before it is taken out.
                let heap = unsafe { &mut **self.heap.get() };
                // SAFETY: `ptr` is a live object (provider- or older-epoch-owned).
                let copied = unsafe {
                    deep_copy_or_gc(
                        heap,
                        self.desc_provider,
                        rws,
                        &self.pool,
                        self.extensions,
                        self.frame_ptr,
                        TopFrame::Native(self.abi),
                        ptr,
                    )
                }?;
                rws.commit_move_from(&storage_key);
                copied
            },
            Err(RuntimeError::ResourceDoesNotExist { .. }) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        // Root the removed object so it survives later GC
        // SAFETY: `ptr` is the live entry value just taken out of storage.
        Ok(Some(Boxed::from_handle(unsafe {
            self.pool.root_object(ptr.as_ptr())
        })))
    }

    fn get_extension<T: NativeExtension>(&self) -> VMResult<RefMut<'_, T>> {
        self.extensions.get_mut::<T>()
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
