// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Interpreter with unified stack (frame metadata inlined in linear memory)
//! and a bump-allocated heap with copying GC.

use crate::{
    error::{ArithOp, RuntimeError, RuntimeResult, RuntimeStatus, Signedness, VecOp},
    global_storage::{EntryPtr, ResourceReadWriteSet},
    heap::{
        macros::{alloc_obj, alloc_vec, gc_collect, grow_vec_ref},
        pinned_roots::PinnedRoots,
        AllocationError, Heap,
    },
    invariant_violation,
    memory::{
        read_fat_ptr, read_obj_size, read_ptr, read_u64, read_u8, vec_elem_ptr, write_fat_ptr,
        write_ptr, write_u64, MemoryRegion,
    },
    types::{
        StepResult, ABORT_MESSAGE_SIZE_LIMIT, DEFAULT_HEAP_SIZE, DEFAULT_STACK_SIZE,
        META_SAVED_FP_OFFSET, META_SAVED_FUNC_PTR_OFFSET, META_SAVED_PC_OFFSET, VEC_DATA_OFFSET,
        VEC_LENGTH_OFFSET,
    },
    value_compare::structural_compare,
    ExecutionContext,
};
use mono_move_core::{
    storage::resource_provider::StorageKey, CallClosureOp, ClosureFuncRef, DescriptorId,
    DescriptorProvider, FrameOffset, Function, IntBinaryOp, IntNegateOp, IntOperand, IntShiftOp,
    IntTy, MicroOp, PackClosureOp, ShiftOperand, CAPTURED_DATA_TAG_MATERIALIZED,
    CAPTURED_DATA_TAG_OFFSET, CAPTURED_DATA_VALUES_OFFSET, CLOSURE_CAPTURED_DATA_PTR_OFFSET,
    CLOSURE_DESCRIPTOR_ID, CLOSURE_FUNC_REF_OFFSET, CLOSURE_MASK_OFFSET, FRAME_METADATA_SIZE,
    FUNC_REF_PAYLOAD_OFFSET, FUNC_REF_TAG_OFFSET, FUNC_REF_TAG_RESOLVED, MAX_ALIGN,
    OBJECT_HEADER_SIZE,
};
use mono_move_gas::GasMeter;
use move_core_types::{
    account_address::AccountAddress,
    int256::{I256, U256},
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::{
    cmp::Ordering,
    ptr::{null, NonNull},
};

// ---------------------------------------------------------------------------
// Runtime state
// ---------------------------------------------------------------------------

/// Interpreter context with a unified call stack and a GC-managed heap.
pub struct InterpreterContext<'a, T: ExecutionContext + DescriptorProvider> {
    /// Per-transaction context (function resolution, gas counters,
    /// descriptor table, etc.).
    pub(crate) exec_ctx: &'a mut T,

    pub(crate) pc: usize,
    /// Pointer to the currently executing function.
    pub(crate) current_func: NonNull<Function>,
    /// Absolute pointer into the linear stack memory. Operand accesses are a
    /// single addition (`fp + offset`).
    /// Recomputed only during calls and returns.
    pub(crate) frame_ptr: *mut u8,

    pub(crate) stack: MemoryRegion,
    pub(crate) heap: Heap,
    /// Auxiliary GC root set for temporarily-live heap pointers that are
    /// not yet stored in any frame slot (e.g. between two allocations in a
    /// fused micro-op, or in native functions).
    pub(crate) pinned_roots: PinnedRoots,
    /// Per-transaction global-storage state: working map of cached
    /// reads / pending writes, linear journal for rollback, and
    /// checkpoint stack.
    pub(crate) read_write_set: ResourceReadWriteSet,
    rng: StdRng,
}

impl<'a, T: ExecutionContext + DescriptorProvider> InterpreterContext<'a, T> {
    pub fn new(exec_ctx: &'a mut T, entry: &Function) -> Self {
        Self::with_heap_size(exec_ctx, entry, DEFAULT_HEAP_SIZE)
    }

    /// Create a new context with a custom heap size (for testing GC pressure).
    pub fn with_heap_size(exec_ctx: &'a mut T, entry: &Function, heap_size: usize) -> Self {
        let verification_errors = crate::verifier::verify_function(entry, exec_ctx);
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
        let frame_ptr = unsafe { base.add(FRAME_METADATA_SIZE) };

        unsafe {
            write_u64(base, META_SAVED_PC_OFFSET, 0);
            write_u64(base, META_SAVED_FP_OFFSET, 0);
            write_ptr(base, META_SAVED_FUNC_PTR_OFFSET, null());
        }

        Self {
            exec_ctx,
            pc: 0,
            current_func: NonNull::from(entry),
            frame_ptr,
            stack,
            heap: Heap::new(heap_size),
            pinned_roots: PinnedRoots::new(),
            read_write_set: ResourceReadWriteSet::new(),
            rng: StdRng::seed_from_u64(0),
        }
    }

