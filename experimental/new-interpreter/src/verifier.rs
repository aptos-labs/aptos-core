// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Static verifier for `Function` bodies. Checks well-formedness properties
//! that would otherwise cause undefined behavior at runtime: frame bounds,
//! missing stack maps at GC safe points, invalid jump targets, etc.

use crate::{Function, Instruction, ObjectDescriptor, FRAME_METADATA_SIZE};
use std::fmt;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct VerificationError {
    pub func_id: usize,
    pub pc: Option<usize>,
    pub message: String,
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.pc {
            Some(pc) => write!(f, "func {}, pc {}: {}", self.func_id, pc, self.message),
            None => write!(f, "func {}: {}", self.func_id, self.message),
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Validate all functions and their stack maps against the descriptor table.
/// Returns an empty `Vec` on success.
pub fn verify_program(
    functions: &[Function],
    descriptors: &[ObjectDescriptor],
) -> Vec<VerificationError> {
    let mut errors = Vec::new();
    for (fid, func) in functions.iter().enumerate() {
        let mut fv = FunctionVerifier {
            func_id: fid,
            func,
            all_functions: functions,
            descriptors,
            errors: &mut errors,
        };
        fv.verify();
    }
    errors
}

// ---------------------------------------------------------------------------
// Per-function verifier — holds shared state so helpers don't need many args
// ---------------------------------------------------------------------------

struct FunctionVerifier<'a> {
    func_id: usize,
    func: &'a Function,
    all_functions: &'a [Function],
    descriptors: &'a [ObjectDescriptor],
    errors: &'a mut Vec<VerificationError>,
}

impl FunctionVerifier<'_> {
    fn verify(&mut self) {
        if self.func.code.is_empty() {
            self.err(None, "code must be non-empty");
        }
        if self.func.frame_size() > self.func.extended_frame_size {
            self.err(
                None,
                format!(
                    "extended_frame_size ({}) must be >= frame_size() (data_size {} + FRAME_METADATA_SIZE {} = {})",
                    self.func.extended_frame_size,
                    self.func.data_size,
                    FRAME_METADATA_SIZE,
                    self.func.frame_size()
                ),
            );
        }

        for (&pc, offsets) in &self.func.stack_maps {
            if pc >= self.func.code.len() {
                self.err(
                    Some(pc),
                    format!(
                        "stack map PC {} is out of bounds (code length {})",
                        pc,
                        self.func.code.len()
                    ),
                );
            }
            for &off in offsets {
                self.check_access(Some(pc), "stack map offset", off, 8);
            }
        }

        for (pc, instr) in self.func.code.iter().enumerate() {
            self.verify_instruction(pc, instr);
        }
    }

    fn verify_instruction(&mut self, pc: usize, instr: &Instruction) {
        match *instr {
            // ----- Arithmetic / data movement (8-byte accesses) -----
            Instruction::StoreU64 { dst_fp_offset, .. } => {
                self.check8(pc, "dst", dst_fp_offset);
            },

            Instruction::SubU64Const {
                src_fp_offset,
                dst_fp_offset,
                ..
            } => {
                self.check8(pc, "src", src_fp_offset);
                self.check8(pc, "dst", dst_fp_offset);
            },

            Instruction::AddU64 {
                src_fp_offset_1,
                src_fp_offset_2,
                dst_fp_offset,
            } => {
                self.check8(pc, "src1", src_fp_offset_1);
                self.check8(pc, "src2", src_fp_offset_2);
                self.check8(pc, "dst", dst_fp_offset);
            },

            Instruction::AddU64Const {
                src_fp_offset,
                dst_fp_offset,
                ..
            } => {
                self.check8(pc, "src", src_fp_offset);
                self.check8(pc, "dst", dst_fp_offset);
            },

            Instruction::ShrU64Const {
                src_fp_offset,
                dst_fp_offset,
                ..
            } => {
                self.check8(pc, "src", src_fp_offset);
                self.check8(pc, "dst", dst_fp_offset);
            },

            Instruction::RemU64 {
                lhs_fp_offset,
                rhs_fp_offset,
                dst_fp_offset,
            } => {
                self.check8(pc, "lhs", lhs_fp_offset);
                self.check8(pc, "rhs", rhs_fp_offset);
                self.check8(pc, "dst", dst_fp_offset);
            },

            Instruction::StoreRandomU64 { dst_fp_offset } => {
                self.check8(pc, "dst", dst_fp_offset);
            },

            Instruction::Mov8 {
                src_fp_offset,
                dst_fp_offset,
            } => {
                self.check8(pc, "src", src_fp_offset);
                self.check8(pc, "dst", dst_fp_offset);
            },

            Instruction::Mov {
                src_fp_offset,
                dst_fp_offset,
                size,
            } => {
                self.check_nonzero_size(pc, size);
                self.check_access(Some(pc), "src", src_fp_offset, size);
                self.check_access(Some(pc), "dst", dst_fp_offset, size);
            },

            // ----- Control flow -----
            Instruction::Jump { dst_pc } => {
                self.check_jump(pc, dst_pc);
            },

            Instruction::JumpIfNotZero {
                src_fp_offset,
                dst_pc,
            } => {
                self.check8(pc, "src", src_fp_offset);
                self.check_jump(pc, dst_pc);
            },

            Instruction::JumpIfGreaterEqualU64Const {
                src_fp_offset,
                dst_pc,
                ..
            } => {
                self.check8(pc, "src", src_fp_offset);
                self.check_jump(pc, dst_pc);
            },

            Instruction::JumpIfLessU64 {
                lhs_fp_offset,
                rhs_fp_offset,
                dst_pc,
            } => {
                self.check8(pc, "lhs", lhs_fp_offset);
                self.check8(pc, "rhs", rhs_fp_offset);
                self.check_jump(pc, dst_pc);
            },

            Instruction::Return => {},

            Instruction::CallFunc { func_id } => {
                if func_id >= self.all_functions.len() {
                    self.err(
                        Some(pc),
                        format!(
                            "CallFunc func_id {} is out of bounds (have {} functions)",
                            func_id,
                            self.all_functions.len()
                        ),
                    );
                }
                let return_pc = pc + 1;
                if !self.func.stack_maps.contains_key(&return_pc) {
                    self.err(
                        Some(pc),
                        format!(
                            "CallFunc return site (pc {}) is missing a stack map",
                            return_pc
                        ),
                    );
                }
            },

            // ----- GC-only -----
            Instruction::ForceGC => {
                self.require_stack_map(pc);
            },

            // ----- Vector instructions -----
            Instruction::VecNew {
                descriptor_id,
                elem_size,
                dst_fp_offset,
                ..
            } => {
                self.check8(pc, "dst", dst_fp_offset);
                self.check_nonzero_size(pc, elem_size);
                if (descriptor_id as usize) >= self.descriptors.len() {
                    self.err(
                        Some(pc),
                        format!(
                            "VecNew descriptor_id {} is out of bounds (have {} descriptors)",
                            descriptor_id,
                            self.descriptors.len()
                        ),
                    );
                }
                self.require_stack_map(pc);
            },

            Instruction::VecLen {
                vec_fp_offset,
                dst_fp_offset,
            } => {
                self.check8(pc, "vec", vec_fp_offset);
                self.check8(pc, "dst", dst_fp_offset);
            },

            Instruction::VecPushBack {
                vec_fp_offset,
                elem_fp_offset,
                elem_size,
            } => {
                self.check8(pc, "vec", vec_fp_offset);
                self.check_nonzero_size(pc, elem_size);
                self.check_access(Some(pc), "elem", elem_fp_offset, elem_size);
                self.require_stack_map(pc);
            },

            Instruction::VecPopBack {
                vec_fp_offset,
                dst_fp_offset,
                elem_size,
            } => {
                self.check8(pc, "vec", vec_fp_offset);
                self.check_nonzero_size(pc, elem_size);
                self.check_access(Some(pc), "dst", dst_fp_offset, elem_size);
            },

            Instruction::VecLoadElem {
                vec_fp_offset,
                idx_fp_offset,
                dst_fp_offset,
                elem_size,
            } => {
                self.check8(pc, "vec", vec_fp_offset);
                self.check8(pc, "idx", idx_fp_offset);
                self.check_nonzero_size(pc, elem_size);
                self.check_access(Some(pc), "dst", dst_fp_offset, elem_size);
            },

            Instruction::VecStoreElem {
                vec_fp_offset,
                idx_fp_offset,
                src_fp_offset,
                elem_size,
            } => {
                self.check8(pc, "vec", vec_fp_offset);
                self.check8(pc, "idx", idx_fp_offset);
                self.check_nonzero_size(pc, elem_size);
                self.check_access(Some(pc), "src", src_fp_offset, elem_size);
            },

            // ----- Reference (fat pointer) instructions -----
            Instruction::VecBorrow {
                vec_fp_offset,
                idx_fp_offset,
                elem_size,
                dst_fp_offset,
            } => {
                self.check8(pc, "vec", vec_fp_offset);
                self.check8(pc, "idx", idx_fp_offset);
                self.check_nonzero_size(pc, elem_size);
                self.check_access(Some(pc), "dst (fat ptr)", dst_fp_offset, 16);
            },

            Instruction::BorrowLocal {
                local_fp_offset,
                dst_fp_offset,
            } => {
                self.check8(pc, "local", local_fp_offset);
                self.check_access(Some(pc), "dst (fat ptr)", dst_fp_offset, 16);
            },

            Instruction::ReadRef {
                ref_fp_offset,
                dst_fp_offset,
                size,
            } => {
                self.check_access(Some(pc), "ref (fat ptr)", ref_fp_offset, 16);
                self.check_nonzero_size(pc, size);
                self.check_access(Some(pc), "dst", dst_fp_offset, size);
            },

            Instruction::WriteRef {
                ref_fp_offset,
                src_fp_offset,
                size,
            } => {
                self.check_access(Some(pc), "ref (fat ptr)", ref_fp_offset, 16);
                self.check_nonzero_size(pc, size);
                self.check_access(Some(pc), "src", src_fp_offset, size);
            },

            // ----- Struct instructions -----
            Instruction::StructNew {
                descriptor_id,
                dst_fp_offset,
            } => {
                self.check8(pc, "dst", dst_fp_offset);
                if (descriptor_id as usize) >= self.descriptors.len() {
                    self.err(
                        Some(pc),
                        format!(
                            "StructNew descriptor_id {} is out of bounds (have {} descriptors)",
                            descriptor_id,
                            self.descriptors.len()
                        ),
                    );
                } else if !matches!(
                    self.descriptors[descriptor_id as usize],
                    ObjectDescriptor::Struct { .. }
                ) {
                    self.err(
                        Some(pc),
                        format!(
                            "StructNew descriptor_id {} does not refer to a Struct descriptor",
                            descriptor_id
                        ),
                    );
                }
                self.require_stack_map(pc);
            },

            Instruction::StructLoadField {
                struct_fp_offset,
                dst_fp_offset,
                size,
                ..
            } => {
                self.check8(pc, "struct", struct_fp_offset);
                self.check_nonzero_size(pc, size);
                self.check_access(Some(pc), "dst", dst_fp_offset, size);
            },

            Instruction::StructStoreField {
                struct_fp_offset,
                src_fp_offset,
                size,
                ..
            } => {
                self.check8(pc, "struct", struct_fp_offset);
                self.check_nonzero_size(pc, size);
                self.check_access(Some(pc), "src", src_fp_offset, size);
            },

            Instruction::StructBorrow {
                struct_fp_offset,
                dst_fp_offset,
                ..
            } => {
                self.check8(pc, "struct", struct_fp_offset);
                self.check_access(Some(pc), "dst (fat ptr)", dst_fp_offset, 16);
            },
        }
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn err(&mut self, pc: Option<usize>, msg: impl Into<String>) {
        self.errors.push(VerificationError {
            func_id: self.func_id,
            pc,
            message: msg.into(),
        });
    }

    fn check_access(&mut self, pc: Option<usize>, label: &str, offset: u32, width: u32) {
        let end = match offset.checked_add(width) {
            Some(e) => e,
            None => {
                self.err(
                    pc,
                    format!(
                        "{} access at offset {} with width {} overflows",
                        label, offset, width
                    ),
                );
                return;
            },
        };

        if end > self.func.extended_frame_size {
            self.err(
                pc,
                format!(
                    "{} access at offset {} with width {} exceeds extended_frame_size {}",
                    label, offset, width, self.func.extended_frame_size
                ),
            );
            return;
        }

        let meta_start = self.func.data_size;
        let meta_end = self.func.data_size + FRAME_METADATA_SIZE as u32;
        if offset < meta_end && meta_start < end {
            self.err(
                pc,
                format!(
                    "{} access [{}, {}) overlaps frame metadata [{}, {})",
                    label, offset, end, meta_start, meta_end
                ),
            );
        }
    }

    fn check8(&mut self, pc: usize, label: &str, offset: u32) {
        self.check_access(Some(pc), label, offset, 8);
    }

    fn check_jump(&mut self, pc: usize, dst_pc: u32) {
        if (dst_pc as usize) >= self.func.code.len() {
            self.err(
                Some(pc),
                format!(
                    "jump target pc {} is out of bounds (code length {})",
                    dst_pc,
                    self.func.code.len()
                ),
            );
        }
    }

    fn check_nonzero_size(&mut self, pc: usize, size: u32) {
        if size == 0 {
            self.err(Some(pc), "size/elem_size must be > 0");
        }
    }

    fn require_stack_map(&mut self, pc: usize) {
        if !self.func.stack_maps.contains_key(&pc) {
            self.err(
                Some(pc),
                "instruction may trigger GC but has no stack map entry",
            );
        }
    }
}
