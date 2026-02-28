// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Interpreter with unified stack (frame metadata inlined in linear memory)
//! and a bump-allocated heap with copying GC.

use crate::{
    heap::Heap, read_ptr, read_u64, vec_elem_ptr, write_ptr, write_u64, Function, Instruction,
    MemoryRegion, ObjectDescriptor, StepResult, DEFAULT_HEAP_SIZE, DEFAULT_STACK_SIZE,
    FRAME_METADATA_SIZE, SENTINEL_FUNC_ID, STRUCT_DATA_OFFSET, VEC_CAPACITY_OFFSET,
    VEC_DATA_OFFSET, VEC_LENGTH_OFFSET,
};
use anyhow::{bail, Result};
use rand::{rngs::StdRng, Rng, SeedableRng};

// ---------------------------------------------------------------------------
// Runtime state
// ---------------------------------------------------------------------------

/// Interpreter context with a unified call stack and a GC-managed heap.
///
/// `frame_ptr` (fp) is an absolute pointer into the linear stack memory, so
/// operand accesses are a single addition (`fp + offset`). It is recomputed
/// only on `CallFunc` and `Return`.
pub struct InterpreterContext<'a> {
    pub(crate) functions: &'a [Function],
    pub(crate) descriptors: &'a [ObjectDescriptor],

    pub(crate) pc: usize,
    pub(crate) func_id: usize,
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
        args: &[u8],
    ) -> Self {
        Self::with_heap_size(functions, descriptors, func_id, args, DEFAULT_HEAP_SIZE)
    }

    /// Create a new context with a custom heap size (for testing GC pressure).
    pub fn with_heap_size(
        functions: &'a [Function],
        descriptors: &'a [ObjectDescriptor],
        func_id: usize,
        args: &[u8],
        heap_size: usize,
    ) -> Self {
        assert!(
            func_id < functions.len(),
            "entry func_id {} is out of bounds (have {} functions)",
            func_id,
            functions.len()
        );
        assert!(
            args.len() <= functions[func_id].data_size as usize,
            "args length ({}) exceeds root function's data_size ({})",
            args.len(),
            functions[func_id].data_size
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
            write_u64(base, 0, 0); // saved_pc  (unused)
            write_u64(base, 8, 0); // saved_fp  (unused)
            write_u64(base, 16, SENTINEL_FUNC_ID);
            std::ptr::copy_nonoverlapping(args.as_ptr(), frame_ptr, args.len());
        }

        Self {
            functions,
            descriptors,
            pc: 0,
            func_id,
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

    /// Read a u64 from the root frame's slot 0 (where the result lands).
    pub fn root_result(&self) -> u64 {
        unsafe { read_u64(self.stack.as_ptr(), FRAME_METADATA_SIZE) }
    }

    /// Read a raw heap pointer from the root frame at the given byte offset.
    pub fn root_heap_ptr(&self, offset: u32) -> *const u8 {
        unsafe { read_ptr(self.stack.as_ptr(), FRAME_METADATA_SIZE + offset as usize) }
    }
}

// ---------------------------------------------------------------------------
// Interpreter loop
// ---------------------------------------------------------------------------

impl InterpreterContext<'_> {
    pub fn step(&mut self) -> Result<StepResult> {
        let func = &self.functions[self.func_id];
        if self.pc >= func.code.len() {
            bail!(
                "pc out of bounds: pc={} but function {} has {} instructions",
                self.pc,
                self.func_id,
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
                Instruction::CallFunc { func_id } => {
                    let meta = fp.add(func.data_size as usize);
                    write_u64(meta, 0, (self.pc + 1) as u64);
                    write_ptr(meta, 8, fp);
                    write_u64(meta, 16, self.func_id as u64);
                    self.frame_ptr = meta.add(FRAME_METADATA_SIZE);
                    self.pc = 0;
                    self.func_id = func_id;
                    return Ok(StepResult::Continue);
                },

                Instruction::JumpIfNotZero {
                    src_fp_offset,
                    dst_pc,
                } => {
                    self.pc = if read_u64(fp, src_fp_offset as usize) != 0 {
                        dst_pc as usize
                    } else {
                        self.pc + 1
                    };
                    return Ok(StepResult::Continue);
                },

                Instruction::JumpIfGreaterEqualU64Const {
                    src_fp_offset,
                    dst_pc,
                    val,
                } => {
                    self.pc = if read_u64(fp, src_fp_offset as usize) >= val {
                        dst_pc as usize
                    } else {
                        self.pc + 1
                    };
                    return Ok(StepResult::Continue);
                },

                Instruction::JumpIfLessU64 {
                    lhs_fp_offset,
                    rhs_fp_offset,
                    dst_pc,
                } => {
                    self.pc = if read_u64(fp, lhs_fp_offset as usize)
                        < read_u64(fp, rhs_fp_offset as usize)
                    {
                        dst_pc as usize
                    } else {
                        self.pc + 1
                    };
                    return Ok(StepResult::Continue);
                },

                Instruction::Jump { dst_pc } => {
                    self.pc = dst_pc as usize;
                    return Ok(StepResult::Continue);
                },

                Instruction::Return => {
                    let meta = fp.sub(FRAME_METADATA_SIZE);
                    let saved_func_id = read_u64(meta, 16);
                    if saved_func_id == SENTINEL_FUNC_ID {
                        return Ok(StepResult::Done);
                    }
                    self.pc = read_u64(meta, 0) as usize;
                    self.frame_ptr = read_ptr(meta, 8);
                    self.func_id = saved_func_id as usize;
                    return Ok(StepResult::Continue);
                },

                // ----- Arithmetic -----
                Instruction::StoreU64 { dst_fp_offset, val } => {
                    write_u64(fp, dst_fp_offset as usize, val);
                },

                Instruction::SubU64Const {
                    src_fp_offset,
                    val,
                    dst_fp_offset,
                } => {
                    let v = read_u64(fp, src_fp_offset as usize);
                    let result = v.checked_sub(val).expect("arithmetic underflow");
                    write_u64(fp, dst_fp_offset as usize, result);
                },

                Instruction::AddU64 {
                    src_fp_offset_1,
                    src_fp_offset_2,
                    dst_fp_offset,
                } => {
                    let v1 = read_u64(fp, src_fp_offset_1 as usize);
                    let v2 = read_u64(fp, src_fp_offset_2 as usize);
                    let result = v1.checked_add(v2).expect("arithmetic overflow");
                    write_u64(fp, dst_fp_offset as usize, result);
                },

                Instruction::AddU64Const {
                    src_fp_offset,
                    val,
                    dst_fp_offset,
                } => {
                    let v = read_u64(fp, src_fp_offset as usize);
                    let result = v.checked_add(val).expect("arithmetic overflow");
                    write_u64(fp, dst_fp_offset as usize, result);
                },

                Instruction::ShrU64Const {
                    src_fp_offset,
                    val,
                    dst_fp_offset,
                } => {
                    let v = read_u64(fp, src_fp_offset as usize);
                    write_u64(fp, dst_fp_offset as usize, v >> val);
                },

                Instruction::RemU64 {
                    lhs_fp_offset,
                    rhs_fp_offset,
                    dst_fp_offset,
                } => {
                    let lhs = read_u64(fp, lhs_fp_offset as usize);
                    let rhs = read_u64(fp, rhs_fp_offset as usize);
                    assert!(rhs != 0, "RemU64: division by zero");
                    write_u64(fp, dst_fp_offset as usize, lhs % rhs);
                },

                Instruction::StoreRandomU64 { dst_fp_offset } => {
                    let val: u64 = self.rng.r#gen();
                    write_u64(fp, dst_fp_offset as usize, val);
                },

                Instruction::ForceGC => {
                    self.gc_collect()?;
                },

                Instruction::Mov8 {
                    src_fp_offset,
                    dst_fp_offset,
                } => {
                    let v = read_u64(fp, src_fp_offset as usize);
                    write_u64(fp, dst_fp_offset as usize, v);
                },

                Instruction::Mov {
                    src_fp_offset,
                    dst_fp_offset,
                    size,
                } => {
                    std::ptr::copy(
                        fp.add(src_fp_offset as usize),
                        fp.add(dst_fp_offset as usize),
                        size as usize,
                    );
                },

                // ----- Vector instructions -----
                Instruction::VecNew {
                    descriptor_id,
                    elem_size,
                    initial_capacity,
                    dst_fp_offset,
                } => {
                    let ptr = self.alloc_vec(descriptor_id, elem_size, initial_capacity)?;
                    write_ptr(fp, dst_fp_offset as usize, ptr);
                },

                Instruction::VecLen {
                    vec_fp_offset,
                    dst_fp_offset,
                } => {
                    let vec_ptr = read_ptr(fp, vec_fp_offset as usize);
                    let len = read_u64(vec_ptr, VEC_LENGTH_OFFSET);
                    write_u64(fp, dst_fp_offset as usize, len);
                },

                Instruction::VecPushBack {
                    vec_fp_offset,
                    elem_fp_offset,
                    elem_size,
                } => {
                    let vec_slot = fp.add(vec_fp_offset as usize) as *mut u64;
                    let mut vec_ptr = read_ptr(fp, vec_fp_offset as usize);

                    let len = read_u64(vec_ptr, VEC_LENGTH_OFFSET);
                    let cap = read_u64(vec_ptr, VEC_CAPACITY_OFFSET);

                    if len >= cap {
                        vec_ptr = self.grow_vec(vec_slot, elem_size, len + 1)?;
                    }

                    std::ptr::copy_nonoverlapping(
                        fp.add(elem_fp_offset as usize),
                        vec_elem_ptr(vec_ptr, len, elem_size) as *mut u8,
                        elem_size as usize,
                    );
                    write_u64(vec_ptr, VEC_LENGTH_OFFSET, len + 1);
                },

                Instruction::VecPopBack {
                    vec_fp_offset,
                    dst_fp_offset,
                    elem_size,
                } => {
                    let vec_ptr = read_ptr(fp, vec_fp_offset as usize);
                    let len = read_u64(vec_ptr, VEC_LENGTH_OFFSET);
                    if len == 0 {
                        bail!("VecPopBack on empty vector");
                    }
                    let new_len = len - 1;
                    std::ptr::copy_nonoverlapping(
                        vec_elem_ptr(vec_ptr, new_len, elem_size),
                        fp.add(dst_fp_offset as usize),
                        elem_size as usize,
                    );
                    write_u64(vec_ptr, VEC_LENGTH_OFFSET, new_len);
                },

                Instruction::VecLoadElem {
                    vec_fp_offset,
                    idx_fp_offset,
                    dst_fp_offset,
                    elem_size,
                } => {
                    let vec_ptr = read_ptr(fp, vec_fp_offset as usize);
                    let idx = read_u64(fp, idx_fp_offset as usize);
                    let len = read_u64(vec_ptr, VEC_LENGTH_OFFSET);
                    if idx >= len {
                        bail!("VecLoadElem index out of bounds: idx={} len={}", idx, len);
                    }
                    std::ptr::copy_nonoverlapping(
                        vec_elem_ptr(vec_ptr, idx, elem_size),
                        fp.add(dst_fp_offset as usize),
                        elem_size as usize,
                    );
                },

                Instruction::VecStoreElem {
                    vec_fp_offset,
                    idx_fp_offset,
                    src_fp_offset,
                    elem_size,
                } => {
                    let vec_ptr = read_ptr(fp, vec_fp_offset as usize);
                    let idx = read_u64(fp, idx_fp_offset as usize);
                    let len = read_u64(vec_ptr, VEC_LENGTH_OFFSET);
                    if idx >= len {
                        bail!("VecStoreElem index out of bounds: idx={} len={}", idx, len);
                    }
                    std::ptr::copy_nonoverlapping(
                        fp.add(src_fp_offset as usize),
                        vec_elem_ptr(vec_ptr, idx, elem_size) as *mut u8,
                        elem_size as usize,
                    );
                },

                // ----- Reference (fat pointer) instructions -----
                Instruction::VecBorrow {
                    vec_fp_offset,
                    idx_fp_offset,
                    elem_size,
                    dst_fp_offset,
                } => {
                    let vec_ptr = read_ptr(fp, vec_fp_offset as usize);
                    let idx = read_u64(fp, idx_fp_offset as usize);
                    let len = read_u64(vec_ptr, VEC_LENGTH_OFFSET);
                    if idx >= len {
                        bail!("VecBorrow index out of bounds: idx={} len={}", idx, len);
                    }
                    let offset = VEC_DATA_OFFSET as u64 + idx * elem_size as u64;
                    write_ptr(fp, dst_fp_offset as usize, vec_ptr);
                    write_u64(fp, (dst_fp_offset + 8) as usize, offset);
                },

                Instruction::BorrowLocal {
                    local_fp_offset,
                    dst_fp_offset,
                } => {
                    write_ptr(fp, dst_fp_offset as usize, fp.add(local_fp_offset as usize));
                    write_u64(fp, (dst_fp_offset + 8) as usize, 0);
                },

                Instruction::ReadRef {
                    ref_fp_offset,
                    dst_fp_offset,
                    size,
                } => {
                    let base = read_ptr(fp, ref_fp_offset as usize);
                    let offset = read_u64(fp, (ref_fp_offset + 8) as usize);
                    let target = base.add(offset as usize);
                    std::ptr::copy_nonoverlapping(
                        target,
                        fp.add(dst_fp_offset as usize),
                        size as usize,
                    );
                },

                Instruction::WriteRef {
                    ref_fp_offset,
                    src_fp_offset,
                    size,
                } => {
                    let base = read_ptr(fp, ref_fp_offset as usize);
                    let offset = read_u64(fp, (ref_fp_offset + 8) as usize);
                    let target = base.add(offset as usize);
                    std::ptr::copy_nonoverlapping(
                        fp.add(src_fp_offset as usize),
                        target,
                        size as usize,
                    );
                },

                // ----- Struct instructions -----
                Instruction::StructNew {
                    descriptor_id,
                    dst_fp_offset,
                } => {
                    let ptr = self.alloc_struct(descriptor_id)?;
                    write_ptr(fp, dst_fp_offset as usize, ptr);
                },

                Instruction::StructLoadField {
                    struct_fp_offset,
                    field_offset,
                    dst_fp_offset,
                    size,
                } => {
                    let struct_ptr = read_ptr(fp, struct_fp_offset as usize);
                    std::ptr::copy_nonoverlapping(
                        struct_ptr.add(STRUCT_DATA_OFFSET + field_offset as usize),
                        fp.add(dst_fp_offset as usize),
                        size as usize,
                    );
                },

                Instruction::StructStoreField {
                    struct_fp_offset,
                    field_offset,
                    src_fp_offset,
                    size,
                } => {
                    let struct_ptr = read_ptr(fp, struct_fp_offset as usize);
                    std::ptr::copy_nonoverlapping(
                        fp.add(src_fp_offset as usize),
                        struct_ptr.add(STRUCT_DATA_OFFSET + field_offset as usize),
                        size as usize,
                    );
                },

                Instruction::StructBorrow {
                    struct_fp_offset,
                    field_offset,
                    dst_fp_offset,
                } => {
                    let struct_ptr = read_ptr(fp, struct_fp_offset as usize);
                    write_ptr(fp, dst_fp_offset as usize, struct_ptr);
                    write_u64(
                        fp,
                        (dst_fp_offset + 8) as usize,
                        (STRUCT_DATA_OFFSET + field_offset as usize) as u64,
                    );
                },
            }
        }

        self.pc += 1;
        Ok(StepResult::Continue)
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            match self.step()? {
                StepResult::Continue => {},
                StepResult::Done => return Ok(()),
            }
        }
    }
}