    pub fn set_rng_seed(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed);
    }

    pub fn gc_count(&self) -> usize {
        self.heap.gc_count
    }

    /// TODO: move to execution context
    pub fn checkpoint(&mut self) {
        self.read_write_set.checkpoint();
    }

    /// TODO: move to execution context
    pub fn rollback(&mut self, n: usize) -> RuntimeResult<()> {
        self.read_write_set.rollback(n)
    }

    /// TODO: move to execution context
    pub fn checkpoint_depth(&self) -> usize {
        self.read_write_set.checkpoint_depth()
    }

    /// TODO: move to execution context
    pub fn current_epoch(&self) -> u64 {
        self.read_write_set.current_epoch()
    }

    /// TODO: move to execution context
    pub fn journal_len(&self) -> usize {
        self.read_write_set.journal_len()
    }

    /// Reset the context to call a different function, preserving the heap.
    ///
    /// Use `set_root_arg` to place arguments before calling `run()`.
    ///
    // TODO: invoke() is test-only for now. When used with real gas budgets,
    // decide whether to reset the gas meter here.
    pub fn invoke(&mut self, func: &Function) {
        let base = self.stack.as_ptr();

        // Reset execution state to root frame.
        self.frame_ptr = unsafe { base.add(FRAME_METADATA_SIZE) };
        self.pc = 0;
        self.current_func = NonNull::from(func);

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
                    self.frame_ptr.add(func.param_sizes_sum),
                    0,
                    func.extended_frame_size - func.param_sizes_sum,
                );
            }
        }
    }

    /// Read a u64 from the root frame's slot 0 (where the result lands).
    pub fn root_result(&self) -> u64 {
        unsafe { read_u64(self.stack.as_ptr(), FRAME_METADATA_SIZE) }
    }

    /// Read a u64 from the root frame at the given byte offset.
    pub fn root_result_at(&self, offset: u32) -> u64 {
        unsafe { read_u64(self.stack.as_ptr(), FRAME_METADATA_SIZE + offset as usize) }
    }

    /// Read `size` raw bytes from the root frame at the given byte offset.
    pub fn root_result_bytes(&self, offset: u32, size: u32) -> &[u8] {
        unsafe {
            let base = self
                .stack
                .as_ptr()
                .add(FRAME_METADATA_SIZE + offset as usize);
            std::slice::from_raw_parts(base, size as usize)
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

    /// Allocate a vector of `u64` values on the heap and return its address
    /// as a `u64` suitable for embedding in args. Useful for passing pre-built
    /// data into a program without generating initialization micro-ops.
    pub fn alloc_u64_vec(
        &mut self,
        descriptor_id: DescriptorId,
        values: &[u64],
    ) -> RuntimeResult<u64> {
        let n = values.len() as u64;
        let ptr = alloc_vec!(self, self.frame_ptr, descriptor_id, 8, n)?;
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
// TODO: re-verify inlining after non-trivial changes to the helpers,
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
/// # Safety
/// `base.add(byte_offset)` must be valid for a read of `size_of::<T>()`
/// bytes, with the appropriate alignment when `align_of::<T>() <= MAX_ALIGN`.
#[inline(always)]
pub(crate) unsafe fn read_int<T: Copy>(base: *const u8, byte_offset: impl Into<usize>) -> T {
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

/// Read a 32-byte [`AccountAddress`] from a frame slot.
///
/// # Safety
///
/// `[fp + offset, fp + offset + 32]` must lie within the current
/// frame's accessible region.
#[inline(always)]
unsafe fn read_address(fp: *const u8, offset: FrameOffset) -> AccountAddress {
    let ptr = unsafe { fp.add(offset.into()) as *const AccountAddress };
    unsafe { ptr.read() }
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
        unsafe fn $fn_name(fp: *mut u8, op: &IntBinaryOp) -> RuntimeResult<()> {
            unsafe {
                macro_rules! exec {
                    ($ty: ty,$_sign: tt,$rhs: expr) => {{
                        let lhs_val: $ty = read_int::<$ty>(fp, op.lhs);
                        let rhs_val: $ty = $rhs;
                        let result: $ty = <$ty>::$method(lhs_val, rhs_val)
                            .ok_or_else(|| RuntimeError::$variant $body)?;
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
        unsafe fn $fn_name(fp: *mut u8, op: &IntBinaryOp) -> RuntimeResult<()> {
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
        unsafe fn $fn_name(fp: *mut u8, op: &IntShiftOp) -> RuntimeResult<()> {
            unsafe {
                let shift_amount: u8 = match &op.rhs {
                    ShiftOperand::SlotU8(off) => read_u8(fp, *off),
                    ShiftOperand::ImmU8(v) => *v,
                };
                let bit_width = op.ty.bit_width() as u32;
                if (shift_amount as u32) >= bit_width {
                    return Err(RuntimeError::ShiftAmountOutOfRange {
                        op: $base_op,
                        ty: op.ty,
                        shift_amount,
                        bit_width,
                    });
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
unsafe fn exec_int_negate(fp: *mut u8, op: &IntNegateOp) -> RuntimeResult<()> {
    unsafe {
        macro_rules! exec {
            ($ty:ty) => {{
                let src_val: $ty = read_int::<$ty>(fp, op.src);
                let result: $ty = <$ty>::checked_neg(src_val)
                    .ok_or_else(|| RuntimeError::NegateMinOverflow { ty: op.ty })?;
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

// ---------------------------------------------------------------------------
// Interpreter loop
// ---------------------------------------------------------------------------

impl<T: ExecutionContext + DescriptorProvider> InterpreterContext<'_, T> {
    #[inline(always)]
    pub fn step(&mut self) -> RuntimeResult<StepResult> {
        // SAFETY: Current function is always a valid, non-null pointer because
        // it is derived from function reference (e.g., entrypoint) or when
        // executing a call instruction, which stores a valid pointer.
        let func = unsafe { self.current_func.as_ref() };

        // TODO:
        //  1. Hoist this out and see effects on performance
        //  2. See if swapping code does not need to be seen by same txn, and
        //     only its future re-executions see new code.
        let code_guard = func.code.load();
        let code = code_guard.as_slice();

        if self.pc >= code.len() {
            invariant_violation!(PcOutOfBounds {
                pc: self.pc,
                func_name: func.name().to_string(),
                code_len: code.len(),
            });
        }

        let fp = self.frame_ptr;
        let instr = &code[self.pc];

        // SAFETY: fp points into the interpreter's linear stack; all byte
        // offsets are within the current frame (enforced by the bytecode
        // compiler). Heap pointers read from the frame are kept valid by GC.
        unsafe {
            match *instr {
                // ----- Control flow (set pc explicitly, return early) -----
                MicroOp::CallIndirect {
                    module_id,
                    func_name,
                    ty_args,
                } => {
                    // TODO: full flow should be like this:
                    //
                    //   1. IC lookup:
                    //      - Hit:  return pointer,
                    //      - Miss: goto 2.
                    //   2. target = load_function(...)
                    //   3. IC insert target
                    //   4. Patching:
                    //      If can patch caller, try it.
                    let target = self
                        .exec_ctx
                        .load_function(module_id, func_name, ty_args)
                        .map_err(RuntimeError::Loader)?;
                    // SAFETY: `target` points to a `Function`, which is not reclaimed during
                    // execution as guaranteed by the execution guard.
                    return self.call(func, fp, target.as_ref_unchecked());
                },
                MicroOp::CallDirect { ptr } => {
                    return self.call(func, fp, ptr.as_ref_unchecked());
                },

                MicroOp::JumpNotZeroU64 { target, src } => {
                    self.pc = if read_u64(fp, src) != 0 {
                        target.into()
                    } else {
                        self.pc + 1
                    };
                    return Ok(StepResult::Continue);
                },

                MicroOp::JumpGreaterEqualU64Imm { target, src, imm } => {
                    self.pc = if read_u64(fp, src) >= imm {
                        target.into()
                    } else {
                        self.pc + 1
                    };
                    return Ok(StepResult::Continue);
                },

                MicroOp::JumpLessU64Imm { target, src, imm } => {
                    self.pc = if read_u64(fp, src) < imm {
                        target.into()
                    } else {
                        self.pc + 1
                    };
                    return Ok(StepResult::Continue);
                },

                MicroOp::JumpGreaterU64Imm { target, src, imm } => {
                    self.pc = if read_u64(fp, src) > imm {
                        target.into()
                    } else {
                        self.pc + 1
                    };
                    return Ok(StepResult::Continue);
                },

                MicroOp::JumpLessEqualU64Imm { target, src, imm } => {
                    self.pc = if read_u64(fp, src) <= imm {
                        target.into()
                    } else {
                        self.pc + 1
                    };
                    return Ok(StepResult::Continue);
                },

                MicroOp::JumpLessU64 { target, lhs, rhs } => {
                    self.pc = if read_u64(fp, lhs) < read_u64(fp, rhs) {
                        target.into()
                    } else {
                        self.pc + 1
                    };
                    return Ok(StepResult::Continue);
                },

                MicroOp::JumpGreaterEqualU64 { target, lhs, rhs } => {
                    self.pc = if read_u64(fp, lhs) >= read_u64(fp, rhs) {
                        target.into()
                    } else {
                        self.pc + 1
                    };
                    return Ok(StepResult::Continue);
                },

                MicroOp::JumpNotEqualU64 { target, lhs, rhs } => {
                    self.pc = if read_u64(fp, lhs) != read_u64(fp, rhs) {
                        target.into()
                    } else {
                        self.pc + 1
                    };
                    return Ok(StepResult::Continue);
                },

                MicroOp::Eq { dst, lhs, rhs, ty } => {
                    let ord = structural_compare(ty, fp.add(lhs.into()), fp.add(rhs.into()))?;
                    // Booleans occupy a u64-width slot (see `LdTrue` lowering),
                    // so write the full 8 bytes.
                    write_u64(fp, dst, (ord == Ordering::Equal) as u64);
                },

                MicroOp::Neq { dst, lhs, rhs, ty } => {
                    let ord = structural_compare(ty, fp.add(lhs.into()), fp.add(rhs.into()))?;
                    write_u64(fp, dst, (ord != Ordering::Equal) as u64);
                },

                MicroOp::Jump { target } => {
                    self.pc = target.into();
                    return Ok(StepResult::Continue);
                },

                MicroOp::Return => {
                    let meta = fp.sub(FRAME_METADATA_SIZE);

                    let saved_func_ptr =
                        read_ptr(meta, META_SAVED_FUNC_PTR_OFFSET) as *const Function;
                    if saved_func_ptr.is_null() {
                        return Ok(StepResult::Done);
                    }
                    // SAFETY: We have just checked that the saved function
                    // pointer is non-null.
                    self.current_func = NonNull::new_unchecked(saved_func_ptr as *mut Function);

                    self.pc = read_u64(meta, META_SAVED_PC_OFFSET) as usize;
                    self.frame_ptr = read_ptr(meta, META_SAVED_FP_OFFSET);
                    return Ok(StepResult::Continue);
                },

                MicroOp::Abort { code } => {
                    let code = read_u64(fp, code);
                    return Ok(StepResult::Aborted {
                        code,
                        message: None,
                    });
                },

                MicroOp::AbortMsg { code, message } => {
                    let code = read_u64(fp, code);
                    let vec_ptr = read_ptr(fp, message);
                    let message = if vec_ptr.is_null() {
                        String::new()
                    } else {
                        // TODO: charge gas for abort message bytes.
                        let len = read_u64(vec_ptr, VEC_LENGTH_OFFSET) as usize;
                        if len > ABORT_MESSAGE_SIZE_LIMIT {
                            return Err(RuntimeError::AbortMessageTooLong {
                                len,
                                max: ABORT_MESSAGE_SIZE_LIMIT,
                            });
                        }
                        // SAFETY: `vec_ptr` is non-null (checked above) and
                        // points at a heap vector with `len` initialized
                        // bytes at `VEC_DATA_OFFSET`.
                        let data = vec_ptr.add(VEC_DATA_OFFSET);
                        String::from_utf8(std::slice::from_raw_parts(data, len).to_vec())
                            .map_err(|_| RuntimeError::InvalidAbortMessage)?
                    };
                    return Ok(StepResult::Aborted {
                        code,
                        message: Some(message),
                    });
                },

                // ----- Arithmetic -----
                MicroOp::StoreImm8 { dst, ref imm } => write_int::<[u8; 8]>(fp, dst, *imm),
                MicroOp::StoreImm16 { dst, ref imm } => write_int::<[u8; 16]>(fp, dst, **imm),
                MicroOp::StoreImm32 { dst, ref imm } => write_int::<[u8; 32]>(fp, dst, **imm),

                // Add
                MicroOp::AddU64 { dst, lhs, rhs } => {
                    checked_binop_u64(fp, dst, lhs, rhs, u64::checked_add).ok_or_else(|| {
                        RuntimeError::ArithmeticOverflow {
                            op: ArithOp::Add,
                            ty: IntTy::U64,
                        }
                    })?
                },
                MicroOp::AddU64Imm { dst, src, imm } => {
                    checked_imm_op_u64(fp, dst, src, imm, u64::checked_add).ok_or_else(|| {
                        RuntimeError::ArithmeticOverflow {
                            op: ArithOp::Add,
                            ty: IntTy::U64,
                        }
                    })?
                },

                // Sub
                MicroOp::SubU64 { dst, lhs, rhs } => {
                    checked_binop_u64(fp, dst, lhs, rhs, u64::checked_sub).ok_or_else(|| {
                        RuntimeError::ArithmeticUnderflow {
                            op: ArithOp::Sub,
                            ty: IntTy::U64,
                        }
                    })?
                },
                MicroOp::SubU64Imm { dst, src, imm } => {
                    checked_imm_op_u64(fp, dst, src, imm, u64::checked_sub).ok_or_else(|| {
                        RuntimeError::ArithmeticUnderflow {
                            op: ArithOp::Sub,
                            ty: IntTy::U64,
                        }
                    })?
                },
                // dst = imm - src, so flip the operand order.
                MicroOp::RSubU64Imm { dst, src, imm } => {
                    checked_imm_op_u64(fp, dst, src, imm, |s, i| u64::checked_sub(i, s))
                        .ok_or_else(|| RuntimeError::ArithmeticUnderflow {
                            op: ArithOp::Sub,
                            ty: IntTy::U64,
                        })?
                },

                // Mul
                MicroOp::MulU64 { dst, lhs, rhs } => {
                    checked_binop_u64(fp, dst, lhs, rhs, u64::checked_mul).ok_or_else(|| {
                        RuntimeError::ArithmeticOverflow {
                            op: ArithOp::Mul,
                            ty: IntTy::U64,
                        }
                    })?
                },
                MicroOp::MulU64Imm { dst, src, imm } => {
                    checked_imm_op_u64(fp, dst, src, imm, u64::checked_mul).ok_or_else(|| {
                        RuntimeError::ArithmeticOverflow {
                            op: ArithOp::Mul,
                            ty: IntTy::U64,
                        }
                    })?
                },

                // Div / Mod
                MicroOp::DivU64 { dst, lhs, rhs } => {
                    checked_binop_u64(fp, dst, lhs, rhs, u64::checked_div).ok_or_else(|| {
                        RuntimeError::DivisionByZero {
                            op: ArithOp::Div,
                            ty: IntTy::U64,
                        }
                    })?
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
                    checked_binop_u64(fp, dst, lhs, rhs, u64::checked_rem).ok_or_else(|| {
                        RuntimeError::DivisionByZero {
                            op: ArithOp::Mod,
                            ty: IntTy::U64,
                        }
                    })?
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
                MicroOp::BitAndU64 { dst, lhs, rhs } => binop_u64(fp, dst, lhs, rhs, |a, b| a & b),
                MicroOp::BitOrU64 { dst, lhs, rhs } => binop_u64(fp, dst, lhs, rhs, |a, b| a | b),
                MicroOp::BitXorU64 { dst, lhs, rhs } => binop_u64(fp, dst, lhs, rhs, |a, b| a ^ b),

                // Shifts
                MicroOp::ShlU64 { dst, lhs, rhs } => shift_u64(fp, dst, lhs, rhs, |v, s| v << s)
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
                MicroOp::ShrU64 { dst, lhs, rhs } => shift_u64(fp, dst, lhs, rhs, |v, s| v >> s)
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
                    gc_collect!(self)?;
                },

                MicroOp::Move8 { dst, src } => {
                    let v = read_u64(fp, src);
                    write_u64(fp, dst, v);
                },

                MicroOp::Move { dst, src, size } => {
                    std::ptr::copy(fp.add(src.into()), fp.add(dst.into()), size as usize);
                },

                // ----- Vector instructions -----
                MicroOp::VecNew { dst } => {
                    write_ptr(fp, dst, std::ptr::null());
                },

                MicroOp::VecLen { dst, vec_ref } => {
                    let (ref_base, ref_off) = read_fat_ptr(fp, vec_ref);
                    let vec_ptr = read_ptr(ref_base, ref_off as usize);
                    let len = if vec_ptr.is_null() {
                        0
                    } else {
                        read_u64(vec_ptr, VEC_LENGTH_OFFSET)
                    };
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
                        vec_ptr = alloc_vec!(self, fp, descriptor_id, elem_size, 4)?;
                        // Re-read base after potential GC.
                        let (ref_base, ref_off) = read_fat_ptr(fp, vec_ref);
                        write_ptr(ref_base, ref_off as usize, vec_ptr);
                    }

                    let len = read_u64(vec_ptr, VEC_LENGTH_OFFSET);
                    let total = read_obj_size(vec_ptr) as usize;
                    let cap_in_elems = ((total - OBJECT_HEADER_SIZE - VEC_DATA_OFFSET)
                        / elem_size as usize) as u64;

                    if len >= cap_in_elems {
                        vec_ptr = grow_vec_ref!(self, fp, vec_ref.into(), elem_size, len + 1)?;
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
                    if vec_ptr.is_null() {
                        return Err(RuntimeError::PopFromEmptyVector);
                    }
                    let len = read_u64(vec_ptr, VEC_LENGTH_OFFSET);
                    if len == 0 {
                        return Err(RuntimeError::PopFromEmptyVector);
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
                    if vec_ptr.is_null() {
                        return Err(RuntimeError::VectorIndexOutOfBounds {
                            op: VecOp::LoadElem,
                            idx,
                            len: 0,
                        });
                    }
                    let len = read_u64(vec_ptr, VEC_LENGTH_OFFSET);
                    if idx >= len {
                        return Err(RuntimeError::VectorIndexOutOfBounds {
                            op: VecOp::LoadElem,
                            idx,
                            len,
                        });
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
                    if vec_ptr.is_null() {
                        return Err(RuntimeError::VectorIndexOutOfBounds {
                            op: VecOp::StoreElem,
                            idx,
                            len: 0,
                        });
                    }
                    let len = read_u64(vec_ptr, VEC_LENGTH_OFFSET);
                    if idx >= len {
                        return Err(RuntimeError::VectorIndexOutOfBounds {
                            op: VecOp::StoreElem,
                            idx,
                            len,
                        });
                    }
                    std::ptr::copy_nonoverlapping(
                        fp.add(src.into()),
                        vec_elem_ptr(vec_ptr, idx, elem_size) as *mut u8,
                        elem_size as usize,
                    );
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
                    if vec_ptr.is_null() {
                        return Err(RuntimeError::VectorIndexOutOfBounds {
                            op: VecOp::Borrow,
                            idx,
                            len: 0,
                        });
                    }
                    let len = read_u64(vec_ptr, VEC_LENGTH_OFFSET);
                    if idx >= len {
                        return Err(RuntimeError::VectorIndexOutOfBounds {
                            op: VecOp::Borrow,
                            idx,
                            len,
                        });
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
                    let ptr = alloc_obj!(self, fp, descriptor_id)?;
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

                MicroOp::Charge { cost } => {
                    self.exec_ctx.gas_meter().charge(cost)?;
                },

                MicroOp::PackClosure(ref op) => {
                    self.exec_pack_closure(fp, op)?;
                },
                MicroOp::CallClosure(ref op) => {
                    return self.exec_call_closure(func, fp, op);
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

                MicroOp::Exists { addr, ty, dst } => {
                    let address = read_address(fp, addr);
                    let exists = self.read_write_set.exists(
                        self.exec_ctx.resource_provider(),
                        StorageKey::Resource(address, ty),
                    )?;
                    // TODO(correctness): temporary hack to avoid boolean writes.
                    write_u64(fp, dst, if exists { 1 } else { 0 });
                },

                MicroOp::BorrowGlobal { addr, ty, dst } => {
                    let address = read_address(fp, addr);
                    let ptr = self.read_write_set.borrow_global(
                        self.exec_ctx.resource_provider(),
                        StorageKey::Resource(address, ty),
                    )?;
                    // A reference is a 16-byte fat pointer; the borrow points
                    // at the start of the resource, so the offset half is 0.
                    write_fat_ptr(fp, dst, ptr.as_ptr(), 0);
                },

                MicroOp::BorrowGlobalMut { addr, ty, dst } => {
                    let address = read_address(fp, addr);
                    let key = StorageKey::Resource(address, ty);
                    let ptr = match self
                        .read_write_set
                        .try_borrow_global_mut(self.exec_ctx.resource_provider(), key)?
                    {
                        EntryPtr::Writable(ptr) => ptr,
                        EntryPtr::NonWritable(ptr) => {
                            let ptr = self.deep_copy(ptr)?;
                            self.read_write_set.commit_borrow_global_mut(key, ptr);
                            ptr
                        },
                    };
                    // A reference is a 16-byte fat pointer; the borrow points
                    // at the start of the resource, so the offset half is 0.
                    write_fat_ptr(fp, dst, ptr.as_ptr(), 0);
                },

                MicroOp::MoveFrom { addr, ty, dst } => {
                    let address = read_address(fp, addr);
                    let key = StorageKey::Resource(address, ty);
                    let entry_ptr = self
                        .read_write_set
                        .try_move_from(self.exec_ctx.resource_provider(), key)?;
                    let ptr = match entry_ptr {
                        EntryPtr::Writable(ptr) => ptr,
                        EntryPtr::NonWritable(ptr) => {
                            let ptr = self.deep_copy(ptr)?;
                            self.read_write_set.commit_move_from(key);
                            ptr
                        },
                    };
                    write_ptr(fp, dst, ptr.as_ptr());
                },

                MicroOp::MoveTo { addr, ty, src } => {
                    let address = read_address(fp, addr);
                    let Some(ptr) = NonNull::new(read_ptr(fp, src)) else {
                        invariant_violation!(MoveToNullSource);
                    };

                    self.read_write_set.move_to(
                        self.exec_ctx.resource_provider(),
                        StorageKey::Resource(address, ty),
                        ptr,
                    )?;
                },
            }
        }

        self.pc += 1;
        Ok(StepResult::Continue)
    }

    /// Deep-copy the value tree rooted at the specified source into the
    /// local heap. Returns the data-region pointer of the freshly-allocated
    /// root copy.
    ///
    /// # Safety
    ///
    /// Source must point to the data region of a live object whose header is
    /// at `src - OBJECT_HEADER_SIZE`.
    unsafe fn deep_copy(&mut self, root: NonNull<u8>) -> RuntimeResult<NonNull<u8>> {
        let root_guard = self.pinned_roots.pin(root);
        // SAFETY: `root_guard.get()` returns the caller-supplied root, which
        // by this function's contract points to a live object.
        match unsafe { self.heap.try_deep_copy(self.exec_ctx, root_guard.get()) } {
            Ok(ptr) => Ok(ptr),
            Err(AllocationError::RuntimeError(err)) => Err(err),
            Err(AllocationError::OutOfHeapMemory { .. }) => {
                gc_collect!(self)?;
                // Re-read the root pointer from the pin, as its address have
                // been changed by the GC.
                // SAFETY: the pin keeps the root live across GC; the relocated
                // pointer still points to the same live object.
                unsafe {
                    self.heap
                        .try_deep_copy(self.exec_ctx, root_guard.get())
                        .map_err(AllocationError::into_runtime_error)
                }
            },
        }
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
    /// is pinned via [`PinnedRoots`] immediately after its own allocation
    /// and stays pinned across the captured-data allocation, so any GC
    /// triggered by the second allocation preserves the closure (even
    /// before it's written to `op.dst`) and relocates our local pointer.
    ///
    // TODO: swap the generic `PinnedRoots` machinery here for a
    // `Heap::reserve(n)` API that pre-secures headroom for both
    // allocations so the second `alloc_obj` can never trigger GC.
    // `PinnedRoots` is still justified for native functions but is
    // overkill for the 2-allocation case here and costs us a guard
    // construction / pointer reload.
    ///
    /// # Safety
    ///
    /// - `fp` is the current frame pointer.
    /// - Each `op.captured` slot is in-bounds for the current frame (the
    ///   verifier checks this).
    /// - The closure descriptor must list `CLOSURE_CAPTURED_DATA_PTR_OFFSET`
    ///   (relative to the data segment, so `32 - 8 = 24`) in its
    ///   `pointer_offsets`, so GC traces the captured-data pointer after
    ///   the closure is reachable via the frame slot.
    unsafe fn exec_pack_closure(&mut self, fp: *mut u8, op: &PackClosureOp) -> RuntimeResult<()> {
        unsafe {
            // Fast path: non-capturing closure. Skip the second allocation
            // and leave `captured_data_ptr` as the zeroed/null value written
            // by `alloc_obj`. No pinning needed — only one allocation.
            if op.captured.is_empty() {
                let closure = alloc_obj!(self, fp, CLOSURE_DESCRIPTOR_ID)?;
                self.write_closure_func_ref_and_mask(closure, op);
                write_ptr(fp, op.dst, closure);
                return Ok(());
            }

            // Capturing path: allocate the closure object, pin it, then
            // allocate and populate the captured-data object.
            //
            // The closure has a null `captured_data_ptr` between the two
            // allocations — safe for GC to see (null heap pointers are
            // skipped). Pinning keeps the closure live across the second
            // allocation and lets GC update the pinned slot in-place if
            // the object is relocated.
            let closure_ptr = alloc_obj!(self, fp, CLOSURE_DESCRIPTOR_ID)?;
            let pin = self.pinned_roots.pin(NonNull::new_unchecked(closure_ptr));

            self.write_closure_func_ref_and_mask(pin.get().as_ptr(), op);

            // SAFETY: the verifier guarantees `captured_data_descriptor_id`
            // is `Some(CapturedData)` whenever `captured` is non-empty.
            let captured_desc_id = op
                .captured_data_descriptor_id
                .expect("verifier ensures Some when captured is non-empty");
            let captured_data = alloc_obj!(self, fp, captured_desc_id)?;
            *captured_data.add(CAPTURED_DATA_TAG_OFFSET) = CAPTURED_DATA_TAG_MATERIALIZED;

            let mut captured_offset = CAPTURED_DATA_VALUES_OFFSET;
            for slot in &op.captured {
                std::ptr::copy_nonoverlapping(
                    fp.add(slot.offset.into()),
                    captured_data.add(captured_offset),
                    slot.size as usize,
                );
                captured_offset += slot.size as usize;
            }

            let closure = pin.get().as_ptr();
            write_ptr(closure, CLOSURE_CAPTURED_DATA_PTR_OFFSET, captured_data);
            write_ptr(fp, op.dst, closure);

            Ok(())
        }
    }

    /// Write the `func_ref` enum (Resolved only in v0) and the mask into
    /// a freshly allocated closure heap object.
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
            }
            write_u64(closure, CLOSURE_MASK_OFFSET, op.mask);
        }
    }

    /// Implementation of `MicroOp::CallClosure`.
    ///
    /// Reads the closure at `op.closure_src`, interleaves its captured
    /// values with the provided arguments into the callee's parameter
    /// region using the mask and the callee's `param_sizes`, then
    /// performs the standard call protocol.
    ///
    /// Only supports `ClosureFuncRef::Resolved` + Materialized captured
    /// data for v0; other cases are errors.
    ///
    /// # Safety
    ///
    /// - `func` is the currently executing function (caller).
    /// - `fp` is the current frame pointer.
    /// - `op.closure_src` holds a non-null heap pointer to a valid closure
    ///   object.
    /// - The callee's `param_sizes` list has one entry per declared
    ///   parameter and sums to `callee.param_sizes_sum`.
    /// - The captured values in the captured-data object are packed in
    ///   param order and their sizes match the corresponding `param_sizes`
    ///   entries (enforced by `PackClosure`).
    unsafe fn exec_call_closure(
        &mut self,
        func: &Function,
        fp: *mut u8,
        op: &CallClosureOp,
    ) -> RuntimeResult<StepResult> {
        unsafe {
            let closure = read_ptr(fp, op.closure_src);
            if closure.is_null() {
                invariant_violation!(NullClosure);
            }

            // Decode `ClosureFuncRef`. v0 supports only Resolved.
            let func_tag = *closure.add(CLOSURE_FUNC_REF_OFFSET + FUNC_REF_TAG_OFFSET);
            if func_tag != FUNC_REF_TAG_RESOLVED {
                todo!(
                    "CallClosure: unsupported func_ref tag {} (only Resolved supported in v0)",
                    func_tag
                );
            }
            let callee_raw = read_ptr(closure, CLOSURE_FUNC_REF_OFFSET + FUNC_REF_PAYLOAD_OFFSET)
                as *const Function;
            if callee_raw.is_null() {
                invariant_violation!(NullFuncRefInClosure);
            }
            let callee = &*callee_raw;

            let mask = read_u64(closure, CLOSURE_MASK_OFFSET);

            // Walk the callee's parameters, interleaving captured values
            // (from the captured-data object, packed sequentially in
            // parameter order) with provided arguments (from the caller's
            // frame).
            //
            // TODO: replace this interleaving scheme with one where the
            // specializer pre-writes provided arguments into the callee's
            // parameter region at the call site (densely packed, in
            // parameter order — exactly the same codegen as a regular
            // call), and `CallClosure` then walks parameter positions
            // backwards patching captured values in. This eliminates the
            // `provided_args` list, makes non-capturing closures skip
            // any copies (every iteration is a no-op move-in-place),
            // and unifies closure call codegen with direct call codegen.
            // See George's pseudocode in PR #19519 review thread.
            if callee.param_sizes.len() > 64 {
                invariant_violation!(TooManyClosureParams {
                    num_params: callee.param_sizes.len(),
                });
            }

            // Stack-overflow check up front: `call_unchecked` skips the
            // check, so we do it here before writing the callee's
            // parameters at `new_fp`.
            let new_fp = self.check_stack_for_call(func, fp, callee)?;

            // Only validate captured-data when the closure actually has
            // captures. Non-capturing closures leave `captured_data_ptr`
            // null (see `exec_pack_closure`).
            let captured_data = read_ptr(closure, CLOSURE_CAPTURED_DATA_PTR_OFFSET);
            if mask != 0 {
                if captured_data.is_null() {
                    invariant_violation!(NullCapturedData);
                }
                let cap_tag = *captured_data.add(CAPTURED_DATA_TAG_OFFSET);
                if cap_tag != CAPTURED_DATA_TAG_MATERIALIZED {
                    todo!("CallClosure: unsupported captured-data tag {} (only Materialized supported in v0)", cap_tag);
                }
            }

            let mut captured_value_offset = CAPTURED_DATA_VALUES_OFFSET;
            let mut provided_idx = 0usize;
            let mut param_offset_in_callee = 0usize;
            for (i, &param_size) in callee.param_sizes.iter().enumerate() {
                let is_captured = (mask >> i) & 1 != 0;
                if is_captured {
                    std::ptr::copy_nonoverlapping(
                        captured_data.add(captured_value_offset),
                        new_fp.add(param_offset_in_callee),
                        param_size as usize,
                    );
                    captured_value_offset += param_size as usize;
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
                    // Use `copy` (not `copy_nonoverlapping`): a provided
                    // arg's source slot may lie in the caller's reserved
                    // callee-arg region, which is the same memory as the
                    // callee's parameter region at `new_fp`. The
                    // overlap is also routine under the planned
                    // pre-write-then-patch redesign in the TODO above.
                    std::ptr::copy(
                        fp.add(slot.offset.into()),
                        new_fp.add(param_offset_in_callee),
                        slot.size as usize,
                    );
                    provided_idx += 1;
                }
                param_offset_in_callee += param_size as usize;
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
            self.call_unchecked(func, fp, callee, new_fp)
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
        callee: &Function,
    ) -> RuntimeResult<*mut u8> {
        unsafe {
            let new_fp = fp.add(caller.param_and_local_sizes_sum + FRAME_METADATA_SIZE);
            let stack_end = self.stack.as_ptr().add(self.stack.len());
            if new_fp.add(callee.extended_frame_size) > stack_end {
                return Err(RuntimeError::StackOverflow);
            }
            Ok(new_fp)
        }
    }

    /// Implementation of call opcodes. Validates the stack first, then
    /// hands off to [`Self::call_unchecked`].
    ///
    /// # Safety
    ///
    /// `callee` must point to a valid, live `Function`. `fp` must be the
    /// current frame pointer and `caller` the currently executing function.
    #[inline(always)]
    unsafe fn call(
        &mut self,
        caller: &Function,
        fp: *mut u8,
        callee: &Function,
    ) -> RuntimeResult<StepResult> {
        let new_fp = unsafe { self.check_stack_for_call(caller, fp, callee)? };
        unsafe { self.call_unchecked(caller, fp, callee, new_fp) }
    }

    /// Perform the standard call protocol after the caller has already
    /// computed `new_fp` (and ensured the callee's frame fits on the
    /// stack). Used by `exec_call_closure`, which needs `new_fp` earlier
    /// to safely write the callee's parameters before the call.
    ///
    /// # Safety
    ///
    /// In addition to the contract on [`Self::call`], `new_fp` must equal
    /// `fp + caller.param_and_local_sizes_sum + FRAME_METADATA_SIZE`, and
    /// `new_fp + callee.extended_frame_size` must be within the stack
    /// (i.e., the caller has already passed the check that
    /// [`Self::check_stack_for_call`] performs).
    #[inline(always)]
    unsafe fn call_unchecked(
        &mut self,
        caller: &Function,
        fp: *mut u8,
        callee: &Function,
        new_fp: *mut u8,
    ) -> RuntimeResult<StepResult> {
        unsafe {
            // Zero everything beyond parameters (locals, metadata, callee
            // arg/return region) so pointer slots start as null.
            // The parameter region (0..param_sizes_sum) was already
            // written by the caller as call arguments.
            if callee.zero_frame {
                let zero_size = callee.extended_frame_size - callee.param_sizes_sum;
                std::ptr::write_bytes(new_fp.add(callee.param_sizes_sum), 0, zero_size);
            }
            let meta = fp.add(caller.param_and_local_sizes_sum);
            write_u64(meta, META_SAVED_PC_OFFSET, (self.pc + 1) as u64);
            write_ptr(meta, META_SAVED_FP_OFFSET, fp);
            write_ptr(
                meta,
                META_SAVED_FUNC_PTR_OFFSET,
                self.current_func.as_ptr() as *const u8,
            );
            self.frame_ptr = new_fp;
        }
        self.pc = 0;
        self.current_func = NonNull::from(callee);
        Ok(StepResult::Continue)
    }

    // TODO: Hoist pc, fp, and current_func into local variables in the run loop
    // instead of reading/writing self.pc, self.frame_ptr, self.current_func each
    // iteration. LLVM can't keep them in registers because heap operations
    // (VecPushBack, etc.) take &mut self, which may alias these fields.
    // Write back only on CallFunc/Return.
    pub fn run(&mut self) -> RuntimeResult<RuntimeStatus> {
        loop {
            match self.step()? {
                StepResult::Continue => {},
                StepResult::Done => return Ok(RuntimeStatus::Success),
                StepResult::Aborted { code, message } => {
                    return Ok(RuntimeStatus::Aborted { code, message })
                },
            }
        }
    }
}
