// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Interpreter with unified stack (frame metadata inlined in linear memory)
//! and a bump-allocated heap with copying GC.

use crate::{
    error::{ArithOp, RuntimeError, RuntimeInvariantViolation, RuntimeStatus, Signedness, VecOp},
    global_storage::{EntryPtr, ResourceReadWriteSet},
    heap::{
        deep_copy_or_gc, deserialize_or_gc,
        macros::{alloc_captured_data, alloc_obj, alloc_vec, gc_collect, grow_vec_ref},
        AllocationError, Heap, TopFrame,
    },
    invariant_violation,
    memory::{
        read_account_address, read_bool, read_descriptor, read_enum_tag, read_fat_ptr,
        read_obj_size, read_ptr, read_u32, read_u64, read_u8, read_vec_len, vec_elem_ptr,
        write_bool, write_enum_tag, write_fat_ptr, write_ptr, write_u32, write_u64, write_u8,
        MemoryRegion,
    },
    native_context::{ProductionNativeContext, ProductionNativeRegistry},
    types::{
        ABORT_MESSAGE_SIZE_LIMIT, DEFAULT_HEAP_SIZE, DEFAULT_STACK_SIZE, META_SAVED_FP_OFFSET,
        META_SAVED_FUNC_PTR_OFFSET, META_SAVED_PC_OFFSET, VEC_DATA_OFFSET, VEC_LENGTH_OFFSET,
        VEC_PUSHBACK_INIT_CAPACITY,
    },
    value_utils,
};
use aptos_types::write_set::WriteSet;
use mono_move_core::{
    captured_values_size,
    interner::{InternedIdentifier, InternedModuleId},
    native::{NativeABI, NativeExtensions, NativeIdx, NativeStatus, ObjectHandle, RootPool},
    next_captured_value_offset,
    storage::resource_provider::InMemoryStorageKey,
    types::{view_type_list, InternedType, InternedTypeList},
    value_layout::LayoutProvider,
    CallClosureOp, ClosureFuncRef, CmpKind, CodeOffset, ConstantPoolIndex, DescriptorId,
    FrameOffset, Function, FunctionPtr, FunctionRef, GasMeter, IntBinaryOp, IntCastOp, IntNegateOp,
    IntOperand, IntShiftOp, IntTy, MicroOp, PackClosureOp, ResourceProvider, ShiftOperand,
    VMInternalError, VMResult, VecPackOp, VecUnpackOp, CAPTURED_DATA_TAG_MATERIALIZED,
    CAPTURED_DATA_TAG_OFFSET, CAPTURED_DATA_VALUES_OFFSET, CAPTURED_DATA_VALUES_SIZE_OFFSET,
    CLOSURE_CAPTURED_DATA_PTR_OFFSET, CLOSURE_DESCRIPTOR_ID, CLOSURE_FUNC_REF_OFFSET,
    CLOSURE_MASK_OFFSET, FRAME_METADATA_SIZE, FUNC_REF_PAYLOAD_OFFSET, FUNC_REF_TAG_OFFSET,
    FUNC_REF_TAG_RESOLVED, FUNC_REF_TAG_UNRESOLVED, MAX_ALIGN, OBJECT_HEADER_SIZE,
};
use mono_move_loader::{Loader, ModuleReadSet};
use move_core_types::int256::{I256, U256};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::ptr::{null, NonNull};
// ---------------------------------------------------------------------------
// Runtime state
// ---------------------------------------------------------------------------

/// The VM's virtual registers.
///
/// # Performance note
///
/// The dispatch loop operates on a local copy that the compiler can hopefully
/// keep in real CPU registers, writing it back only on entry/exit. A reference
/// to these must never escape into a non-inlined function, or the compiler may
/// be forced to spill them to the stack.
#[derive(Clone, Copy)]
pub(crate) struct VMRegisters {
    /// Index of the next instruction to execute within `func`'s code.
    pub(crate) pc: usize,
    /// Frame pointer of the current call frame.
    pub(crate) fp: *mut u8,
    /// The function currently executing.
    pub(crate) func: NonNull<Function>,
}

impl VMRegisters {
    /// Registers initialized with a fresh root frame, ready to begin execution.
    fn new(stack_base: *mut u8, func: &Function) -> Self {
        Self {
            pc: 0,
            // SAFETY: `stack_base` points to a stack allocation far larger than
            // `FRAME_METADATA_SIZE`, so the offset stays in bounds.
            fp: unsafe { stack_base.add(FRAME_METADATA_SIZE) },
            func: NonNull::from(func),
        }
    }
}

/// What a finished transaction leaves behind: the frozen heap, the
/// global-storage read-write set, and the native extensions (event store and
/// friends). Read-write-set and extension entries point into `heap`, so the
/// parts are only valid together; external read pointers remain owned by the
/// resource provider.
///
// TODO(correctness): the effects hold interned types but carry no lifetime, so
// they can outlive the guard and dereference freed arenas after a maintenance
// reset. Tie them to the guard lifetime (`SessionEffects<'guard>`).
pub struct SessionEffects {
    pub heap: Heap,
    pub read_write_set: ResourceReadWriteSet,
    pub extensions: NativeExtensions,
}

impl SessionEffects {
    /// The transaction's write set.
    pub fn write_set(&self, layouts: &impl LayoutProvider) -> VMResult<WriteSet> {
        crate::write_set::build_write_set(&self.read_write_set, layouts)
    }
}

/// Per-transaction interpreter context with a unified call stack and a
/// GC-managed heap: owns the transaction state (code loader and read-set, gas
/// meter, native extensions, resource read-write set) and the machine state
/// (stack, heap, VM registers). Interpreter sessions are [`Self::invoke`] /
/// [`Self::reset`] calls on the same context, so state like the heap and the
/// read-write set lives across sessions within one transaction.
///
/// Construction wires in the transaction inputs (loader, read-set and gas
/// meter carried over from loading the entry function, resource provider,
/// natives) and verifies the entry function; further sessions reuse the
/// buffers via [`reset`](Self::reset).
pub struct InterpreterContext<'guard> {
    /// Per-transaction code loader; also the access point for the execution
    /// guard (descriptor/layout lookups). The loader's global-context
    /// lifetime is shrunk to `'guard` (covariance): nothing the interpreter
    /// exposes outlives the guard.
    pub(crate) loader: Loader<'guard, 'guard>,
    /// Read-set of the modules loaded by this transaction.
    read_set: ModuleReadSet<'guard>,
    pub(crate) gas_meter: GasMeter,
    // TODO(cleanup): Move the native registry off the per-transaction context
    // and onto a long-lived owner (e.g. the global context).
    //
    // TODO(correctness): Enforce that `natives` here and the `NativeResolver`
    // passed to `loader` are the same instance.
    natives: &'guard ProductionNativeRegistry,
    /// Per-transaction native extensions, shared across native calls.
    pub(crate) extensions: NativeExtensions,
    resource_provider: &'guard dyn ResourceProvider,

    /// The VM registers; the dispatch loop works on a local copy and writes it
    /// back here on exit.
    pub(crate) registers: VMRegisters,
    stack: MemoryRegion,
    pub(crate) heap: Heap,
    /// Auxiliary GC root set for temporarily-live heap pointers that are
    /// not yet stored in any frame slot (e.g. between two allocations in a
    /// fused micro-op, or in native functions).
    pub(crate) root_pool: RootPool,
    /// Per-transaction global-storage state: working map of cached
    /// reads / pending writes, linear journal for rollback, and
    /// checkpoint stack.
    pub(crate) read_write_set: ResourceReadWriteSet,
    rng: StdRng,
}

impl<'guard> InterpreterContext<'guard> {
    pub fn new(
        loader: Loader<'guard, 'guard>,
        read_set: ModuleReadSet<'guard>,
        gas_meter: GasMeter,
        resource_provider: &'guard dyn ResourceProvider,
        natives: &'guard ProductionNativeRegistry,
        entry: &Function,
    ) -> Self {
        Self::with_heap_size(
            loader,
            read_set,
            gas_meter,
            resource_provider,
            natives,
            entry,
            DEFAULT_HEAP_SIZE,
        )
    }

    /// Create a new context with a custom heap size (for testing GC pressure).
    /// Verifies `entry` and installs it, ready for [`run`](Self::run); panics
    /// if verification fails. `read_set` and `gas_meter` carry over the
    /// entry-load state (the entry's module must be in the read-set for its
    /// constants and call targets to resolve).
    pub fn with_heap_size(
        loader: Loader<'guard, 'guard>,
        read_set: ModuleReadSet<'guard>,
        gas_meter: GasMeter,
        resource_provider: &'guard dyn ResourceProvider,
        natives: &'guard ProductionNativeRegistry,
        entry: &Function,
        heap_size: usize,
    ) -> Self {
        let verification_errors = crate::verifier::verify_function(entry, loader.guard());
        assert!(
            verification_errors.is_empty(),
            "verification failed:\n{}",
            verification_errors
                .iter()
                .map(|e| format!("  {}", e))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let stack = MemoryRegion::new(DEFAULT_STACK_SIZE);
        let base = stack.as_ptr();

        unsafe {
            write_u64(base, META_SAVED_PC_OFFSET, 0);
            write_u64(base, META_SAVED_FP_OFFSET, 0);
            write_ptr(base, META_SAVED_FUNC_PTR_OFFSET, null());
        }

        Self {
            loader,
            read_set,
            gas_meter,
            natives,
            extensions: NativeExtensions::new(),
            resource_provider,
            registers: VMRegisters::new(base, entry),
            stack,
            heap: Heap::new(heap_size),
            root_pool: RootPool::new(),
            read_write_set: ResourceReadWriteSet::new(),
            rng: StdRng::seed_from_u64(0),
        }
    }

    /// Install the per-transaction native extensions. Replaces any previously
    /// installed set.
    pub fn with_extensions(mut self, extensions: NativeExtensions) -> Self {
        self.extensions = extensions;
        self
    }

    /// Resolve a runtime function call: look the target up in the read-set,
    /// falling back to the [`Loader`] on cache miss. May trigger lazy module
    /// loading, gas charge on a cache miss, and lowering of the function's
    /// code.
    pub fn load_function(
        &mut self,
        module_id: InternedModuleId,
        name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> VMResult<FunctionPtr> {
        self.loader.load_function(
            &mut self.read_set,
            &mut self.gas_meter,
            module_id,
            name,
            ty_args,
        )
    }

    /// Resolve a constant from `module_id`'s constant pool, returning its
    /// interned type and BCS bytes. The calling function was loaded from
    /// `module_id`, so the module is always present and loaded in the read
    /// set; a missing or not-yet-loaded entry is an invariant violation.
    fn load_constant(
        &self,
        module_id: InternedModuleId,
        idx: ConstantPoolIndex,
    ) -> VMResult<(InternedType, &'guard [u8])> {
        let arena_ref = self.loader.guard().arena_ref_for_module_id(module_id);
        let module = &self.read_set.get_loaded(arena_ref)?.ir().module;
        Ok((
            module.interned_constant_type_at(idx),
            module.constant_data_at(idx),
        ))
    }

    /// Returns the transaction's read-set.
    pub fn read_set(&self) -> &ModuleReadSet<'guard> {
        &self.read_set
    }

    pub fn set_rng_seed(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed);
    }

    pub fn gc_count(&self) -> usize {
        self.heap.gc_count
    }

    /// The VM heap. Exposed so callers can read heap-resident values.
    pub fn heap(&self) -> &Heap {
        &self.heap
    }

    pub fn extensions(&self) -> &NativeExtensions {
        &self.extensions
    }

    /// Takes a checkpoint (opening a new sub-session): checkpoints the
    /// read-write set and signals every native extension. The two advance in
    /// lockstep, so a single [`Self::rollback`] depth undoes a checkpoint's
    /// effects across both.
    pub fn checkpoint(&mut self) -> VMResult<()> {
        self.read_write_set.checkpoint();
        self.extensions.checkpoint()
    }

    /// Rolls back the `n` most recent checkpoints across the read-write set and
    /// every native extension. `n == 0` is a no-op; `n` beyond the current
    /// depth is an invariant violation. The read-write set rolls back first, so
    /// an underflow is caught before any extension is touched.
    pub fn rollback(&mut self, n: usize) -> VMResult<()> {
        self.read_write_set.rollback(n)?;
        self.extensions.rollback(n)
    }

    pub fn checkpoint_depth(&self) -> usize {
        self.read_write_set.checkpoint_depth()
    }

    pub fn current_epoch(&self) -> u64 {
        self.read_write_set.current_epoch()
    }

    pub fn journal_len(&self) -> usize {
        self.read_write_set.journal_len()
    }

    /// Remaining gas balance.
    pub fn gas_balance(&self) -> u64 {
        self.gas_meter.balance()
    }

    /// Consumes the context, returning the transaction's side effects for
    /// publication. No execution can follow, so the heap is frozen and every
    /// heap pointer inside the read-write set and extensions stays valid for
    /// as long as the returned effects live.
    pub fn finish(self) -> SessionEffects {
        SessionEffects {
            heap: self.heap,
            read_write_set: self.read_write_set,
            extensions: self.extensions,
        }
    }

    /// The transaction's write set, read out of the still-live context.
    ///
    /// Used for testing and benchmarking.
    pub fn write_set(&self) -> VMResult<WriteSet> {
        crate::write_set::build_write_set(&self.read_write_set, self.loader.guard())
    }

