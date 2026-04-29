// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Interpreter with unified stack (frame metadata inlined in linear memory)
//! and a bump-allocated heap with copying GC.

use crate::{
    bail,
    error::ExecutionResult,
    heap::{
        macros::{alloc_obj, alloc_vec, gc_collect, grow_vec_ref},
        pinned_roots::PinnedRoots,
        Heap,
    },
    memory::{read_ptr, read_u32, read_u64, vec_elem_ptr, write_ptr, write_u64, MemoryRegion},
    types::{
        ObjectDescriptor, StepResult, DEFAULT_HEAP_SIZE, DEFAULT_STACK_SIZE, HEADER_SIZE_OFFSET,
        META_SAVED_FP_OFFSET, META_SAVED_FUNC_PTR_OFFSET, META_SAVED_PC_OFFSET, VEC_DATA_OFFSET,
        VEC_LENGTH_OFFSET,
    },
};
use mono_move_core::{
    CallClosureOp, ClosureFuncRef, DescriptorId, Function, MicroOp, PackClosureOp, SizedSlot,
    TransactionContext, CAPTURED_DATA_TAG_MATERIALIZED, CAPTURED_DATA_TAG_OFFSET,
    CAPTURED_DATA_VALUES_OFFSET, CLOSURE_CAPTURED_DATA_PTR_OFFSET, CLOSURE_FUNC_REF_OFFSET,
    CLOSURE_MASK_OFFSET, FRAME_METADATA_SIZE, FUNC_REF_PAYLOAD_OFFSET, FUNC_REF_TAG_OFFSET,
    FUNC_REF_TAG_RESOLVED,
};
use mono_move_gas::GasMeter;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::ptr::{null, NonNull};

// ---------------------------------------------------------------------------
// Runtime state
// ---------------------------------------------------------------------------

/// Interpreter context with a unified call stack and a GC-managed heap.
pub struct InterpreterContext<'a, G: GasMeter> {
    /// Per-transaction context (function resolution, gas counters, etc.).
    pub(crate) txn_ctx: &'a dyn TransactionContext,
    /// Externally-provided object layout descriptors (will be replaced by execution context).
    pub(crate) descriptors: &'a [ObjectDescriptor],
    /// Externally-provided gas meter (will be replaced by execution context).
    pub(crate) gas_meter: G,

    pub(crate) pc: usize,
    /// Pointer to the currently executing function.
    pub(crate) current_func: NonNull<Function>,
    /// Absolute pointer into the linear stack memory. Operand accesses are a
    /// single addition (`fp + offset`). Recomputed only on `CallFunc` and `Return`.
    pub(crate) frame_ptr: *mut u8,

    pub(crate) stack: MemoryRegion,
    pub(crate) heap: Heap,
    /// Auxiliary GC root set for temporarily-live heap pointers that are
    /// not yet stored in any frame slot (e.g. between two allocations in a
    /// fused micro-op, or in native functions).
    pub(crate) pinned_roots: PinnedRoots,
    rng: StdRng,
}

impl<'a, G: GasMeter> InterpreterContext<'a, G> {
    pub fn new(
        txn_ctx: &'a dyn TransactionContext,
        descriptors: &'a [ObjectDescriptor],
        gas_meter: G,
        entry: &Function,
    ) -> Self {
        Self::with_heap_size(txn_ctx, descriptors, gas_meter, entry, DEFAULT_HEAP_SIZE)
    }

