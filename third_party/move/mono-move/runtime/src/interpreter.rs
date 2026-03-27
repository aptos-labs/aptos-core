// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Interpreter with unified stack (frame metadata inlined in linear memory)
//! and a bump-allocated heap with copying GC.

use crate::{
    heap::Heap,
    memory::{read_ptr, read_u32, read_u64, vec_elem_ptr, write_ptr, write_u64, MemoryRegion},
    types::{
        ObjectDescriptor, StepResult, DEFAULT_HEAP_SIZE, DEFAULT_STACK_SIZE, HEADER_SIZE_OFFSET,
        META_SAVED_FP_OFFSET, META_SAVED_FUNC_PTR_OFFSET, META_SAVED_PC_OFFSET, VEC_DATA_OFFSET,
        VEC_LENGTH_OFFSET,
    },
};
use anyhow::{bail, Result};
use mono_move_core::{DescriptorId, Function, MicroOp, FRAME_METADATA_SIZE};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::ptr::{null, NonNull};

// ---------------------------------------------------------------------------
// Runtime state
// ---------------------------------------------------------------------------

/// Interpreter context with a unified call stack and a GC-managed heap.
pub struct InterpreterContext<'a> {
    /// Externally-provided function table (will be replaced by execution context).
    pub(crate) functions: &'a [Function],
    /// Externally-provided object layout descriptors (will be replaced by execution context).
    pub(crate) descriptors: &'a [ObjectDescriptor],

    pub(crate) pc: usize,
    /// Pointer to the currently executing function.
    pub(crate) current_func: NonNull<Function>,
    /// Absolute pointer into the linear stack memory. Operand accesses are a
    /// single addition (`fp + offset`). Recomputed only on `CallFunc` and `Return`.
    pub(crate) frame_ptr: *mut u8,

    pub(crate) stack: MemoryRegion,
    pub(crate) heap: Heap,
    rng: StdRng,
}

impl<'a> InterpreterContext<'a> {
    pub fn new(
        functions: &'a [Function],
        descriptors: &'a [ObjectDescriptor],
        func_id: usize,
    ) -> Self {
        Self::with_heap_size(functions, descriptors, func_id, DEFAULT_HEAP_SIZE)
    }

    /// Create a new context with a custom heap size (for testing GC pressure).
    pub fn with_heap_size(
        functions: &'a [Function],
        descriptors: &'a [ObjectDescriptor],
        func_id: usize,
        heap_size: usize,
    ) -> Self {
        assert!(
            func_id < functions.len(),
            "entry func_id {} is out of bounds (have {} functions)",
            func_id,
            functions.len()
        );

        let verification_errors = crate::verifier::verify_program(functions, descriptors);
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
            functions,
            descriptors,
            pc: 0,
            current_func: NonNull::from(&functions[func_id]),
            frame_ptr,
            stack,
            heap: Heap::new(heap_size),
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
    pub fn invoke(&mut self, func_id: usize) {
        assert!(
            func_id < self.functions.len(),
            "func_id {} out of bounds (have {} functions)",
            func_id,
            self.functions.len()
        );

        let func = &self.functions[func_id];
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

        // Zero everything beyond args (locals, metadata, callee arg/return
        // region) so pointer slots start as null.
        if func.zero_frame {
            unsafe {
                std::ptr::write_bytes(
                    self.frame_ptr.add(func.args_size),
                    0,
                    func.extended_frame_size - func.args_size,
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
    pub fn alloc_u64_vec(&mut self, descriptor_id: DescriptorId, values: &[u64]) -> Result<u64> {
        let n = values.len() as u64;
        let ptr = self.alloc_vec(descriptor_id, 8, n)?;
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

impl InterpreterContext<'_> {
    #[inline(always)]
    pub fn step(&mut self) -> Result<StepResult> {
        // SAFETY: current_func is always a valid, non-null pointer either
        // derived from `self.functions[]` or from a `CallLocalFunc` pointer
        // (which is itself non-null and a valid reference).
        let func = unsafe { self.current_func.as_ref() };
        if self.pc >= func.code.len() {
            bail!(
                "pc out of bounds: pc={} but function {} has {} instructions",
                self.pc,
                unsafe { func.name.as_ref_unchecked() },
                func.code.len()
            );
        }

        let fp = self.frame_ptr;
        let instr = &func.code[self.pc];

        // SAFETY: fp points into the interpreter's linear stack; all byte
        // offsets are within the current frame (enforced by the bytecode
        // compiler). Heap pointers read from the frame are kept valid by GC.
        unsafe {
            match *instr {
                // ----- Control flow (set pc explicitly, return early) -----
                MicroOp::CallFunc { func_id } => {
                    let func_id = func_id as usize;
                    let callee = &self.functions[func_id];
                    return self.call(func, fp, callee);
                },
                MicroOp::CallLocalFunc { ptr } => {
                    return self.call(func, fp, ptr.as_ref());
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
                    self.gc_collect()?;
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
                        vec_ptr = self.alloc_vec(descriptor_id, elem_size, 4)?;
                        // Re-read base after potential GC.
                        let ref_base = read_ptr(fp, vec_ref);
                        let ref_off = read_u64(fp, vec_ref + 8) as usize;
                        write_ptr(ref_base, ref_off, vec_ptr);
                    }

                    let len = read_u64(vec_ptr, VEC_LENGTH_OFFSET);
                    let size = read_u32(vec_ptr, HEADER_SIZE_OFFSET) as usize;
                    let cap = ((size - VEC_DATA_OFFSET) / elem_size as usize) as u64;

                    if len >= cap {
                        vec_ptr = self.grow_vec_ref(fp, vec_ref.into(), elem_size, len + 1)?;
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
                    let ptr = self.alloc_obj(descriptor_id)?;
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

                MicroOp::Charge { .. } => {
                    // TODO: wire up to a GasMeter once the runtime carries one.
                },
            }
        }

        self.pc += 1;
        Ok(StepResult::Continue)
    }

    /// Implementation of call opcodes.
    ///
    /// # Safety
    ///
    /// `callee` must point to a valid, live `Function`. `fp` must be the
    /// current frame pointer and `func` the currently executing function.
    #[inline(always)]
    unsafe fn call(
        &mut self,
        caller: &Function,
        fp: *mut u8,
        callee: &Function,
    ) -> Result<StepResult> {
        unsafe {
            let new_fp = fp.add(caller.args_and_locals_size + FRAME_METADATA_SIZE);
            let stack_end = self.stack.as_ptr().add(self.stack.len());
            if new_fp.add(callee.extended_frame_size) > stack_end {
                bail!("stack overflow");
            }
            // Zero everything beyond args (locals, metadata, callee
            // arg/return region) so pointer slots start as null.
            // The argument region (0..args_size) was already written
            // by the caller.
            if callee.zero_frame {
                let zero_size = callee.extended_frame_size - callee.args_size;
                std::ptr::write_bytes(new_fp.add(callee.args_size), 0, zero_size);
            }
            let meta = fp.add(caller.args_and_locals_size);
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
    pub fn run(&mut self) -> Result<()> {
        loop {
            match self.step()? {
                StepResult::Continue => {},
                StepResult::Done => return Ok(()),
            }
        }
    }
}