    /// Reset the context to call a different function, preserving the heap.
    ///
    /// Use `set_root_arg` to place arguments before calling `run()`.
    pub fn invoke(&mut self, func: &Function) {
        let base = self.stack.as_ptr();

        // Reset execution state to root frame.
        self.registers = VMRegisters::new(base, func);

        // Re-write sentinel metadata so Return from root triggers Done.
        unsafe {
            write_u64(base, META_SAVED_PC_OFFSET, 0);
            write_u64(base, META_SAVED_FP_OFFSET, 0);
            write_ptr(base, META_SAVED_FUNC_PTR_OFFSET, null());
        }

        // Zero everything beyond parameters (locals, metadata, callee
        // arg/return region) so pointer slots start as null.
        if func.zero_frame {
            unsafe {
                std::ptr::write_bytes(
                    self.registers.fp.add(func.param_region_size),
                    0,
                    func.extended_frame_size - func.param_region_size,
                );
            }
        }
    }

    /// Resets the context to run `func` again from a clean state, reusing the already-allocated
    /// stack and heap buffers instead of reallocating them. Place arguments with
    /// [`set_root_arg`](Self::set_root_arg) before calling [`run`](Self::run).
    pub fn reset(&mut self, func: &Function, gas_budget: u64) {
        self.invoke(func);
        self.heap.reset();
        self.read_write_set = ResourceReadWriteSet::new();
        self.root_pool = RootPool::new();
        self.gas_meter.reset(gas_budget);
        self.rng = StdRng::seed_from_u64(0);
    }

    /// Read a u64 from the root frame's slot 0 (where the result lands).
    pub fn root_result(&self) -> u64 {
        unsafe { read_u64(self.stack.as_ptr(), FRAME_METADATA_SIZE) }
    }

    /// Read a u64 from the root frame at the given byte offset.
    pub fn root_result_at(&self, offset: u32) -> u64 {
        unsafe { read_u64(self.stack.as_ptr(), FRAME_METADATA_SIZE + offset as usize) }
    }

    /// Read `size` raw bytes from the root frame at the given byte offset. For
    /// tests inspecting an entry/native function's raw return slots.
    pub fn root_result_bytes_for_test(&self, offset: u32, size: u32) -> &[u8] {
        unsafe {
            let base = self
                .stack
                .as_ptr()
                .add(FRAME_METADATA_SIZE + offset as usize);
            std::slice::from_raw_parts(base, size as usize)
        }
    }

    /// Reads a heap `vector<u8>` (or a `String`, same slot layout) from the root
    /// result slot at `offset`; empty if the pointer is null. For tests.
    pub fn root_result_byte_vector_for_test(&self, offset: u32) -> Vec<u8> {
        // SAFETY: the slot holds a live pointer to a heap vector<u8>; the heap
        // is still owned by this context, so the read stays in bounds.
        unsafe {
            let ptr = read_ptr(self.stack.as_ptr(), FRAME_METADATA_SIZE + offset as usize);
            if ptr.is_null() {
                return vec![];
            }
            let len = read_u64(ptr, VEC_LENGTH_OFFSET) as usize;
            std::slice::from_raw_parts(ptr.add(VEC_DATA_OFFSET), len).to_vec()
        }
    }

    /// Copy argument bytes into the root frame at the given byte offset.
    pub fn set_root_arg(&mut self, offset: u32, arg: &[u8]) {
        unsafe {
            let dst = self
                .stack
                .as_ptr()
                .add(FRAME_METADATA_SIZE + offset as usize);
            std::ptr::copy_nonoverlapping(arg.as_ptr(), dst, arg.len());
        }
    }

    /// Read a raw heap pointer from the root frame at the given byte offset.
    pub fn root_heap_ptr(&self, offset: u32) -> *const u8 {
        unsafe { read_ptr(self.stack.as_ptr(), FRAME_METADATA_SIZE + offset as usize) }
    }

    /// Deserialize a BCS-encoded argument of type `ty` into the root frame at `offset`, allocating
    /// any nested heap data on this context's heap. Handles struct/vector arguments, unlike
    /// [`set_root_arg`](Self::set_root_arg) (a raw copy for primitives only).
    ///
    /// # Safety
    ///
    /// `offset` and `ty` must correspond to a real parameter slot, and `ty` must not be a reference.
    pub unsafe fn deserialize_root_arg(
        &mut self,
        offset: u32,
        ty: InternedType,
        bytes: &[u8],
    ) -> VMResult<()> {
        let dst = unsafe {
            self.stack
                .as_ptr()
                .add(FRAME_METADATA_SIZE + offset as usize)
        };
        // SAFETY: `dst` is a slot in the root frame (caller guarantees offset/ty match a
        // parameter); the guard is the LayoutProvider and `heap` is where nested data is boxed.
        unsafe {
            value_utils::deserialize_into(self.loader.guard(), &mut self.heap, ty, bytes, dst)
        }
    }

    /// Allocate a vector of `u64` values on the heap and return its address
    /// as a `u64` suitable for embedding in args. Useful for passing pre-built
    /// data into a program without generating initialization micro-ops.
    pub fn alloc_u64_vec(&mut self, descriptor_id: DescriptorId, values: &[u64]) -> VMResult<u64> {
        let n = values.len() as u64;
        let ptr = alloc_vec!(
            self,
            self.registers.fp,
            self.registers.pc,
            self.registers.func,
            descriptor_id,
            8,
            n
        )?;
        unsafe {
            write_u64(ptr, VEC_LENGTH_OFFSET, n);
            let data = ptr.add(VEC_DATA_OFFSET);
            for (i, &v) in values.iter().enumerate() {
                write_u64(data, i * 8, v);
            }
        }
        Ok(ptr as u64)
    }
}

// ---------------------------------------------------------------------------
// Arithmetic helpers
// ---------------------------------------------------------------------------
//
// All arithmetic micro-ops follow one of a few shapes — read 1 or 2 u64
// frame slots, apply a (possibly fallible) computation, write the result
// to a destination slot. The helpers below capture each shape so the
// `step()` arms stay one line each and the read/write boilerplate lives
// in one place.
//
// `#[inline(always)]` ensures the closures and the helper itself are
// folded into the caller in release builds. Inlining verified by
// inspecting the release-build x64 asm for `step::<SimpleGasMeter>` on
// 2026-04-30: zero standalone definitions for any helper or closure,
// zero call/jmp instructions targeting them, and individual arms
// compile to a handful of direct memory ops (e.g. AddU64 is 4 movs +
// addq + jae + jmp).
//
// TODO(perf): re-verify inlining after non-trivial changes to the helpers,
// the call sites, or the rustc/LLVM versions the workspace pins to.

/// `dst <- op(lhs_slot, rhs_slot)` (infallible).
#[inline(always)]
unsafe fn binop_u64<F: FnOnce(u64, u64) -> u64>(
    fp: *mut u8,
    dst: FrameOffset,
    lhs: FrameOffset,
    rhs: FrameOffset,
    op: F,
) {
    // SAFETY: `fp` is the current frame pointer and `lhs`/`rhs`/`dst` are
    // in-bounds 8-byte slots within that frame (enforced by the verifier).
    unsafe {
        let a = read_u64(fp, lhs);
        let b = read_u64(fp, rhs);
        write_u64(fp, dst, op(a, b));
    }
}

/// `dst <- op(lhs_slot, rhs_slot)` (fallible).
#[inline(always)]
unsafe fn checked_binop_u64<F: FnOnce(u64, u64) -> Option<u64>>(
    fp: *mut u8,
    dst: FrameOffset,
    lhs: FrameOffset,
    rhs: FrameOffset,
    op: F,
) -> Option<()> {
    // SAFETY: `fp` is the current frame pointer and `lhs`/`rhs`/`dst` are
    // in-bounds 8-byte slots within that frame (enforced by the verifier).
    unsafe {
        let a = read_u64(fp, lhs);
        let b = read_u64(fp, rhs);
        let v = op(a, b)?;
        write_u64(fp, dst, v);
        Some(())
    }
}

/// `dst <- op(src_slot, imm)` (infallible).
#[inline(always)]
unsafe fn imm_op_u64<F: FnOnce(u64, u64) -> u64>(
    fp: *mut u8,
    dst: FrameOffset,
    src: FrameOffset,
    imm: u64,
    op: F,
) {
    // SAFETY: `fp` is the current frame pointer and `src`/`dst` are
    // in-bounds 8-byte slots within that frame (enforced by the verifier).
    unsafe {
        let a = read_u64(fp, src);
        write_u64(fp, dst, op(a, imm));
    }
}

/// `dst <- op(src_slot, imm)` (fallible).
#[inline(always)]
unsafe fn checked_imm_op_u64<F: FnOnce(u64, u64) -> Option<u64>>(
    fp: *mut u8,
    dst: FrameOffset,
    src: FrameOffset,
    imm: u64,
    op: F,
) -> Option<()> {
    // SAFETY: `fp` is the current frame pointer and `src`/`dst` are
    // in-bounds 8-byte slots within that frame (enforced by the verifier).
    unsafe {
        let a = read_u64(fp, src);
        let v = op(a, imm)?;
        write_u64(fp, dst, v);
        Some(())
    }
}