    /// Create a new context with a custom heap size (for testing GC pressure).
    pub fn with_heap_size(
        txn_ctx: &'a dyn TransactionContext,
        descriptors: &'a [ObjectDescriptor],
        gas_meter: G,
        entry: &Function,
        heap_size: usize,
    ) -> Self {
        let verification_errors = crate::verifier::verify_function(entry, descriptors);
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
            txn_ctx,
            descriptors,
            gas_meter,
            pc: 0,
            current_func: NonNull::from(entry),
            frame_ptr,
            stack,
            heap: Heap::new(heap_size),
            pinned_roots: PinnedRoots::new(),
            rng: StdRng::seed_from_u64(0),
        }
    }

    pub fn set_rng_seed(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed);
    }

    pub fn gc_count(&self) -> usize {
        self.heap.gc_count
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
    ) -> ExecutionResult<u64> {
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
// Interpreter loop
// ---------------------------------------------------------------------------

impl<G: GasMeter> InterpreterContext<'_, G> {
    #[inline(always)]
    pub fn step(&mut self) -> ExecutionResult<StepResult> {
        // SAFETY: Current function is always a valid, non-null pointer because
        // it is derived from function reference (e.g., entrypoint) or when
        // executing a call instruction, which stores a valid pointer.
        let func = unsafe { self.current_func.as_ref() };
        // SAFETY: The function's code is allocated in an executable arena that
        // is alive for the duration of execution.
        let code = unsafe { func.code.as_ref_unchecked() };
        if self.pc >= code.len() {
            bail!(
                "pc out of bounds: pc={} but function {} has {} instructions",
                self.pc,
                unsafe { func.name.as_ref_unchecked() },
                code.len()
            );
        }

        let fp = self.frame_ptr;
        let instr = &code[self.pc];

        // SAFETY: fp points into the interpreter's linear stack; all byte
        // offsets are within the current frame (enforced by the bytecode
        // compiler). Heap pointers read from the frame are kept valid by GC.
        unsafe {
            match *instr {
                // ----- Control flow (set pc explicitly, return early) -----
                MicroOp::CallFunc { .. } => {
                    bail!("CallFunc must be resolved before execution");
                },
                MicroOp::CallIndirect {
                    executable_id,
                    func_name,
                } => {
                    // Cross-module slow path, may trigger lazy module loading here.
                    let Some(target) = self.txn_ctx.resolve_function(executable_id, func_name)
                    else {
                        // TODO: Once loader is in place, this should load the module
                        // and retry resolution instead of bailing immediately.
                        bail!("CallIndirect: function not found");
                    };
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

                // ----- Arithmetic -----
                MicroOp::StoreImm8 { dst, imm } => {
                    write_u64(fp, dst, imm);
                },

                MicroOp::SubU64Imm { dst, src, imm } => {
                    let v = read_u64(fp, src);
                    let result = match v.checked_sub(imm) {
                        Some(v) => v,
                        None => bail!("arithmetic underflow"),
                    };
                    write_u64(fp, dst, result);
                },

                MicroOp::AddU64 { dst, lhs, rhs } => {
                    let v1 = read_u64(fp, lhs);
                    let v2 = read_u64(fp, rhs);
                    let result = match v1.checked_add(v2) {
                        Some(v) => v,
                        None => bail!("arithmetic overflow"),
                    };
                    write_u64(fp, dst, result);
                },

                MicroOp::AddU64Imm { dst, src, imm } => {
                    let v = read_u64(fp, src);
                    let result = match v.checked_add(imm) {
                        Some(v) => v,
                        None => bail!("arithmetic overflow"),
                    };
                    write_u64(fp, dst, result);
                },

                MicroOp::RSubU64Imm { dst, src, imm } => {
                    let v = read_u64(fp, src);
                    let result = match imm.checked_sub(v) {
                        Some(v) => v,
                        None => bail!("arithmetic underflow"),
                    };
                    write_u64(fp, dst, result);
                },

                MicroOp::XorU64 { dst, lhs, rhs } => {
                    let lhs_val = read_u64(fp, lhs);
                    let rhs_val = read_u64(fp, rhs);
                    write_u64(fp, dst, lhs_val ^ rhs_val);
                },

                MicroOp::ShrU64Imm { dst, src, imm } => {
                    if imm > 63 {
                        bail!("ShrU64Imm: shift amount {} exceeds 63", imm);
                    }
                    let v = read_u64(fp, src);
                    write_u64(fp, dst, v >> imm);
                },

                MicroOp::ModU64 { dst, lhs, rhs } => {
                    let lhs_val = read_u64(fp, lhs);
                    let rhs_val = read_u64(fp, rhs);
                    if rhs_val == 0 {
                        bail!("ModU64: division by zero");
                    }
                    write_u64(fp, dst, lhs_val % rhs_val);
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
                    let ref_base = read_ptr(fp, vec_ref);
                    let ref_off = read_u64(fp, vec_ref + 8) as usize;
                    let vec_ptr = read_ptr(ref_base, ref_off);
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
                    let ref_base = read_ptr(fp, vec_ref);
                    let ref_off = read_u64(fp, vec_ref + 8) as usize;
                    let mut vec_ptr = read_ptr(ref_base, ref_off);

                    if vec_ptr.is_null() {
                        vec_ptr = alloc_vec!(self, fp, descriptor_id, elem_size, 4)?;
                        // Re-read base after potential GC.
                        let ref_base = read_ptr(fp, vec_ref);
                        let ref_off = read_u64(fp, vec_ref + 8) as usize;
                        write_ptr(ref_base, ref_off, vec_ptr);
                    }

                    let len = read_u64(vec_ptr, VEC_LENGTH_OFFSET);
                    let size = read_u32(vec_ptr, HEADER_SIZE_OFFSET) as usize;
                    let cap = ((size - VEC_DATA_OFFSET) / elem_size as usize) as u64;

                    if len >= cap {
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
                    let ref_base = read_ptr(fp, vec_ref);
                    let ref_off = read_u64(fp, vec_ref + 8) as usize;
                    let vec_ptr = read_ptr(ref_base, ref_off);
                    if vec_ptr.is_null() {
                        bail!("VecPopBack on empty vector");
                    }
                    let len = read_u64(vec_ptr, VEC_LENGTH_OFFSET);
                    if len == 0 {
                        bail!("VecPopBack on empty vector");
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
                    let ref_base = read_ptr(fp, vec_ref);
                    let ref_off = read_u64(fp, vec_ref + 8) as usize;
                    let vec_ptr = read_ptr(ref_base, ref_off);
                    let idx_val = read_u64(fp, idx);
                    if vec_ptr.is_null() {
                        bail!("VecLoadElem index out of bounds: idx={} len=0", idx_val,);
                    }
                    let len = read_u64(vec_ptr, VEC_LENGTH_OFFSET);
                    if idx_val >= len {
                        bail!(
                            "VecLoadElem index out of bounds: idx={} len={}",
                            idx_val,
                            len
                        );
                    }
                    std::ptr::copy_nonoverlapping(
                        vec_elem_ptr(vec_ptr, idx_val, elem_size),
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
                    let ref_base = read_ptr(fp, vec_ref);
                    let ref_off = read_u64(fp, vec_ref + 8) as usize;
                    let vec_ptr = read_ptr(ref_base, ref_off);
                    let idx_val = read_u64(fp, idx);
                    if vec_ptr.is_null() {
                        bail!("VecStoreElem index out of bounds: idx={} len=0", idx_val,);
                    }
                    let len = read_u64(vec_ptr, VEC_LENGTH_OFFSET);
                    if idx_val >= len {
                        bail!(
                            "VecStoreElem index out of bounds: idx={} len={}",
                            idx_val,
                            len
                        );
                    }
                    std::ptr::copy_nonoverlapping(
                        fp.add(src.into()),
                        vec_elem_ptr(vec_ptr, idx_val, elem_size) as *mut u8,
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
                    let ref_base = read_ptr(fp, vec_ref);
                    let ref_off = read_u64(fp, vec_ref + 8) as usize;
                    let vec_ptr = read_ptr(ref_base, ref_off);
                    let idx_val = read_u64(fp, idx);
                    if vec_ptr.is_null() {
                        bail!("VecBorrow index out of bounds: idx={} len=0", idx_val);
                    }
                    let len = read_u64(vec_ptr, VEC_LENGTH_OFFSET);
                    if idx_val >= len {
                        bail!("VecBorrow index out of bounds: idx={} len={}", idx_val, len);
                    }
                    let offset = VEC_DATA_OFFSET as u64 + idx_val * elem_size as u64;
                    write_ptr(fp, dst, vec_ptr);
                    write_u64(fp, dst + 8, offset);
                },

                MicroOp::SlotBorrow { dst, local } => {
                    write_ptr(fp, dst, fp.add(local.into()));
                    write_u64(fp, dst + 8, 0);
                },

                MicroOp::ReadRef { dst, ref_ptr, size } => {
                    let base = read_ptr(fp, ref_ptr);
                    let offset = read_u64(fp, ref_ptr + 8);
                    let target = base.add(offset as usize);
                    std::ptr::copy_nonoverlapping(target, fp.add(dst.into()), size as usize);
                },

                MicroOp::WriteRef { ref_ptr, src, size } => {
                    let base = read_ptr(fp, ref_ptr);
                    let offset = read_u64(fp, ref_ptr + 8);
                    let target = base.add(offset as usize);
                    std::ptr::copy_nonoverlapping(fp.add(src.into()), target, size as usize);
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
                    let ref_base = read_ptr(fp, obj_ref);
                    let ref_off = read_u64(fp, obj_ref + 8) as usize;
                    let obj_ptr = read_ptr(ref_base, ref_off);
                    // Unlike vectors, heap objects are never null — they
                    // are always allocated by HeapNew before being borrowed.
                    debug_assert!(!obj_ptr.is_null(), "HeapBorrow: null object pointer");
                    write_ptr(fp, dst, obj_ptr);
                    write_u64(fp, dst + 8, offset as u64);
                },

                MicroOp::Charge { cost } => {
                    self.gas_meter.charge(cost)?;
                },

                MicroOp::PackClosure(ref op) => {
                    self.exec_pack_closure(fp, op)?;
                },
                MicroOp::CallClosure(ref op) => {
                    return self.exec_call_closure(func, fp, op);
                },
            }
        }

        self.pc += 1;
        Ok(StepResult::Continue)
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
    unsafe fn exec_pack_closure(&mut self, fp: *mut u8, op: &PackClosureOp) -> ExecutionResult<()> {
        unsafe {
            // Fast path: non-capturing closure. Skip the second allocation
            // and leave `captured_data_ptr` as the zeroed/null value written
            // by `alloc_obj`. No pinning needed — only one allocation.
            if op.captured.is_empty() {
                let closure = alloc_obj!(self, fp, op.closure_descriptor_id)?;
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
            let closure_ptr = alloc_obj!(self, fp, op.closure_descriptor_id)?;
            let pin = self.pinned_roots.pin(NonNull::new_unchecked(closure_ptr));

            self.write_closure_func_ref_and_mask(pin.get().as_ptr(), op);

            let captured_data = alloc_obj!(self, fp, op.captured_data_descriptor_id)?;
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
    ) -> ExecutionResult<StepResult> {
        unsafe {
            let closure = read_ptr(fp, op.closure_src);
            if closure.is_null() {
                bail!("CallClosure: null closure pointer");
            }

            // Decode `ClosureFuncRef`. v0 supports only Resolved.
            let func_tag = *closure.add(CLOSURE_FUNC_REF_OFFSET + FUNC_REF_TAG_OFFSET);
            if func_tag != FUNC_REF_TAG_RESOLVED {
                bail!(
                    "CallClosure: unsupported func_ref tag {} (only Resolved supported in v0)",
                    func_tag
                );
            }
            let callee_raw = read_ptr(closure, CLOSURE_FUNC_REF_OFFSET + FUNC_REF_PAYLOAD_OFFSET)
                as *const Function;
            if callee_raw.is_null() {
                bail!("CallClosure: null function pointer in closure");
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
            let param_sizes = callee.param_sizes.as_ref_unchecked();
            if param_sizes.len() > 64 {
                bail!(
                    "CallClosure: callee has {} params, exceeds 64-bit mask capacity",
                    param_sizes.len()
                );
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
                    bail!("CallClosure: null captured_data for closure with captured params");
                }
                let cap_tag = *captured_data.add(CAPTURED_DATA_TAG_OFFSET);
                if cap_tag != CAPTURED_DATA_TAG_MATERIALIZED {
                    bail!(
                        "CallClosure: unsupported captured-data tag {} (only Materialized supported in v0)",
                        cap_tag
                    );
                }
            }

            let mut captured_value_offset = CAPTURED_DATA_VALUES_OFFSET;
            let mut provided_idx = 0usize;
            let mut param_offset_in_callee = 0usize;
            for (i, &param_size) in param_sizes.iter().enumerate() {
                let is_captured = (mask >> i) & 1 != 0;
                if is_captured {
                    std::ptr::copy_nonoverlapping(
                        captured_data.add(captured_value_offset),
                        new_fp.add(param_offset_in_callee),
                        param_size as usize,
                    );
                    captured_value_offset += param_size as usize;
                } else {
                    let slot: &SizedSlot = op
                        .provided_args
                        .get(provided_idx)
                        .ok_or_else(|| anyhow::anyhow!("CallClosure: not enough provided args"))?;
                    if slot.size != param_size {
                        bail!(
                            "CallClosure: provided_args[{}].size {} != callee param_sizes[{}] {}",
                            provided_idx,
                            slot.size,
                            i,
                            param_size
                        );
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
            if provided_idx != op.provided_args.len() {
                bail!(
                    "CallClosure: {} provided_args but only {} non-captured params consumed",
                    op.provided_args.len(),
                    provided_idx
                );
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
    ) -> ExecutionResult<*mut u8> {
        unsafe {
            let new_fp = fp.add(caller.param_and_local_sizes_sum + FRAME_METADATA_SIZE);
            let stack_end = self.stack.as_ptr().add(self.stack.len());
            if new_fp.add(callee.extended_frame_size) > stack_end {
                bail!("stack overflow");
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
    ) -> ExecutionResult<StepResult> {
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
    ) -> ExecutionResult<StepResult> {
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
    pub fn run(&mut self) -> ExecutionResult<()> {
        loop {
            match self.step()? {
                StepResult::Continue => {},
                StepResult::Done => return Ok(()),
            }
        }
    }
}