/// `dst <- op(lhs_slot, rhs_slot_as_shift)`. The shift amount lives in
/// a 1-byte slot (Move bytecode invariant); only that byte is read.
/// Return `Err(shift)` if the shift amount is `>= 64`.
#[inline(always)]
unsafe fn shift_u64<F: FnOnce(u64, u64) -> u64>(
    fp: *mut u8,
    dst: FrameOffset,
    lhs: FrameOffset,
    rhs: FrameOffset,
    op: F,
) -> Result<(), u8> {
    // SAFETY: `fp` is the current frame pointer; `lhs`/`dst` are in-bounds
    // 8-byte slots and `rhs` is an in-bounds 1-byte slot within that frame
    // (enforced by the verifier).
    unsafe {
        let shift = read_u8(fp, rhs);
        if shift >= 64 {
            return Err(shift);
        }
        let v = read_u64(fp, lhs);
        write_u64(fp, dst, op(v, shift as u64));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Unspecialized integer op dispatchers
// ---------------------------------------------------------------------------

/// Read a `T`-sized value from `base + byte_offset`. Aligned access for
/// `T` whose alignment fits the VM's [`MAX_ALIGN`] cap, unaligned otherwise.
///
/// TODO(correctness): this reads with native endianness, but `StoreImm*` writes immediates
/// as little-endian bytes. Consistent on LE hosts (all current targets); force
/// LE here (`from_le`/`to_le`, no-op on LE) to be portable.
///
/// # Safety
/// `base.add(byte_offset)` must be valid for a read of `size_of::<T>()`
/// bytes, with the appropriate alignment when `align_of::<T>() <= MAX_ALIGN`.
#[inline(always)]
unsafe fn read_int<T: Copy>(base: *const u8, byte_offset: impl Into<usize>) -> T {
    let ptr = unsafe { base.add(byte_offset.into()) as *const T };
    unsafe {
        if std::mem::align_of::<T>() <= MAX_ALIGN {
            ptr.read()
        } else {
            ptr.read_unaligned()
        }
    }
}

/// Mirror of [`read_int`] for writes.
#[inline(always)]
unsafe fn write_int<T: Copy>(base: *mut u8, byte_offset: impl Into<usize>, val: T) {
    let ptr = unsafe { base.add(byte_offset.into()) as *mut T };
    unsafe {
        if std::mem::align_of::<T>() <= MAX_ALIGN {
            ptr.write(val)
        } else {
            ptr.write_unaligned(val)
        }
    }
}

/// [`U256`]'s `Shl`/`Shr` trait impls require `Self` as the rhs.
#[inline(always)]
fn u256_from_u8(x: u8) -> U256 {
    let mut bytes = [0u8; 32];
    bytes[0] = x;
    U256::from_le_bytes(bytes)
}

// Dispatch on an [`IntOperand`]: for each variant, invoke the caller's
// `$action!` macro with three arguments — `($rust_ty, $sign, $rhs_value)`.
// `$sign` is the literal token `unsigned` or `signed`, letting `$action!`
// match on it to specialize the body (e.g. bitwise ops reject signed at
// the language level). `$rhs_value` is an expression of type `$rust_ty`,
// already loaded for slot arms and inlined for imm arms.
//
// Example usage: see [`impl_int_arith!`] / [`impl_int_bitwise!`].
//
// Centralizing the 24-arm fanout in one place keeps each per-op
// dispatcher (`exec_int_add` etc.) at a single invocation.
macro_rules! dispatch_int_operand {
    ($fp:expr, $rhs:expr, $action:ident) => {
        match $rhs {
            IntOperand::SlotU8(off) => $action!(u8, unsigned, read_int::<u8>($fp, *off)),
            IntOperand::SlotU16(off) => $action!(u16, unsigned, read_int::<u16>($fp, *off)),
            IntOperand::SlotU32(off) => $action!(u32, unsigned, read_int::<u32>($fp, *off)),
            IntOperand::SlotU64(off) => $action!(u64, unsigned, read_int::<u64>($fp, *off)),
            IntOperand::SlotU128(off) => $action!(u128, unsigned, read_int::<u128>($fp, *off)),
            IntOperand::SlotU256(off) => $action!(U256, unsigned, read_int::<U256>($fp, *off)),
            IntOperand::SlotI8(off) => $action!(i8, signed, read_int::<i8>($fp, *off)),
            IntOperand::SlotI16(off) => $action!(i16, signed, read_int::<i16>($fp, *off)),
            IntOperand::SlotI32(off) => $action!(i32, signed, read_int::<i32>($fp, *off)),
            IntOperand::SlotI64(off) => $action!(i64, signed, read_int::<i64>($fp, *off)),
            IntOperand::SlotI128(off) => $action!(i128, signed, read_int::<i128>($fp, *off)),
            IntOperand::SlotI256(off) => $action!(I256, signed, read_int::<I256>($fp, *off)),
            IntOperand::ImmU8(v) => $action!(u8, unsigned, *v),
            IntOperand::ImmU16(v) => $action!(u16, unsigned, *v),
            IntOperand::ImmU32(v) => $action!(u32, unsigned, *v),
            IntOperand::ImmU64(v) => $action!(u64, unsigned, *v),
            IntOperand::ImmI8(v) => $action!(i8, signed, *v),
            IntOperand::ImmI16(v) => $action!(i16, signed, *v),
            IntOperand::ImmI32(v) => $action!(i32, signed, *v),
            IntOperand::ImmI64(v) => $action!(i64, signed, *v),
            IntOperand::ImmU128(b) => $action!(u128, unsigned, **b),
            IntOperand::ImmU256(b) => $action!(U256, unsigned, **b),
            IntOperand::ImmI128(b) => $action!(i128, signed, **b),
            IntOperand::ImmI256(b) => $action!(I256, signed, **b),
        }
    };
}

// Generates an `#[inline(never)]` arith dispatcher (`exec_int_add` etc.)
// from a function name, an error variant used when the checked op
// returns `None`, and the checked associated fn to call on the operand
// pair. Marking the dispatcher `#[inline(never)]` keeps the hot
// [`InterpreterContext::step`] loop compact: the per-op type fanout
// (12 widths × 2 operand kinds) lives in the out-of-line function and
// only inflates the i-cache for that op when it actually runs.
//
// Example usage:
//   impl_int_arith!(exec_int_add, ArithmeticUnderOverflow { op: ArithOp::Add }, checked_add);
#[rustfmt::skip]
macro_rules! impl_int_arith {
    ($fn_name:ident, $variant:ident $body:tt, $method:ident) => {
        /// # Safety
        /// `fp` is the current frame pointer; `op`'s slot offsets are
        /// in-bounds (enforced by the verifier).
        #[inline(never)]
        unsafe fn $fn_name(fp: *mut u8, op: &IntBinaryOp) -> VMResult<()> {
            unsafe {
                macro_rules! exec {
                    ($ty: ty,$_sign: tt,$rhs: expr) => {{
                        let lhs_val: $ty = read_int::<$ty>(fp, op.lhs);
                        let rhs_val: $ty = $rhs;
                        let result: $ty = <$ty>::$method(lhs_val, rhs_val)
                            .ok_or_else(|| VMInternalError::new(RuntimeError::$variant $body))?;
                        write_int::<$ty>(fp, op.dst, result);
                    }};
                }
                dispatch_int_operand!(fp, &op.rhs, exec);
                Ok(())
            }
        }
    };
}

// Each error message notes the abort condition. Signed arith can
// underflow on `Add` (e.g. `i8::MIN + (-1)`) as well as overflow on `Sub`,
// so both are reported as "under/overflow" to keep the message accurate
// for either.
impl_int_arith!(
    exec_int_add,
    ArithmeticUnderOverflow { op: ArithOp::Add },
    checked_add
);
impl_int_arith!(
    exec_int_sub,
    ArithmeticUnderOverflow { op: ArithOp::Sub },
    checked_sub
);
impl_int_arith!(
    exec_int_mul,
    ArithmeticUnderOverflow { op: ArithOp::Mul },
    checked_mul
);
impl_int_arith!(
    exec_int_div,
    DivisionByZeroOrOverflow { op: ArithOp::Div },
    checked_div
);
impl_int_arith!(
    exec_int_mod,
    DivisionByZeroOrOverflow { op: ArithOp::Mod },
    checked_rem
);

// Generates an `#[inline(never)]` bitwise dispatcher. Same shape as
// [`impl_int_arith!`] but uses an infix `$bop` (one of `&`, `|`, `^`) and
// rejects signed operands at the interpreter level — bitwise on signed
// integers is undefined in Move and would also fail to compile against
// [`I256`], which doesn't implement the Rust bit operators.
//
// `#[rustfmt::skip]`: nested `macro_rules! exec` uses literal
// `unsigned` / `signed` tokens that confuse rustfmt's indenter.
#[rustfmt::skip]
macro_rules! impl_int_bitwise {
    ($fn_name:ident, $base_op:expr, $bop:tt) => {
        /// # Safety
        /// See [`exec_int_add`].
        #[inline(never)]
        unsafe fn $fn_name(fp: *mut u8, op: &IntBinaryOp) -> VMResult<()> {
            unsafe {
                macro_rules! exec {
                    ($ty:ty, unsigned, $rhs:expr) => {{
                        let lhs_val: $ty = read_int::<$ty>(fp, op.lhs);
                        let rhs_val: $ty = $rhs;
                        let result: $ty = lhs_val $bop rhs_val;
                        write_int::<$ty>(fp, op.dst, result);
                    }};
                    ($ty:ty, signed, $rhs:expr) => {{
                        let _ = $rhs;
                        invariant_violation!(OperationNotSupportedForType {
                            op: $base_op,
                            signedness: Signedness::Signed,
                        });
                    }};
                }
                dispatch_int_operand!(fp, &op.rhs, exec);
                Ok(())
            }
        }
    };
}

impl_int_bitwise!(exec_int_bit_and, ArithOp::BitAnd, &);
impl_int_bitwise!(exec_int_bit_or, ArithOp::BitOr, |);
impl_int_bitwise!(exec_int_bit_xor, ArithOp::BitXor, ^);

// Dispatch on a shift op's lhs type. Centralizes the 12-arm fanout so
// `impl_int_shift!` can stay a one-invocation generator. Native widths
// shift by `u32`; [`U256`]'s `Shl`/`Shr` impls require `Self` for the rhs,
// so its arm passes `u256_from_u8($shift_amount)` instead. Signed arms
// fall through to a `signed` action arm so the caller can bail.
macro_rules! dispatch_shift_lhs_ty {
    ($ty:expr, $shift_amount:expr, $action:ident) => {
        match $ty {
            IntTy::U8 => $action!(u8, unsigned, $shift_amount as u32),
            IntTy::U16 => $action!(u16, unsigned, $shift_amount as u32),
            IntTy::U32 => $action!(u32, unsigned, $shift_amount as u32),
            IntTy::U64 => $action!(u64, unsigned, $shift_amount as u32),
            IntTy::U128 => $action!(u128, unsigned, $shift_amount as u32),
            IntTy::U256 => $action!(U256, unsigned, u256_from_u8($shift_amount)),
            IntTy::I8 => $action!(i8, signed, 0u32),
            IntTy::I16 => $action!(i16, signed, 0u32),
            IntTy::I32 => $action!(i32, signed, 0u32),
            IntTy::I64 => $action!(i64, signed, 0u32),
            IntTy::I128 => $action!(i128, signed, 0u32),
            IntTy::I256 => $action!(I256, signed, 0u32),
        }
    };
}

// Generates a shift dispatcher (`exec_int_shl` / `exec_int_shr`). Same
// shape as [`impl_int_arith!`] / [`impl_int_bitwise!`]: a nested `exec!`
// macro defines the per-type body once, and [`dispatch_shift_lhs_ty!`]
// fans it out over the 12 [`IntTy`] arms.
//
// The shift amount is always `u8` in Move and is range-checked here
// against `op.ty.bit_width()`. The `signed` arms raise an invariant
// violation; the verifier rejects them ahead of time.
// `#[rustfmt::skip]` on the outer macro: the nested `macro_rules! exec`
// uses literal `unsigned` / `signed` tokens in its arms, which confuses
// rustfmt into over-indenting every arm past the first. Skipping the
// whole macro keeps the body readable.
#[rustfmt::skip]
macro_rules! impl_int_shift {
    ($fn_name:ident, $base_op:expr, $bop:tt) => {
        /// # Safety
        /// See [`exec_int_add`].
        #[inline(never)]
        unsafe fn $fn_name(fp: *mut u8, op: &IntShiftOp) -> VMResult<()> {
            unsafe {
                let shift_amount: u8 = match &op.rhs {
                    ShiftOperand::SlotU8(off) => read_u8(fp, *off),
                    ShiftOperand::ImmU8(v) => *v,
                };
                let bit_width = op.ty.bit_width() as u32;
                if (shift_amount as u32) >= bit_width {
                    return Err(VMInternalError::new(RuntimeError::ShiftAmountOutOfRange {
                        op: $base_op,
                        ty: op.ty,
                        shift_amount,
                        bit_width,
                    }));
                }
                macro_rules! exec {
                    ($ty:ty, unsigned, $shift_val:expr) => {{
                        let lhs_val: $ty = read_int::<$ty>(fp, op.lhs);
                        let result: $ty = lhs_val $bop $shift_val;
                        write_int::<$ty>(fp, op.dst, result);
                    }};
                    ($_ty:ty, signed, $_shift_val:expr) => {{
                        invariant_violation!(OperationNotSupportedForType {
                            op: $base_op,
                            signedness: Signedness::Signed,
                        });
                    }};
                }
                dispatch_shift_lhs_ty!(op.ty, shift_amount, exec);
                Ok(())
            }
        }
    };
}

impl_int_shift!(exec_int_shl, ArithOp::Shl, <<);
impl_int_shift!(exec_int_shr, ArithOp::Shr, >>);

/// # Safety
/// See [`exec_int_add`].
#[inline(never)]
unsafe fn exec_int_negate(fp: *mut u8, op: &IntNegateOp) -> VMResult<()> {
    unsafe {
        macro_rules! exec {
            ($ty:ty) => {{
                let src_val: $ty = read_int::<$ty>(fp, op.src);
                let result: $ty = <$ty>::checked_neg(src_val).ok_or_else(|| {
                    VMInternalError::new(RuntimeError::NegateMinOverflow { ty: op.ty })
                })?;
                write_int::<$ty>(fp, op.dst, result);
            }};
        }
        match op.ty {
            IntTy::I8 => exec!(i8),
            IntTy::I16 => exec!(i16),
            IntTy::I32 => exec!(i32),
            IntTy::I64 => exec!(i64),
            IntTy::I128 => exec!(i128),
            IntTy::I256 => exec!(I256),
            IntTy::U8 | IntTy::U16 | IntTy::U32 | IntTy::U64 | IntTy::U128 | IntTy::U256 => {
                invariant_violation!(OperationNotSupportedForType {
                    op: ArithOp::Negate,
                    signedness: Signedness::Unsigned,
                });
            },
        }
        Ok(())
    }
}

// Helper macro to dispatch based on an [`IntTy`] tag.
macro_rules! dispatch_int_ty {
    ($ty:expr, $action:ident) => {
        match $ty {
            IntTy::U8 => $action!(u8),
            IntTy::U16 => $action!(u16),
            IntTy::U32 => $action!(u32),
            IntTy::U64 => $action!(u64),
            IntTy::U128 => $action!(u128),
            IntTy::U256 => $action!(U256),
            IntTy::I8 => $action!(i8),
            IntTy::I16 => $action!(i16),
            IntTy::I32 => $action!(i32),
            IntTy::I64 => $action!(i64),
            IntTy::I128 => $action!(i128),
            IntTy::I256 => $action!(I256),
        }
    };
}

/// Executes an `IntCast` operation. Handles all possible pairs of integer types.
///
/// # Safety
///
/// Same as [`exec_int_add`].
#[inline(never)]
unsafe fn exec_int_cast(fp: *mut u8, op: &IntCastOp) -> VMResult<()> {
    unsafe {
        macro_rules! cast_from {
            ($src_ty:ty) => {{
                let src_val: $src_ty = read_int::<$src_ty>(fp, op.src);
                macro_rules! cast_to {
                    ($dst_ty: ty) => {{
                        let result: $dst_ty = <$dst_ty>::try_from(src_val).map_err(|_| {
                            RuntimeError::CastOutOfRange {
                                from: op.from,
                                to: op.to,
                            }
                        })?;
                        write_int::<$dst_ty>(fp, op.dst, result);
                    }};
                }
                dispatch_int_ty!(op.to, cast_to);
            }};
        }
        dispatch_int_ty!(op.from, cast_from);
        Ok(())
    }
}

/// Reads `lhs` at `rhs`'s concrete type and returns `op(lhs, rhs)`; the
/// comparison is signed iff that type is signed.
///
/// # Safety
/// See [`exec_int_add`].
#[inline(never)]
unsafe fn int_cmp_bool(fp: *mut u8, lhs: FrameOffset, op: CmpKind, rhs: &IntOperand) -> bool {
    unsafe {
        macro_rules! exec {
            ($ty:ty, $_sign:tt, $rhs:expr) => {{
                let lhs_val: $ty = read_int::<$ty>(fp, lhs);
                let rhs_val: $ty = $rhs;
                match op {
                    CmpKind::Lt => lhs_val < rhs_val,
                    CmpKind::Le => lhs_val <= rhs_val,
                    CmpKind::Gt => lhs_val > rhs_val,
                    CmpKind::Ge => lhs_val >= rhs_val,
                    CmpKind::Eq => lhs_val == rhs_val,
                    CmpKind::Neq => lhs_val != rhs_val,
                }
            }};
        }
        dispatch_int_operand!(fp, rhs, exec)
    }
}

// ---------------------------------------------------------------------------
// Interpreter loop
// ---------------------------------------------------------------------------

impl InterpreterContext<'_> {
    /// Shared body of the conditional `Jump*` micro-ops: charge the chosen
    /// edge's cost, then jump to `target` or fall through to the next pc.
    #[inline(always)]
    fn cond_branch(
        &mut self,
        cond: bool,
        target: CodeOffset,
        gas_taken: u64,
        gas_fallthrough: u64,
        regs: &mut VMRegisters,
    ) -> VMResult<()> {
        if cond {
            self.gas_meter.charge(gas_taken)?;
            regs.pc = target.into();
        } else {
            self.gas_meter.charge(gas_fallthrough)?;
            regs.pc += 1;
        }
        Ok(())
    }

    pub fn run(&mut self) -> VMResult<RuntimeStatus> {
        // Hoist the VM registers into a local so the dispatch loop keeps
        // them in CPU registers rather than reloading from `self.registers`
        // each iteration. Only sync back to `self` on exit.
        let mut regs = self.registers;

        // Charge the entry function's entry block before any of its instructions run.
        let entry_gas = unsafe { regs.func.as_ref() }.entry_gas;
        self.gas_meter.charge(entry_gas)?;

        let outcome = loop {
            // SAFETY: Current function is always a valid, non-null pointer because
            // it is derived from function reference (e.g., entrypoint) or when
            // executing a call instruction, which stores a valid pointer.
            let func = unsafe { regs.func.as_ref() };

            let code = func.code.get();

            if regs.pc >= code.len() {
                invariant_violation!(PcOutOfBounds {
                    pc: regs.pc,
                    func_name: func.name().to_string(),
                    code_len: code.len(),
                });
            }

            let fp = regs.fp;
            let instr = &code[regs.pc];

            // SAFETY: fp points into the interpreter's linear stack; all byte
            // offsets are within the current frame (enforced by the bytecode
            // compiler). Heap pointers read from the frame are kept valid by GC.
            unsafe {
                // TODO(perf): explore faster dispatch -- super-instructions (fuse
                // common sequences like load+compare+branch), threaded/computed-goto
                // dispatch, and copy-and-patch JIT.
                match *instr {
                    // ----- Control flow (set pc explicitly, return early) -----
                    MicroOp::CallIndirect {
                        module_id,
                        func_name,
                        ty_args,
                    } => {
                        // TODO(perf): full flow should be like this:
                        //
                        //   1. IC lookup:
                        //      - Hit:  return pointer,
                        //      - Miss: goto 2.
                        //   2. target = load_function(...)
                        //   3. IC insert target
                        //   4. Patching:
                        //      If can patch caller, try it.
                        let target = self.load_function(module_id, func_name, ty_args)?;
                        // SAFETY: `target` points to a `Function`, which is not reclaimed during
                        // execution as guaranteed by the execution guard.
                        self.call(func, &mut regs, target.as_ref_unchecked())?;
                        continue;
                    },
                    MicroOp::CallDirect { ptr } => {
                        self.call(func, &mut regs, ptr.as_ref_unchecked())?;
                        continue;
                    },

                    MicroOp::CallNative {
                        native_idx,
                        ty_args,
                        ref abi,
                    } => {
                        // On abort, halt; otherwise native success falls through to
                        // the common tail, which advances the pc by one.
                        if let Some((code, message)) =
                            self.exec_call_native(func, regs, native_idx, ty_args, abi)?
                        {
                            break RuntimeStatus::Aborted { code, message };
                        }
                    },

                    MicroOp::JumpNotZeroU64 {
                        target,
                        src,
                        gas_taken,
                        gas_fallthrough,
                    } => {
                        self.cond_branch(
                            read_u64(fp, src) != 0,
                            target,
                            gas_taken,
                            gas_fallthrough,
                            &mut regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpNotZeroByte {
                        target,
                        src,
                        gas_taken,
                        gas_fallthrough,
                    } => {
                        // Read as `u8` only to test against zero; the byte's sign is
                        // irrelevant.
                        self.cond_branch(
                            read_u8(fp, src) != 0,
                            target,
                            gas_taken,
                            gas_fallthrough,
                            &mut regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpZeroByte {
                        target,
                        src,
                        gas_taken,
                        gas_fallthrough,
                    } => {
                        // Read as `u8` only to test against zero; the byte's sign is
                        // irrelevant.
                        self.cond_branch(
                            read_u8(fp, src) == 0,
                            target,
                            gas_taken,
                            gas_fallthrough,
                            &mut regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpIntCmp(ref op) => {
                        self.cond_branch(
                            int_cmp_bool(fp, op.lhs, op.op, &op.rhs),
                            op.target,
                            op.gas_taken,
                            op.gas_fallthrough,
                            &mut regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpValueCmp(ref op) => {
                        // Operands are the aggregate values at their slots; a
                        // vector slot holds a pointer read through to its heap data.
                        let a = fp.add(op.lhs.into());
                        let b = fp.add(op.rhs.into());
                        let eq = value_utils::equals(self.loader.guard(), a, b, op.ty)?;
                        self.cond_branch(
                            eq ^ op.negate,
                            op.target,
                            op.gas_taken,
                            op.gas_fallthrough,
                            &mut regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpValueRefCmp(ref op) => {
                        // Operands are references; read through the fat pointers to
                        // obtain the operand data pointers.
                        let (lb, lo) = read_fat_ptr(fp, op.lhs);
                        let (rb, ro) = read_fat_ptr(fp, op.rhs);
                        let eq = value_utils::equals(
                            self.loader.guard(),
                            lb.add(lo as usize),
                            rb.add(ro as usize),
                            op.ty,
                        )?;
                        self.cond_branch(
                            eq ^ op.negate,
                            op.target,
                            op.gas_taken,
                            op.gas_fallthrough,
                            &mut regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpGreaterEqualU64Imm {
                        target,
                        src,
                        imm,
                        gas_taken,
                        gas_fallthrough,
                    } => {
                        self.cond_branch(
                            read_u64(fp, src) >= imm,
                            target,
                            gas_taken,
                            gas_fallthrough,
                            &mut regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpLessU64Imm {
                        target,
                        src,
                        imm,
                        gas_taken,
                        gas_fallthrough,
                    } => {
                        self.cond_branch(
                            read_u64(fp, src) < imm,
                            target,
                            gas_taken,
                            gas_fallthrough,
                            &mut regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpGreaterU64Imm {
                        target,
                        src,
                        imm,
                        gas_taken,
                        gas_fallthrough,
                    } => {
                        self.cond_branch(
                            read_u64(fp, src) > imm,
                            target,
                            gas_taken,
                            gas_fallthrough,
                            &mut regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpLessEqualU64Imm {
                        target,
                        src,
                        imm,
                        gas_taken,
                        gas_fallthrough,
                    } => {
                        self.cond_branch(
                            read_u64(fp, src) <= imm,
                            target,
                            gas_taken,
                            gas_fallthrough,
                            &mut regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpLessU64 {
                        target,
                        lhs,
                        rhs,
                        gas_taken,
                        gas_fallthrough,
                    } => {
                        self.cond_branch(
                            read_u64(fp, lhs) < read_u64(fp, rhs),
                            target,
                            gas_taken,
                            gas_fallthrough,
                            &mut regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpGreaterEqualU64 {
                        target,
                        lhs,
                        rhs,
                        gas_taken,
                        gas_fallthrough,
                    } => {
                        self.cond_branch(
                            read_u64(fp, lhs) >= read_u64(fp, rhs),
                            target,
                            gas_taken,
                            gas_fallthrough,
                            &mut regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpNotEqualU64 {
                        target,
                        lhs,
                        rhs,
                        gas_taken,
                        gas_fallthrough,
                    } => {
                        self.cond_branch(
                            read_u64(fp, lhs) != read_u64(fp, rhs),
                            target,
                            gas_taken,
                            gas_fallthrough,
                            &mut regs,
                        )?;
                        continue;
                    },

                    MicroOp::Jump { target, gas } => {
                        self.gas_meter.charge(gas)?;
                        regs.pc = target.into();
                        continue;
                    },

                    MicroOp::Return => {
                        let meta = fp.sub(FRAME_METADATA_SIZE);

                        let saved_func_ptr =
                            read_ptr(meta, META_SAVED_FUNC_PTR_OFFSET) as *const Function;
                        if saved_func_ptr.is_null() {
                            break RuntimeStatus::Success;
                        }
                        // SAFETY: We have just checked that the saved function
                        // pointer is non-null.
                        regs.func = NonNull::new_unchecked(saved_func_ptr as *mut Function);

                        regs.pc = read_u64(meta, META_SAVED_PC_OFFSET) as usize;
                        regs.fp = read_ptr(meta, META_SAVED_FP_OFFSET);
                        continue;
                    },

                    MicroOp::Abort { code } => {
                        let code = read_u64(fp, code);
                        break RuntimeStatus::Aborted {
                            code,
                            message: None,
                        };
                    },

                    MicroOp::AbortMsg { code, message } => {
                        let code = read_u64(fp, code);
                        let vec_ptr = read_ptr(fp, message);
                        let len = read_vec_len(vec_ptr) as usize;
                        let message = if len == 0 {
                            String::new()
                        } else {
                            // TODO(metering): charge gas for abort message bytes.
                            if len > ABORT_MESSAGE_SIZE_LIMIT {
                                return Err(VMInternalError::new(
                                    RuntimeError::AbortMessageTooLong {
                                        len,
                                        max: ABORT_MESSAGE_SIZE_LIMIT,
                                    },
                                ));
                            }
                            // SAFETY: `vec_ptr` is non-null for non-zero lengths
                            // and points at a heap vector with `len` initialized
                            // bytes at `VEC_DATA_OFFSET`.
                            let data = vec_ptr.add(VEC_DATA_OFFSET);
                            String::from_utf8(std::slice::from_raw_parts(data, len).to_vec())
                                .map_err(|_| RuntimeError::InvalidAbortMessage)?
                        };
                        break RuntimeStatus::Aborted {
                            code,
                            message: Some(message),
                        };
                    },

                    // ----- Arithmetic -----
                    MicroOp::StoreImm1 { dst, imm } => write_u8(fp, dst, imm),
                    MicroOp::StoreImm2 { dst, ref imm } => write_int::<[u8; 2]>(fp, dst, *imm),
                    MicroOp::StoreImm4 { dst, ref imm } => write_int::<[u8; 4]>(fp, dst, *imm),
                    MicroOp::StoreImm8 { dst, ref imm } => write_int::<[u8; 8]>(fp, dst, *imm),
                    MicroOp::StoreImm16 { dst, ref imm } => write_int::<[u8; 16]>(fp, dst, **imm),
                    MicroOp::StoreImm32 { dst, ref imm } => write_int::<[u8; 32]>(fp, dst, **imm),
                    MicroOp::StoreImmVec { dst, idx } => {
                        self.exec_store_imm_vec(regs, dst, idx)?;
                    },

                    // Add
                    MicroOp::AddU64 { dst, lhs, rhs } => {
                        checked_binop_u64(fp, dst, lhs, rhs, u64::checked_add).ok_or(
                            RuntimeError::ArithmeticOverflow {
                                op: ArithOp::Add,
                                ty: IntTy::U64,
                            },
                        )?
                    },
                    MicroOp::AddU64Imm { dst, src, imm } => {
                        checked_imm_op_u64(fp, dst, src, imm, u64::checked_add).ok_or(
                            RuntimeError::ArithmeticOverflow {
                                op: ArithOp::Add,
                                ty: IntTy::U64,
                            },
                        )?
                    },

                    // Sub
                    MicroOp::SubU64 { dst, lhs, rhs } => {
                        checked_binop_u64(fp, dst, lhs, rhs, u64::checked_sub).ok_or(
                            RuntimeError::ArithmeticUnderflow {
                                op: ArithOp::Sub,
                                ty: IntTy::U64,
                            },
                        )?
                    },
                    MicroOp::SubU64Imm { dst, src, imm } => {
                        checked_imm_op_u64(fp, dst, src, imm, u64::checked_sub).ok_or(
                            RuntimeError::ArithmeticUnderflow {
                                op: ArithOp::Sub,
                                ty: IntTy::U64,
                            },
                        )?
                    },
                    // dst = imm - src, so flip the operand order.
                    MicroOp::RSubU64Imm { dst, src, imm } => {
                        checked_imm_op_u64(fp, dst, src, imm, |s, i| u64::checked_sub(i, s))
                            .ok_or_else(|| {
                                VMInternalError::new(RuntimeError::ArithmeticUnderflow {
                                    op: ArithOp::Sub,
                                    ty: IntTy::U64,
                                })
                            })?
                    },

                    // Mul
                    MicroOp::MulU64 { dst, lhs, rhs } => {
                        checked_binop_u64(fp, dst, lhs, rhs, u64::checked_mul).ok_or(
                            RuntimeError::ArithmeticOverflow {
                                op: ArithOp::Mul,
                                ty: IntTy::U64,
                            },
                        )?
                    },
                    MicroOp::MulU64Imm { dst, src, imm } => {
                        checked_imm_op_u64(fp, dst, src, imm, u64::checked_mul).ok_or(
                            RuntimeError::ArithmeticOverflow {
                                op: ArithOp::Mul,
                                ty: IntTy::U64,
                            },
                        )?
                    },

                    // Div / Mod
                    MicroOp::DivU64 { dst, lhs, rhs } => {
                        checked_binop_u64(fp, dst, lhs, rhs, u64::checked_div).ok_or(
                            RuntimeError::DivisionByZero {
                                op: ArithOp::Div,
                                ty: IntTy::U64,
                            },
                        )?
                    },
                    // INVARIANT: the verifier rejects `imm == 0`, so plain `s / imm`
                    // cannot trigger Rust's div-by-zero panic. Asserted below in
                    // debug builds as a defensive check.
                    MicroOp::DivU64Imm { dst, src, imm } => {
                        debug_assert!(
                            imm != 0,
                            "DivU64Imm: imm must be non-zero (verifier invariant)"
                        );
                        imm_op_u64(fp, dst, src, imm, |s, i| s / i)
                    },
                    MicroOp::ModU64 { dst, lhs, rhs } => {
                        checked_binop_u64(fp, dst, lhs, rhs, u64::checked_rem).ok_or(
                            RuntimeError::DivisionByZero {
                                op: ArithOp::Mod,
                                ty: IntTy::U64,
                            },
                        )?
                    },
                    // INVARIANT: the verifier rejects `imm == 0`, so plain `s % imm`
                    // cannot trigger Rust's div-by-zero panic. Asserted below in
                    // debug builds as a defensive check.
                    MicroOp::ModU64Imm { dst, src, imm } => {
                        debug_assert!(
                            imm != 0,
                            "ModU64Imm: imm must be non-zero (verifier invariant)"
                        );
                        imm_op_u64(fp, dst, src, imm, |s, i| s % i)
                    },

                    // Bitwise (infallible)
                    MicroOp::BitAndU64 { dst, lhs, rhs } => {
                        binop_u64(fp, dst, lhs, rhs, |a, b| a & b)
                    },
                    MicroOp::BitOrU64 { dst, lhs, rhs } => {
                        binop_u64(fp, dst, lhs, rhs, |a, b| a | b)
                    },
                    MicroOp::BitXorU64 { dst, lhs, rhs } => {
                        binop_u64(fp, dst, lhs, rhs, |a, b| a ^ b)
                    },

                    // Shifts
                    MicroOp::ShlU64 { dst, lhs, rhs } => shift_u64(fp, dst, lhs, rhs, |v, s| {
                        v << s
                    })
                    .map_err(|shift_amount| RuntimeError::ShiftAmountOutOfRange {
                        op: ArithOp::Shl,
                        ty: IntTy::U64,
                        shift_amount,
                        bit_width: 64,
                    })?,
                    // INVARIANT: the verifier rejects `imm >= 64`, so plain `s << imm`
                    // cannot wrap or trigger UB. Asserted below in debug builds as a
                    // defensive check.
                    MicroOp::ShlU64Imm { dst, src, imm } => {
                        debug_assert!(imm < 64, "ShlU64Imm: imm must be < 64 (verifier invariant)");
                        imm_op_u64(fp, dst, src, imm as u64, |s, i| s << i)
                    },
                    MicroOp::ShrU64 { dst, lhs, rhs } => shift_u64(fp, dst, lhs, rhs, |v, s| {
                        v >> s
                    })
                    .map_err(|shift_amount| RuntimeError::ShiftAmountOutOfRange {
                        op: ArithOp::Shr,
                        ty: IntTy::U64,
                        shift_amount,
                        bit_width: 64,
                    })?,
                    // INVARIANT: the verifier rejects `imm >= 64`, so plain `s >> imm`
                    // cannot wrap or trigger UB. Asserted below in debug builds as a
                    // defensive check.
                    MicroOp::ShrU64Imm { dst, src, imm } => {
                        debug_assert!(imm < 64, "ShrU64Imm: imm must be < 64 (verifier invariant)");
                        imm_op_u64(fp, dst, src, imm as u64, |s, i| s >> i)
                    },

                    MicroOp::StoreRandomU64 { dst } => {
                        let val: u64 = self.rng.r#gen();
                        write_u64(fp, dst, val);
                    },

                    MicroOp::ForceGC => {
                        gc_collect!(self, regs.fp, regs.pc, regs.func)?;
                    },

                    MicroOp::Move8 { dst, src } => {
                        let v = read_u64(fp, src);
                        write_u64(fp, dst, v);
                    },

                    MicroOp::Move { dst, src, size } => {
                        // TODO(perf): consider adding a provably non-overlapping variant of this op.
                        // Overlap-safe `copy`: `dst` and `src` may partially overlap.
                        // E.g. the return-value shuffle may move results in the same home region.
                        std::ptr::copy(fp.add(src.into()), fp.add(dst.into()), size as usize);
                    },

                    // ----- Vector instructions -----
                    MicroOp::VecNew { dst } => {
                        write_ptr(fp, dst, std::ptr::null());
                    },

                    MicroOp::VecLen { dst, vec_ref } => {
                        let (ref_base, ref_off) = read_fat_ptr(fp, vec_ref);
                        let vec_ptr = read_ptr(ref_base, ref_off as usize);
                        let len = read_vec_len(vec_ptr);
                        write_u64(fp, dst, len);
                    },

                    MicroOp::VecPushBack {
                        vec_ref,
                        elem,
                        elem_size,
                        descriptor_id,
                    } => {
                        let (ref_base, ref_off) = read_fat_ptr(fp, vec_ref);
                        let mut vec_ptr = read_ptr(ref_base, ref_off as usize);

                        if vec_ptr.is_null() {
                            vec_ptr = alloc_vec!(
                                self,
                                fp,
                                regs.pc,
                                regs.func,
                                descriptor_id,
                                elem_size,
                                VEC_PUSHBACK_INIT_CAPACITY
                            )?;
                            // Re-read base after potential GC.
                            let (ref_base, ref_off) = read_fat_ptr(fp, vec_ref);
                            write_ptr(ref_base, ref_off as usize, vec_ptr);
                        }

                        let len = read_vec_len(vec_ptr);
                        let total = read_obj_size(vec_ptr) as usize;
                        let cap_in_elems = ((total - OBJECT_HEADER_SIZE - VEC_DATA_OFFSET)
                            / elem_size as usize) as u64;

                        if len >= cap_in_elems {
                            vec_ptr = grow_vec_ref!(
                                self,
                                fp,
                                regs.pc,
                                regs.func,
                                vec_ref.into(),
                                elem_size,
                                len + 1
                            )?;
                        }

                        std::ptr::copy_nonoverlapping(
                            fp.add(elem.into()),
                            vec_elem_ptr(vec_ptr, len, elem_size) as *mut u8,
                            elem_size as usize,
                        );
                        write_u64(vec_ptr, VEC_LENGTH_OFFSET, len + 1);
                    },

                    MicroOp::VecPopBack {
                        dst,
                        vec_ref,
                        elem_size,
                    } => {
                        let (ref_base, ref_off) = read_fat_ptr(fp, vec_ref);
                        let vec_ptr = read_ptr(ref_base, ref_off as usize);
                        let len = read_vec_len(vec_ptr);
                        if len == 0 {
                            return Err(VMInternalError::new(RuntimeError::PopFromEmptyVector));
                        }
                        let new_len = len - 1;
                        std::ptr::copy_nonoverlapping(
                            vec_elem_ptr(vec_ptr, new_len, elem_size),
                            fp.add(dst.into()),
                            elem_size as usize,
                        );
                        write_u64(vec_ptr, VEC_LENGTH_OFFSET, new_len);
                    },

                    MicroOp::VecLoadElem {
                        dst,
                        vec_ref,
                        idx,
                        elem_size,
                    } => {
                        let (ref_base, ref_off) = read_fat_ptr(fp, vec_ref);
                        let vec_ptr = read_ptr(ref_base, ref_off as usize);
                        let idx = read_u64(fp, idx);
                        let len = read_vec_len(vec_ptr);
                        if idx >= len {
                            return Err(VMInternalError::new(
                                RuntimeError::VectorIndexOutOfBounds {
                                    op: VecOp::LoadElem,
                                    idx,
                                    len,
                                },
                            ));
                        }
                        std::ptr::copy_nonoverlapping(
                            vec_elem_ptr(vec_ptr, idx, elem_size),
                            fp.add(dst.into()),
                            elem_size as usize,
                        );
                    },

                    MicroOp::VecStoreElem {
                        vec_ref,
                        idx,
                        src,
                        elem_size,
                    } => {
                        let (ref_base, ref_off) = read_fat_ptr(fp, vec_ref);
                        let vec_ptr = read_ptr(ref_base, ref_off as usize);
                        let idx = read_u64(fp, idx);
                        let len = read_vec_len(vec_ptr);
                        if idx >= len {
                            return Err(VMInternalError::new(
                                RuntimeError::VectorIndexOutOfBounds {
                                    op: VecOp::StoreElem,
                                    idx,
                                    len,
                                },
                            ));
                        }
                        std::ptr::copy_nonoverlapping(
                            fp.add(src.into()),
                            vec_elem_ptr(vec_ptr, idx, elem_size) as *mut u8,
                            elem_size as usize,
                        );
                    },

                    MicroOp::VecPack(ref op) => {
                        self.exec_vec_pack(regs, op)?;
                    },

                    MicroOp::VecUnpack(ref op) => {
                        self.exec_vec_unpack(fp, op)?;
                    },

                    MicroOp::VecSwap {
                        vec_ref,
                        idx_a,
                        idx_b,
                        elem_size,
                    } => {
                        let (ref_base, ref_off) = read_fat_ptr(fp, vec_ref);
                        let vec_ptr = read_ptr(ref_base, ref_off as usize);
                        let idx_a = read_u64(fp, idx_a);
                        let idx_b = read_u64(fp, idx_b);
                        let len = read_vec_len(vec_ptr);
                        // Indices are checked before the equal-indices no-op.
                        for idx in [idx_a, idx_b] {
                            if idx >= len {
                                return Err(VMInternalError::new(
                                    RuntimeError::VectorIndexOutOfBounds {
                                        op: VecOp::Swap,
                                        idx,
                                        len,
                                    },
                                ));
                            }
                        }
                        // Equal pointers are UB for `swap_nonoverlapping`; distinct
                        // in-bounds indices guarantee disjoint element ranges.
                        if idx_a != idx_b {
                            std::ptr::swap_nonoverlapping(
                                vec_elem_ptr(vec_ptr, idx_a, elem_size) as *mut u8,
                                vec_elem_ptr(vec_ptr, idx_b, elem_size) as *mut u8,
                                elem_size as usize,
                            );
                        }
                    },

                    // ----- Reference (fat pointer) instructions -----
                    MicroOp::VecBorrow {
                        dst,
                        vec_ref,
                        idx,
                        elem_size,
                    } => {
                        let (ref_base, ref_off) = read_fat_ptr(fp, vec_ref);
                        let vec_ptr = read_ptr(ref_base, ref_off as usize);
                        let idx = read_u64(fp, idx);
                        let len = read_vec_len(vec_ptr);
                        if idx >= len {
                            return Err(VMInternalError::new(
                                RuntimeError::VectorIndexOutOfBounds {
                                    op: VecOp::Borrow,
                                    idx,
                                    len,
                                },
                            ));
                        }
                        let offset = VEC_DATA_OFFSET as u64 + idx * elem_size as u64;
                        write_fat_ptr(fp, dst, vec_ptr, offset);
                    },

                    MicroOp::SlotBorrow { dst, local } => {
                        write_fat_ptr(fp, dst, fp.add(local.into()), 0);
                    },

                    MicroOp::ReadRef { dst, ref_ptr, size } => {
                        let (base, offset) = read_fat_ptr(fp, ref_ptr);
                        let target = base.add(offset as usize);
                        // Overlap-safe `copy`: `dst` and `*ref` may alias.
                        std::ptr::copy(target, fp.add(dst.into()), size as usize);
                    },

                    MicroOp::WriteRef { ref_ptr, src, size } => {
                        let (base, offset) = read_fat_ptr(fp, ref_ptr);
                        let target = base.add(offset as usize);
                        // Overlap-safe `copy`: `src` and `*ref` may alias.
                        std::ptr::copy(fp.add(src.into()), target, size as usize);
                    },

                    MicroOp::DeriveRefOffsetImm {
                        dst_ref,
                        src_ref,
                        offset,
                    } => {
                        let (base, src_off) = read_fat_ptr(fp, src_ref);
                        write_fat_ptr(fp, dst_ref, base, src_off + offset as u64);
                    },

                    MicroOp::ReadRefOffset {
                        dst,
                        ref_ptr,
                        offset,
                        size,
                    } => {
                        let (base, ref_off) = read_fat_ptr(fp, ref_ptr);
                        let target = base.add(ref_off as usize + offset as usize);
                        // Overlap-safe `copy`: `dst` and `*ref` may alias.
                        std::ptr::copy(target, fp.add(dst.into()), size as usize);
                    },

                    MicroOp::WriteRefOffset {
                        ref_ptr,
                        offset,
                        src,
                        size,
                    } => {
                        let (base, ref_off) = read_fat_ptr(fp, ref_ptr);
                        let target = base.add(ref_off as usize + offset as usize);
                        // Overlap-safe `copy`: `src` and `*ref` may alias.
                        std::ptr::copy(fp.add(src.into()), target, size as usize);
                    },

                    // ----- Heap object instructions (structs and enums) -----
                    MicroOp::HeapNew { dst, descriptor_id } => {
                        let ptr = alloc_obj!(self, fp, regs.pc, regs.func, descriptor_id)?;
                        write_ptr(fp, dst, ptr);
                    },

                    MicroOp::HeapMoveFrom8 {
                        dst,
                        heap_ptr,
                        offset,
                    } => {
                        let obj_ptr = read_ptr(fp, heap_ptr);
                        let val = read_u64(obj_ptr, offset as usize);
                        write_u64(fp, dst, val);
                    },

                    MicroOp::HeapMoveFrom {
                        dst,
                        heap_ptr,
                        offset,
                        size,
                    } => {
                        let obj_ptr = read_ptr(fp, heap_ptr);
                        std::ptr::copy_nonoverlapping(
                            obj_ptr.add(offset as usize),
                            fp.add(dst.into()),
                            size as usize,
                        );
                    },

                    MicroOp::HeapMoveTo8 {
                        heap_ptr,
                        offset,
                        src,
                    } => {
                        let obj_ptr = read_ptr(fp, heap_ptr);
                        let val = read_u64(fp, src);
                        write_u64(obj_ptr, offset as usize, val);
                    },

                    MicroOp::HeapMoveToImm8 {
                        heap_ptr,
                        offset,
                        imm,
                    } => {
                        let obj_ptr = read_ptr(fp, heap_ptr);
                        write_u64(obj_ptr, offset as usize, imm);
                    },

                    MicroOp::HeapMoveTo {
                        heap_ptr,
                        offset,
                        src,
                        size,
                    } => {
                        let obj_ptr = read_ptr(fp, heap_ptr);
                        std::ptr::copy_nonoverlapping(
                            fp.add(src.into()),
                            obj_ptr.add(offset as usize),
                            size as usize,
                        );
                    },

                    MicroOp::HeapBorrow {
                        dst,
                        obj_ref,
                        offset,
                    } => {
                        let (ref_base, ref_off) = read_fat_ptr(fp, obj_ref);
                        let obj_ptr = read_ptr(ref_base, ref_off as usize);
                        // Unlike vectors, heap objects are never null — they
                        // are always allocated by HeapNew before being borrowed.
                        debug_assert!(!obj_ptr.is_null(), "HeapBorrow: null object pointer");
                        write_fat_ptr(fp, dst, obj_ptr, offset as u64);
                    },

                    MicroOp::PackClosure(ref op) => {
                        self.exec_pack_closure(regs, op)?;
                    },
                    MicroOp::CallClosure(ref op) => {
                        regs = self.exec_call_closure(func, regs, op)?;
                        continue;
                    },

                    MicroOp::IntAdd(ref op) => exec_int_add(fp, op)?,
                    MicroOp::IntSub(ref op) => exec_int_sub(fp, op)?,
                    MicroOp::IntMul(ref op) => exec_int_mul(fp, op)?,
                    MicroOp::IntDiv(ref op) => exec_int_div(fp, op)?,
                    MicroOp::IntMod(ref op) => exec_int_mod(fp, op)?,
                    MicroOp::IntBitAnd(ref op) => exec_int_bit_and(fp, op)?,
                    MicroOp::IntBitOr(ref op) => exec_int_bit_or(fp, op)?,
                    MicroOp::IntBitXor(ref op) => exec_int_bit_xor(fp, op)?,
                    MicroOp::IntShl(ref op) => exec_int_shl(fp, op)?,
                    MicroOp::IntShr(ref op) => exec_int_shr(fp, op)?,
                    MicroOp::IntNegate(ref op) => exec_int_negate(fp, op)?,
                    MicroOp::IntCast(ref op) => exec_int_cast(fp, op)?,

                    MicroOp::Exists { addr, ty, dst } => {
                        let address = read_account_address(fp, addr);
                        let exists = self.read_write_set.exists(
                            self.resource_provider,
                            &InMemoryStorageKey::resource(address, ty),
                        )?;
                        write_bool(fp, dst, exists);
                    },

                    MicroOp::BorrowGlobal { addr, ty, dst } => {
                        let address = read_account_address(fp, addr);
                        let ptr = self.read_write_set.borrow_global(
                            self.resource_provider,
                            &InMemoryStorageKey::resource(address, ty),
                        )?;
                        // A reference is a 16-byte fat pointer; the borrow points
                        // at the start of the resource, so the offset half is 0.
                        write_fat_ptr(fp, dst, ptr.as_ptr(), 0);
                    },

                    MicroOp::BorrowGlobalMut { addr, ty, dst } => {
                        let address = read_account_address(fp, addr);
                        let key = InMemoryStorageKey::resource(address, ty);
                        let ptr = match self
                            .read_write_set
                            .try_borrow_global_mut(self.resource_provider, &key)?
                        {
                            EntryPtr::Writable(ptr) => ptr,
                            EntryPtr::NonWritable(ptr) => {
                                let ptr = self.deep_copy(regs, ptr)?;
                                self.read_write_set.commit_borrow_global_mut(&key, ptr);
                                ptr
                            },
                        };
                        // A reference is a 16-byte fat pointer; the borrow points
                        // at the start of the resource, so the offset half is 0.
                        write_fat_ptr(fp, dst, ptr.as_ptr(), 0);
                    },

                    MicroOp::MoveFrom { addr, ty, dst } => {
                        let address = read_account_address(fp, addr);
                        let key = InMemoryStorageKey::resource(address, ty);
                        let entry_ptr = self
                            .read_write_set
                            .try_move_from(self.resource_provider, &key)?;
                        let ptr = match entry_ptr {
                            EntryPtr::Writable(ptr) => ptr,
                            EntryPtr::NonWritable(ptr) => {
                                let ptr = self.deep_copy(regs, ptr)?;
                                self.read_write_set.commit_move_from(&key);
                                ptr
                            },
                        };
                        write_ptr(fp, dst, ptr.as_ptr());
                    },

                    MicroOp::MoveTo {
                        signer_ref,
                        ty,
                        src,
                    } => {
                        // Dereference the `&signer` to obtain the 32-byte publishing address
                        let (base, offset) = read_fat_ptr(fp, signer_ref);
                        let address = read_account_address(base, offset as usize);
                        let Some(ptr) = NonNull::new(read_ptr(fp, src)) else {
                            invariant_violation!(MoveToNullSource);
                        };

                        self.read_write_set.move_to(
                            self.resource_provider,
                            &InMemoryStorageKey::resource(address, ty),
                            ptr,
                        )?;
                    },
                    MicroOp::IntCmp(ref op) => {
                        let result = int_cmp_bool(fp, op.lhs, op.op, &op.rhs);
                        write_u8(fp, op.dst, result as u8);
                    },
                    MicroOp::ValueCmp(ref op) => {
                        // Operands are the aggregate values at their slots; a
                        // vector slot holds a pointer read through to its heap data.
                        let a = fp.add(op.lhs.into());
                        let b = fp.add(op.rhs.into());
                        let eq = value_utils::equals(self.loader.guard(), a, b, op.ty)?;
                        write_bool(fp, op.dst, eq ^ op.negate);
                    },
                    MicroOp::ValueRefCmp(ref op) => {
                        // Operands are references; read through the fat pointers to
                        // obtain the operand data pointers.
                        let (lb, lo) = read_fat_ptr(fp, op.lhs);
                        let (rb, ro) = read_fat_ptr(fp, op.rhs);
                        let eq = value_utils::equals(
                            self.loader.guard(),
                            lb.add(lo as usize),
                            rb.add(ro as usize),
                            op.ty,
                        )?;
                        write_bool(fp, op.dst, eq ^ op.negate);
                    },
                    MicroOp::BoolNot { dst, src } => write_bool(fp, dst, !read_bool(fp, src)),
                    MicroOp::BoolAnd { dst, lhs, rhs } => {
                        let left = read_bool(fp, lhs);
                        let right = read_bool(fp, rhs);
                        write_bool(fp, dst, left && right)
                    },
                    MicroOp::BoolOr { dst, lhs, rhs } => {
                        let left = read_bool(fp, lhs);
                        let right = read_bool(fp, rhs);
                        write_bool(fp, dst, left || right)
                    },
                    MicroOp::EnumTestTag {
                        dst,
                        enum_ref,
                        variant,
                    } => {
                        // Deref the enum fat pointer to the heap object, then read the tag.
                        let (ref_base, ref_off) = read_fat_ptr(fp, enum_ref);
                        let obj_ptr = read_ptr(ref_base, ref_off as usize);
                        let tag = read_enum_tag(obj_ptr);
                        write_bool(fp, dst, tag == variant);
                    },

                    MicroOp::EnumBorrowVariantField {
                        dst,
                        enum_ref,
                        ref offsets,
                    } => {
                        // Deref the enum fat pointer to the heap object, read the
                        // tag, then borrow the field at that variant's offset.
                        let (ref_base, ref_off) = read_fat_ptr(fp, enum_ref);
                        let obj_ptr = read_ptr(ref_base, ref_off as usize);
                        let tag = read_enum_tag(obj_ptr);
                        let Some(variant_offset) = offsets.get(tag as usize) else {
                            // A tag past the variant table is heap corruption (a
                            // well-formed enum's tag is always in range), not a
                            // user-level mismatch. Surface it as an invariant
                            // violation.
                            invariant_violation!(EnumTagOutOfRange {
                                tag,
                                variant_count: offsets.len(),
                            });
                        };
                        match variant_offset {
                            Some(offset) => write_fat_ptr(fp, dst, obj_ptr, *offset as u64),
                            // Tag in range but this variant does not declare the
                            // field (move semantics for this is a runtime error).
                            None => {
                                return Err(VMInternalError::new(
                                    RuntimeError::EnumVariantMismatch { tag },
                                ))
                            },
                        }
                    },

                    MicroOp::EnumCheckVariant { enum_ptr, variant } => {
                        // `enum_ptr` is the heap-pointer value; runtime error if
                        // its tag is not the expected variant.
                        let obj_ptr = read_ptr(fp, enum_ptr);
                        let tag = read_enum_tag(obj_ptr);
                        if tag != variant {
                            return Err(VMInternalError::new(RuntimeError::EnumVariantMismatch {
                                tag,
                            }));
                        }
                    },

                    MicroOp::EnumNew {
                        dst,
                        descriptor_id,
                        variant,
                    } => {
                        let obj_ptr = alloc_obj!(self, fp, regs.pc, regs.func, descriptor_id)?;
                        // No safe point between the allocation and these
                        // non-allocating writes: stamp the tag through the fresh
                        // pointer, then publish it to `dst`.
                        write_enum_tag(obj_ptr, variant);
                        write_ptr(fp, dst, obj_ptr);
                    },

                    MicroOp::EnumReadVariantField {
                        dst,
                        enum_ref,
                        offset,
                        size,
                    } => {
                        // Double deref of the enum fat pointer to the heap object,
                        // then copy the field bytes directly — no intermediate
                        // scratch reference. The offset is a static uniform offset
                        // (no tag dispatch).
                        let (ref_base, ref_off) = read_fat_ptr(fp, enum_ref);
                        let obj_ptr = read_ptr(ref_base, ref_off as usize);
                        // Non-overlapping: `dst` is a stack-region slot, the field
                        // is heap-object bytes.
                        std::ptr::copy_nonoverlapping(
                            obj_ptr.add(offset as usize),
                            fp.add(dst.into()),
                            size as usize,
                        );
                    },

                    MicroOp::EnumWriteVariantField {
                        enum_ref,
                        offset,
                        src,
                        size,
                    } => {
                        let (ref_base, ref_off) = read_fat_ptr(fp, enum_ref);
                        let obj_ptr = read_ptr(ref_base, ref_off as usize);
                        // Non-overlapping: `src` is a stack-region slot, the field
                        // is heap-object bytes.
                        std::ptr::copy_nonoverlapping(
                            fp.add(src.into()),
                            obj_ptr.add(offset as usize),
                            size as usize,
                        );
                    },

                    MicroOp::DeepCopyHeapPtrs { base, ref offsets } => {
                        // Pre-condition: each `base + off` holds a heap pointer
                        // byte-copied from another value, so the pointee is still
                        // shared with that value. Replace each non-null pointer
                        // with a pointer to a fresh deep copy, making the value at
                        // `base` independent (Move value semantics). Null (e.g. an
                        // empty vector) stays null.
                        //
                        // TODO(metering): the IR-level cost of the materializing
                        // instruction charges only the shallow byte move, not this
                        // heap-graph copy. Change charging to reflect at least the
                        // amount of work done.
                        //
                        // TODO(perf): the materializing op currently emits a byte
                        // move into `base` followed by this in-place fixup that
                        // re-reads from `base`. A fused `src -> dst` deep copy
                        // would drop the separate move and the re-read. It would
                        // need deep-copying variants of `ReadRef`, `Read*Field`,
                        // etc.
                        if let &[off] = offsets.as_ref() {
                            // Fast path for copying whole-enum / whole-vector /
                            // aggregate with one heap backed field: a single owned
                            // pointer at one offset. Uses single-root `deep_copy`,
                            // avoiding the batch's per-op `Vec`s.
                            if let Some(src) = NonNull::new(read_ptr(fp, (base.0 + off) as usize)) {
                                let new = self.deep_copy(regs, src)?;
                                write_ptr(fp, (base.0 + off) as usize, new.as_ptr());
                            }
                        } else {
                            let mut live_offsets: Vec<u32> = Vec::with_capacity(offsets.len());
                            let mut sources: Vec<NonNull<u8>> = Vec::with_capacity(offsets.len());
                            for &off in offsets.iter() {
                                if let Some(src) =
                                    NonNull::new(read_ptr(fp, (base.0 + off) as usize))
                                {
                                    live_offsets.push(off);
                                    sources.push(src);
                                }
                            }
                            let copies = self.deep_copy_batch(regs, &sources)?;
                            for (off, new) in live_offsets.iter().zip(copies) {
                                write_ptr(fp, (base.0 + off) as usize, new.as_ptr());
                            }
                        }
                    },
                }
            }

            regs.pc += 1;
        };

        self.registers = regs;
        Ok(outcome)
    }

    /// Allocates vector from constant pool and writes data pointer into `dst`.
    #[inline(never)]
    fn exec_store_imm_vec(
        &mut self,
        regs: VMRegisters,
        dst: FrameOffset,
        idx: ConstantPoolIndex,
    ) -> VMResult<()> {
        // SAFETY: `regs.func` points to the live, currently-executing function.
        let module_id = unsafe { regs.func.as_ref() }.module_id;
        let (ty, bytes) = self.load_constant(module_id, idx)?;

        // SAFETY: `dst` is a verified 8-byte frame slot for a vector pointer
        // and is writable (no aliasing to the heap).
        unsafe {
            let dst = regs.fp.add(usize::from(dst));
            deserialize_or_gc(
                self.loader.guard(),
                &mut self.heap,
                ty,
                bytes,
                dst,
                self.loader.guard(),
                &mut self.read_write_set,
                &self.root_pool,
                &self.extensions,
                regs.fp,
                crate::heap::TopFrame::Function {
                    func: regs.func,
                    pc: regs.pc,
                },
            )
        }
    }

    /// Allocates the vector at exact capacity in a single allocation, writes
    /// the length, copies each source element from the frame, and writes the
    /// heap pointer to `op.dst`. Any GC runs inside the allocation, before the
    /// fp-relative element reads, so those reads see post-GC pointers.
    ///
    /// # Safety
    ///
    /// - `regs.fp` is the current frame pointer.
    /// - `op.dst` and each `op.srcs` slot are in-bounds for the current frame.
    // Outlined to keep the dispatch loop small: an inlined body here adds to
    // the register pressure that competes for `fp`'s register across the loop.
    #[inline(never)]
    unsafe fn exec_vec_pack(&mut self, regs: VMRegisters, op: &VecPackOp) -> VMResult<()> {
        let fp = regs.fp;
        let count = op.srcs.len() as u64;
        // TODO(perf): `alloc_vec!` zero-fills the whole allocation, but every
        // payload byte is overwritten below before the op returns and no GC can
        // intervene afterwards. A fully-initialized-payload alloc variant would
        // avoid the dead memset. Padding needs to be carefully considered for
        // the optimization.
        let vec_ptr = alloc_vec!(
            self,
            fp,
            regs.pc,
            regs.func,
            op.descriptor_id,
            op.elem_size,
            count
        )?;
        unsafe {
            write_u64(vec_ptr, VEC_LENGTH_OFFSET, count);
            for (elem_idx, src) in op.srcs.iter().enumerate() {
                // Non-overlapping: the source is a frame slot, the destination
                // is in the freshly-allocated heap vector — disjoint regions.
                std::ptr::copy_nonoverlapping(
                    fp.add((*src).into()),
                    vec_elem_ptr(vec_ptr, elem_idx as u64, op.elem_size) as *mut u8,
                    op.elem_size as usize,
                );
            }
            write_ptr(fp, op.dst, vec_ptr);
        }
        Ok(())
    }

    /// Checks that the vector's length equals `op.dsts.len()` — erroring on a
    /// mismatch; then copies each element into its destination slot. A null
    /// `src` is the empty vector: it unpacks cleanly when `dsts` is empty and
    /// fails the length check otherwise.
    ///
    /// # Safety
    ///
    /// - `fp` is the current frame pointer.
    /// - `op.src` and each `op.dsts` slot are in-bounds for the current frame.
    unsafe fn exec_vec_unpack(&mut self, fp: *mut u8, op: &VecUnpackOp) -> VMResult<()> {
        unsafe {
            let vec_ptr = read_ptr(fp, op.src);
            let expected = op.dsts.len() as u64;
            let actual = read_vec_len(vec_ptr);
            if actual != expected {
                return Err(VMInternalError::new(
                    RuntimeError::VecUnpackLengthMismatch { expected, actual },
                ));
            }
            for (elem_idx, dst) in op.dsts.iter().enumerate() {
                // Non-overlapping: the source is a heap vector element, the
                // destination is a frame slot — disjoint regions.
                std::ptr::copy_nonoverlapping(
                    vec_elem_ptr(vec_ptr, elem_idx as u64, op.elem_size),
                    fp.add((*dst).into()),
                    op.elem_size as usize,
                );
            }
        }
        Ok(())
    }

    /// Deep-copy the value tree rooted at the specified source into the
    /// local heap. Returns the data-region pointer of the freshly-allocated
    /// root copy.
    ///
    /// # Safety
    ///
    /// Source must point to the data region of a live object whose header is
    /// at `src - OBJECT_HEADER_SIZE`.
    unsafe fn deep_copy(&mut self, regs: VMRegisters, root: NonNull<u8>) -> VMResult<NonNull<u8>> {
        // SAFETY: by this function's contract `root` points to a live object.
        unsafe {
            deep_copy_or_gc(
                &mut self.heap,
                self.loader.guard(),
                &mut self.read_write_set,
                &self.root_pool,
                &self.extensions,
                regs.fp,
                TopFrame::Function {
                    func: regs.func,
                    pc: regs.pc,
                },
                root,
            )
        }
    }

    /// Deep-copy each of `sources` into the local heap, returning the new root
    /// pointers in the same order.
    ///
    /// All sources are rooted for the whole batch, so a GC triggered partway
    /// through preserves and relocates the not-yet-copied ones. Because
    /// `try_deep_copy` never GCs mid-copy, a *successful* pass builds every
    /// result without an intervening GC, so the already-built results need no
    /// root of their own — only the sources are rooted. Mirrors
    /// [`Self::deep_copy`]'s single GC-then-retry-once policy, batched over all
    /// sources.
    ///
    /// # Safety
    ///
    /// Every `source` must point to the data region of a live object whose
    /// header is at `source - OBJECT_HEADER_SIZE`.
    unsafe fn deep_copy_batch(
        &mut self,
        regs: VMRegisters,
        sources: &[NonNull<u8>],
    ) -> VMResult<Vec<NonNull<u8>>> {
        // SAFETY: each source is a live object (caller contract); the handle
        // keeps it live and relocated across any GC during the batch.
        let guards: Vec<ObjectHandle> = sources
            .iter()
            .map(|&src| unsafe { self.root_pool.root_object(src.as_ptr()) })
            .collect();
        // First attempt. On out-of-memory, the partial copies are unrooted
        // garbage; drop them, GC (which relocates the rooted sources), and
        // retry the whole batch once.
        let mut out = Vec::with_capacity(guards.len());
        let mut needs_gc = false;
        for guard in &guards {
            // SAFETY: each root holds a live object; GC keeps `guard.ptr()`
            // valid and relocated.
            match unsafe {
                self.heap
                    .try_deep_copy(self.loader.guard(), NonNull::new_unchecked(guard.ptr()))
            } {
                Ok(ptr) => out.push(ptr),
                Err(AllocationError::RuntimeError(err)) => return Err(VMInternalError::new(err)),
                Err(AllocationError::OutOfHeapMemory { .. }) => {
                    needs_gc = true;
                    break;
                },
            }
        }
        if !needs_gc {
            return Ok(out);
        }
        gc_collect!(self, regs.fp, regs.pc, regs.func)?;
        out.clear();
        for guard in &guards {
            // SAFETY: as above, after relocation.
            let ptr = unsafe {
                self.heap
                    .try_deep_copy(self.loader.guard(), NonNull::new_unchecked(guard.ptr()))
            }
            .map_err(AllocationError::into_runtime_error)?;
            out.push(ptr);
        }
        Ok(out)
    }

    /// Implementation of `MicroOp::PackClosure`.
    ///
    /// Allocates a closure heap object and a paired `ClosureCapturedData`
    /// (Materialized) heap object, copies captured values from the caller's
    /// frame into the captured data object, and writes the closure pointer
    /// to `op.dst`.
    ///
    /// For non-capturing closures the captured-data allocation is skipped
    /// and `captured_data_ptr` is left null.
    ///
    /// For capturing closures, two allocations happen. The closure object
    /// is rooted in the [`RootPool`] immediately after its own allocation
    /// and stays rooted across the captured-data allocation, so any GC
    /// triggered by the second allocation preserves the closure (even
    /// before it's written to `op.dst`) and relocates our local pointer.
    ///
    // TODO(perf): swap the generic [`RootPool`] machinery here for a
    // `Heap::reserve(n)` API that pre-secures headroom for both
    // allocations so the second `alloc_obj` can never trigger GC.
    // The pool is still justified for native functions but is overkill for
    // the 2-allocation case here and costs us a handle construction /
    // pointer reload.
    ///
    /// # Safety
    ///
    /// - `regs.fp` is the current frame pointer.
    /// - Each `op.captured` slot is in-bounds for the current frame (the
    ///   verifier checks this).
    /// - The closure descriptor must list `CLOSURE_CAPTURED_DATA_PTR_OFFSET`
    ///   (relative to the data segment, so `32 - 8 = 24`) in its
    ///   `pointer_offsets`, so GC traces the captured-data pointer after
    ///   the closure is reachable via the frame slot.
    #[inline(never)]
    unsafe fn exec_pack_closure(&mut self, regs: VMRegisters, op: &PackClosureOp) -> VMResult<()> {
        let fp = regs.fp;
        unsafe {
            // Fast path: non-capturing closure. Skip the second allocation
            // and leave `captured_data_ptr` as the zeroed/null value written
            // by `alloc_obj`. No pinning needed — only one allocation.
            if op.captured.is_empty() {
                let closure = alloc_obj!(self, fp, regs.pc, regs.func, CLOSURE_DESCRIPTOR_ID)?;
                self.write_closure_func_ref_and_mask(closure, op);
                write_ptr(fp, op.dst, closure);
                return Ok(());
            }

            // Capturing path: allocate the closure object, root it, then
            // allocate and populate the captured-data object.
            //
            // The closure has a null `captured_data_ptr` between the two
            // allocations — safe for GC to see (null heap pointers are
            // skipped). Rooting keeps the closure live across the second
            // allocation and lets GC update the rooted slot in-place if
            // the object is relocated.
            let closure_ptr = alloc_obj!(self, fp, regs.pc, regs.func, CLOSURE_DESCRIPTOR_ID)?;
            // SAFETY: `alloc_obj!` returns a live, freshly-allocated object.
            let closure_root = self.root_pool.root_object(closure_ptr);

            self.write_closure_func_ref_and_mask(closure_root.ptr(), op);

            // SAFETY: the verifier guarantees `captured_data_descriptor_id`
            // is `Some` whenever `captured` is non-empty. The values-region
            // size comes from the op, not the descriptor (see `PackClosureOp`).
            let captured_desc_id = op
                .captured_data_descriptor_id
                .expect("verifier ensures Some when captured is non-empty");
            let captured_data = alloc_captured_data!(
                self,
                fp,
                regs.pc,
                regs.func,
                op.values_size,
                captured_desc_id
            )?;
            *captured_data.add(CAPTURED_DATA_TAG_OFFSET) = CAPTURED_DATA_TAG_MATERIALIZED;
            // Persist the exact values-region size so `CallClosure` can validate
            // a lazily-resolved callee's captured layout against it; the header
            // records only the alignment-rounded allocation size.
            //
            // TODO(correctness): persisting only the total lets `CallClosure` check totals but
            // not the per-capture `(size, align)` breakdown. Persist that layout
            // here to enable element-wise validation of an `Unresolved` callee.
            write_u32(
                captured_data,
                CAPTURED_DATA_VALUES_SIZE_OFFSET,
                op.values_size,
            );

            // Captured values are laid out at their natural alignment within
            // the values region (see `next_captured_value_offset`), matching the
            // descriptor's pointer offsets and the call-site read layout.
            let mut cursor = 0usize;
            for slot in &op.captured {
                let (offset, next) =
                    next_captured_value_offset(cursor, slot.size as usize, slot.align as usize);
                std::ptr::copy_nonoverlapping(
                    fp.add(slot.offset.into()),
                    captured_data.add(CAPTURED_DATA_VALUES_OFFSET + offset),
                    slot.size as usize,
                );
                cursor = next;
            }

            let closure = closure_root.ptr();
            write_ptr(closure, CLOSURE_CAPTURED_DATA_PTR_OFFSET, captured_data);
            write_ptr(fp, op.dst, closure);

            Ok(())
        }
    }

    /// Write the `func_ref` enum and the mask into a freshly allocated closure
    /// heap object.
    #[inline]
    unsafe fn write_closure_func_ref_and_mask(&self, closure: *mut u8, op: &PackClosureOp) {
        unsafe {
            match &op.func_ref {
                ClosureFuncRef::Resolved(func_ptr) => {
                    *closure.add(CLOSURE_FUNC_REF_OFFSET + FUNC_REF_TAG_OFFSET) =
                        FUNC_REF_TAG_RESOLVED;
                    write_ptr(
                        closure,
                        CLOSURE_FUNC_REF_OFFSET + FUNC_REF_PAYLOAD_OFFSET,
                        func_ptr.as_non_null().as_ptr() as *const u8,
                    );
                },
                ClosureFuncRef::Unresolved(func_ref) => {
                    *closure.add(CLOSURE_FUNC_REF_OFFSET + FUNC_REF_TAG_OFFSET) =
                        FUNC_REF_TAG_UNRESOLVED;
                    write_ptr(
                        closure,
                        CLOSURE_FUNC_REF_OFFSET + FUNC_REF_PAYLOAD_OFFSET,
                        func_ref.as_raw_ptr() as *const u8,
                    );
                },
            }
            write_u64(closure, CLOSURE_MASK_OFFSET, op.mask);
        }
    }

    /// Implementation of `MicroOp::CallClosure`.
    ///
    /// Reads the closure at `op.closure_src`, interleaves its captured
    /// values with the provided arguments into the callee's parameter
    /// region using the mask and the callee's `param_slots` (each
    /// argument lands at its parameter's natural-aligned offset), then
    /// performs the standard call protocol.
    ///
    /// Handles both `ClosureFuncRef::Resolved` and `Unresolved` targets — the
    /// latter resolved lazily via the loader on first call, then memoized into
    /// the closure object as `Resolved` so repeat calls take the fast path.
    /// Captured data must be Materialized; other tags are errors.
    ///
    /// # Safety
    ///
    /// - `func` is the currently executing function (caller).
    /// - `regs` carries the caller's VM registers.
    /// - `op.closure_src` holds a non-null heap pointer to a valid closure
    ///   object.
    /// - The callee's `param_slots` list has one (offset, size) entry per
    ///   declared parameter; the last entry's `offset + size` equals
    ///   `callee.param_region_size`.
    /// - The captured values in the captured-data object are packed in
    ///   param order and their sizes match the corresponding `param_slots`
    ///   entries (enforced by `PackClosure`).
    #[inline(never)]
    unsafe fn exec_call_closure(
        &mut self,
        func: &Function,
        mut regs: VMRegisters,
        op: &CallClosureOp,
    ) -> VMResult<VMRegisters> {
        let fp = regs.fp;
        unsafe {
            let closure = read_ptr(fp, op.closure_src);
            if closure.is_null() {
                invariant_violation!(NullClosure);
            }
            // Guard the func-pointer cast below against a non-closure pointer.
            let descriptor_id = read_descriptor(closure);
            if descriptor_id != CLOSURE_DESCRIPTOR_ID.0 {
                invariant_violation!(ClosureSrcNotClosure { descriptor_id });
            }

            // Decode `ClosureFuncRef`: `Resolved` carries a baked-in function
            // pointer; `Unresolved` carries a symbolic `(module, name, ty_args)`
            // identity resolved lazily.
            let func_tag = *closure.add(CLOSURE_FUNC_REF_OFFSET + FUNC_REF_TAG_OFFSET);
            let payload = read_ptr(closure, CLOSURE_FUNC_REF_OFFSET + FUNC_REF_PAYLOAD_OFFSET);
            if payload.is_null() {
                invariant_violation!(NullFuncRefInClosure);
            }
            let (callee, resolved_now): (&Function, bool) = match func_tag {
                FUNC_REF_TAG_RESOLVED => (&*(payload as *const Function), false),
                FUNC_REF_TAG_UNRESOLVED => {
                    let func_ref = &*(payload as *const FunctionRef);
                    let func_ptr = self.load_function(
                        func_ref.module_id,
                        func_ref.func_name,
                        func_ref.ty_args,
                    )?;
                    (func_ptr.as_ref_unchecked(), true)
                },
                other => invariant_violation!(InvalidClosureFuncRefTag { tag: other }),
            };

            // Re-read `closure` from its frame slot (a GC root): if resolution
            // relocated the heap, the slot holds the moved object while the
            // local `closure` above would dangle. A `Resolved` closure re-reads
            // the same pointer.
            let closure = read_ptr(fp, op.closure_src);
            let mask = read_u64(closure, CLOSURE_MASK_OFFSET);
            let captured_data = read_ptr(closure, CLOSURE_CAPTURED_DATA_PTR_OFFSET);

            // Callee-dependent validation runs once, on the first resolution of
            // an `Unresolved` closure; we then memoize the resolved pointer into
            // the closure object (flipping it to `Resolved`) so later calls of
            // the same closure value skip both the loader and these checks. An
            // already-`Resolved` closure was validated earlier — by the verifier
            // at pack time, or by this block on a prior call — and its captured
            // data is immutable, so re-validation is unnecessary.
            if resolved_now {
                let num_params = callee.param_slots.len();
                if num_params > 64 {
                    invariant_violation!(TooManyClosureParams { num_params });
                }
                // The mask must not reference parameters the resolved callee
                // lacks, or the captured-read cursor below would desync.
                if num_params < 64 && (mask >> num_params) != 0 {
                    invariant_violation!(ClosureMaskExceedsParams { mask, num_params });
                }
                if mask != 0 {
                    if captured_data.is_null() {
                        invariant_violation!(NullCapturedData);
                    }
                    let cap_tag = *captured_data.add(CAPTURED_DATA_TAG_OFFSET);
                    if cap_tag != CAPTURED_DATA_TAG_MATERIALIZED {
                        // TODO(completeness): only the Materialized captured-data tag is supported.
                        todo!("CallClosure: unsupported captured-data tag {} (only Materialized supported now)", cap_tag);
                    }
                    // The resolved callee's captured `values_size` must equal the
                    // one the object was packed with (persisted exactly, not the
                    // alignment-rounded header), rejecting signature skew before
                    // the copy loop reads the bytes at the callee's offsets.
                    //
                    // TODO(correctness): this compares only the *total* values_size, so a
                    // same-total but different per-capture `(size, align)` layout
                    // (a cross-module skew) still passes and is read at the wrong
                    // per-value offsets. The `Resolved` path is fully covered by
                    // the verifier's per-slot size+align check; closing it for
                    // `Unresolved` targets needs the packed per-capture layout
                    // persisted in the object to compare element-wise here.
                    let expected = captured_values_size(
                        callee
                            .param_slots
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| (mask >> i) & 1 != 0)
                            .map(|(_, pslot)| (pslot.size, pslot.align)),
                    );
                    let packed = read_u32(captured_data, CAPTURED_DATA_VALUES_SIZE_OFFSET);
                    if expected != packed {
                        invariant_violation!(ClosureCapturedLayoutMismatch { expected, packed });
                    }
                }
                // Memoize: bake the resolved function pointer into the closure
                // and flip the tag to `Resolved`. The func-ref payload is not
                // GC-traced and `FunctionPtr` is a stable leaked address, so this
                // survives heap relocation exactly like a closure packed as
                // `Resolved`.
                write_ptr(
                    closure,
                    CLOSURE_FUNC_REF_OFFSET + FUNC_REF_PAYLOAD_OFFSET,
                    callee as *const Function as *mut u8,
                );
                *closure.add(CLOSURE_FUNC_REF_OFFSET + FUNC_REF_TAG_OFFSET) = FUNC_REF_TAG_RESOLVED;
            }

            // Walk the callee's parameters, interleaving captured values
            // (from the captured-data object, packed sequentially in
            // parameter order) with provided arguments (from the caller's
            // frame).
            //
            // TODO(perf): replace this interleaving scheme with one where the
            // specializer pre-writes provided arguments into the callee's
            // parameter region at the call site (densely packed, in
            // parameter order — exactly the same codegen as a regular
            // call), and `CallClosure` then walks parameter positions
            // backwards patching captured values in. This eliminates the
            // `provided_args` list, makes non-capturing closures skip
            // any copies (every iteration is a no-op move-in-place),
            // and unifies closure call codegen with direct call codegen.
            // See George's pseudocode in PR #19519 review thread.

            // Stack-overflow check up front: `call_unchecked` skips the
            // check, so we do it here before writing the callee's
            // parameters at `new_fp`.
            let new_fp = self.check_stack_for_call(func, fp, callee.extended_frame_size)?;

            // Captured values are read from the values region at their natural
            // alignment — the same layout `exec_pack_closure` wrote and the
            // descriptor records (see `next_captured_value_offset`).
            //
            // Interleaving is safe: captured writes land at/above `new_fp`,
            // while provided-arg sources are always below it — the destacker
            // never leaves a closure call's provided args in the callee
            // region. The ranges are disjoint, so a captured write cannot
            // clobber a provided source read on a later iteration.
            let mut cursor = 0usize;
            let mut provided_idx = 0usize;
            for (i, pslot) in callee.param_slots.iter().enumerate() {
                let param_size = pslot.size;
                // Destination is the parameter's aligned offset in the callee frame.
                let dst = new_fp.add(pslot.offset.0 as usize);
                let is_captured = (mask >> i) & 1 != 0;
                if is_captured {
                    let (offset, next) = next_captured_value_offset(
                        cursor,
                        param_size as usize,
                        pslot.align as usize,
                    );
                    std::ptr::copy_nonoverlapping(
                        captured_data.add(CAPTURED_DATA_VALUES_OFFSET + offset),
                        dst,
                        param_size as usize,
                    );
                    cursor = next;
                } else {
                    let Some(slot) = op.provided_args.get(provided_idx) else {
                        invariant_violation!(NotEnoughProvidedArgs);
                    };
                    if slot.size != param_size {
                        invariant_violation!(ClosureArgSizeMismatch {
                            provided_idx,
                            provided_size: slot.size,
                            param_idx: i,
                            param_size,
                        });
                    }
                    // `copy`, not `copy_nonoverlapping`: a provided source
                    // (below `new_fp`) never overlaps its callee-region
                    // destination today, but the planned pre-write-then-patch
                    // redesign copies in place and needs memmove semantics.
                    std::ptr::copy(fp.add(slot.offset.into()), dst, slot.size as usize);
                    provided_idx += 1;
                }
            }
            let provided = op.provided_args.len();
            if provided_idx != provided {
                invariant_violation!(ClosureArgsCountMismatch {
                    provided,
                    consumed: provided_idx,
                });
            }

            // Standard call protocol: save metadata and switch to the
            // callee frame. Use the unchecked variant — we already
            // validated the stack above.
            self.call_unchecked(func, &mut regs, callee, new_fp)?;
            Ok(regs)
        }
    }

    /// Compute the callee's frame pointer and verify the callee's full
    /// frame fits on the stack. Returns the new frame pointer on success.
    ///
    /// # Safety
    ///
    /// `caller` must be the currently executing function and `fp` the
    /// current frame pointer.
    #[inline(always)]
    unsafe fn check_stack_for_call(
        &self,
        caller: &Function,
        fp: *mut u8,
        callee_extended_frame_size: usize,
    ) -> VMResult<*mut u8> {
        unsafe {
            let new_fp = fp.add(caller.param_and_local_sizes_sum + FRAME_METADATA_SIZE);
            let stack_end = self.stack.as_ptr().add(self.stack.len());
            if new_fp.add(callee_extended_frame_size) > stack_end {
                return Err(VMInternalError::new(RuntimeError::StackOverflow));
            }
            Ok(new_fp)
        }
    }

    /// Implementation of call opcodes. Validates the stack first, then
    /// hands off to [`Self::call_unchecked`].
    ///
    /// # Safety
    ///
    /// `callee` must point to a valid, live `Function`. `regs` must carry the
    /// caller's VM registers and `caller` be the currently executing
    /// function.
    #[inline(always)]
    unsafe fn call(
        &mut self,
        caller: &Function,
        regs: &mut VMRegisters,
        callee: &Function,
    ) -> VMResult<()> {
        let new_fp =
            unsafe { self.check_stack_for_call(caller, regs.fp, callee.extended_frame_size)? };
        unsafe { self.call_unchecked(caller, regs, callee, new_fp) }
    }

    /// Perform the standard call protocol after the caller has already
    /// computed `new_fp` (and ensured the callee's frame fits on the
    /// stack). Used by `exec_call_closure`, which needs `new_fp` earlier
    /// to safely write the callee's parameters before the call.
    ///
    /// # Safety
    ///
    /// In addition to the contract on [`Self::call`], `new_fp` must equal
    /// `regs.fp + caller.param_and_local_sizes_sum + FRAME_METADATA_SIZE`, and
    /// `new_fp + callee.extended_frame_size` must be within the stack
    /// (i.e., the caller has already passed the check that
    /// [`Self::check_stack_for_call`] performs).
    #[inline(always)]
    unsafe fn call_unchecked(
        &mut self,
        caller: &Function,
        regs: &mut VMRegisters,
        callee: &Function,
        new_fp: *mut u8,
    ) -> VMResult<()> {
        // Charge the callee's entry block before any of its instructions run.
        self.gas_meter.charge(callee.entry_gas)?;
        unsafe {
            // Zero everything beyond parameters (locals, metadata, callee
            // arg/return region) so pointer slots start as null.
            // The parameter region (0..param_region_size) was already
            // written by the caller as call arguments.
            if callee.zero_frame {
                let zero_size = callee.extended_frame_size - callee.param_region_size;
                std::ptr::write_bytes(new_fp.add(callee.param_region_size), 0, zero_size);
            }
            // Save the caller's registers into its frame metadata before
            // switching `regs` to the callee.
            self.write_frame_metadata(caller, regs);
        }
        regs.fp = new_fp;
        regs.pc = 0;
        regs.func = NonNull::from(callee);
        Ok(())
    }

    /// Write the calling-convention frame metadata `(saved_pc, saved_fp,
    /// saved_func_ptr)` at the end of the caller's frame. Used by both
    /// regular calls (where it's read back by `Return`) and native calls
    /// (where it lets stack-walking natives traverse the chain).
    ///
    /// # Safety
    ///
    /// `caller` must be the currently executing function and `regs` must carry
    /// the caller's VM registers.
    #[inline(always)]
    unsafe fn write_frame_metadata(&self, caller: &Function, regs: &VMRegisters) {
        unsafe {
            let meta = regs.fp.add(caller.param_and_local_sizes_sum);
            write_u64(meta, META_SAVED_PC_OFFSET, (regs.pc + 1) as u64);
            write_ptr(meta, META_SAVED_FP_OFFSET, regs.fp);
            write_ptr(
                meta,
                META_SAVED_FUNC_PTR_OFFSET,
                regs.func.as_ptr() as *const u8,
            );
        }
    }

    /// Dispatch a [`MicroOp::CallNative`].
    ///
    /// # Safety
    ///
    /// `caller` must be the currently executing function and `regs` must carry
    /// the caller's VM registers.
    unsafe fn exec_call_native(
        &mut self,
        caller: &Function,
        regs: VMRegisters,
        native_idx: NativeIdx,
        ty_args: InternedTypeList,
        abi: &NativeABI,
    ) -> VMResult<Option<(u64, Option<String>)>> {
        // Check if we have enough space on the stack to allocate the native's frame.
        let new_fp =
            unsafe { self.check_stack_for_call(caller, regs.fp, abi.total_frame_size() as usize)? };

        // Write frame metadata just like normal calls. This is still needed
        // as some natives may want to inspect the call stack.
        unsafe { self.write_frame_metadata(caller, &regs) };

        // Zero out return-slot bytes that extend past the args, for extra safety.
        if abi.total_frame_size() > abi.args_end() {
            unsafe {
                std::ptr::write_bytes(
                    new_fp.add(abi.args_end() as usize),
                    0,
                    (abi.total_frame_size() - abi.args_end()) as usize,
                );
            }
        }

        // The native's own allocations scan GC roots from `self.registers.fp`,
        // so point it at the native frame for the duration of the call, then
        // restore the caller frame.
        let saved_fp = regs.fp;
        self.registers.fp = new_fp;
        let result = {
            let func = self.natives.lookup_by_idx(native_idx).ok_or_else(|| {
                RuntimeError::InvariantViolation(RuntimeInvariantViolation::NativeIdxOutOfBounds {
                    idx: native_idx.0,
                    registry_size: self.natives.len(),
                })
            })?;
            // TODO(cleanup): eventually pass the interpreter context itself rather than
            // unpacking `gas_meter` / `heap` / `read_write_set` (and giving
            // access to the loader + global context). Need to first work out
            // whether that's sound under the context's interior-mutability model
            // — clearer once everything (rws → table natives, gas → all) is
            // wired up.
            let guard = self.loader.guard();
            let ctx = ProductionNativeContext::new(
                new_fp,
                abi,
                view_type_list(ty_args),
                &mut self.gas_meter,
                guard,
                guard,
                self.resource_provider,
                &mut self.heap,
                &mut self.read_write_set,
                &self.extensions,
            );
            func(&ctx)
        };
        self.registers.fp = saved_fp;

        result.map(|status| match status {
            NativeStatus::Success => None,
            NativeStatus::Abort { code, message } => Some((code, message)),
        })
    }
}
