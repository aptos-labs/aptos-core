// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Static verifier for `Function` bodies. Checks well-formedness properties
//! that would otherwise cause undefined behavior at runtime: frame bounds,
//! pointer slot validity, invalid jump targets, etc.

use crate::{
    CodeOffset, DescriptorId, FrameOffset, Function, MicroOp, ObjectDescriptor, FRAME_METADATA_SIZE,
};
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

/// Validate all functions and their pointer slots against the descriptor table.
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
                    "extended_frame_size ({}) must be >= frame_size() (args_and_locals_size {} + FRAME_METADATA_SIZE {} = {})",
                    self.func.extended_frame_size,
                    self.func.args_and_locals_size,
                    FRAME_METADATA_SIZE,
                    self.func.frame_size()
                ),
            );
        }
        if self.func.args_size > self.func.args_and_locals_size {
            self.err(
                None,
                format!(
                    "args_size ({}) must be <= args_and_locals_size ({})",
                    self.func.args_size, self.func.args_and_locals_size
                ),
            );
        }

        for &off in &self.func.pointer_offsets {
            self.check_frame_access(None, off, 8);
        }

        for (pc, instr) in self.func.code.iter().enumerate() {
            self.verify_instruction(pc, instr);
        }
    }

    fn verify_instruction(&mut self, pc: usize, instr: &MicroOp) {
        match *instr {
            MicroOp::StoreImm8 { dst, imm: _ } => {
                self.check_frame_access_8(pc, dst);
            },

            MicroOp::StoreRandomU64 { dst } => {
                self.check_frame_access_8(pc, dst);
            },

            MicroOp::SubU64Imm { dst, src, imm: _ }
            | MicroOp::RSubU64Imm { dst, src, imm: _ }
            | MicroOp::AddU64Imm { dst, src, imm: _ }
            | MicroOp::ShrU64Imm { dst, src, imm: _ } => {
                self.check_frame_access_8(pc, src);
                self.check_frame_access_8(pc, dst);
            },

            MicroOp::Move8 { dst, src } => {
                self.check_frame_access_8(pc, src);
                self.check_frame_access_8(pc, dst);
            },

            MicroOp::AddU64 { dst, lhs, rhs }
            | MicroOp::XorU64 { dst, lhs, rhs }
            | MicroOp::ModU64 { dst, lhs, rhs } => {
                self.check_frame_access_8(pc, lhs);
                self.check_frame_access_8(pc, rhs);
                self.check_frame_access_8(pc, dst);
            },

            MicroOp::Move { dst, src, size } => {
                self.check_nonzero_size(pc, size);
                self.check_frame_access(Some(pc), src, size);
                self.check_frame_access(Some(pc), dst, size);
            },

            MicroOp::Jump { target } => {
                self.check_jump(pc, target);
            },

            MicroOp::JumpNotZeroU64 { target, src } => {
                self.check_frame_access_8(pc, src);
                self.check_jump(pc, target);
            },

            MicroOp::JumpGreaterEqualU64Imm {
                target,
                src,
                imm: _,
            } => {
                self.check_frame_access_8(pc, src);
                self.check_jump(pc, target);
            },

            MicroOp::JumpLessU64Imm {
                target,
                src,
                imm: _,
            } => {
                self.check_frame_access_8(pc, src);
                self.check_jump(pc, target);
            },

            MicroOp::JumpLessU64 { target, lhs, rhs }
            | MicroOp::JumpGreaterEqualU64 { target, lhs, rhs }
            | MicroOp::JumpNotEqualU64 { target, lhs, rhs } => {
                self.check_frame_access_8(pc, lhs);
                self.check_frame_access_8(pc, rhs);
                self.check_jump(pc, target);
            },

            MicroOp::Return | MicroOp::ForceGC => {},

            MicroOp::CallFunc { func_id } => {
                if func_id as usize >= self.all_functions.len() {
                    self.err(Some(pc), format!("func_id {} out of bounds", func_id));
                }
            },

            // ----- VecNew -----
            MicroOp::VecNew { dst } => {
                self.check_frame_access_8(pc, dst);
            },

            MicroOp::VecLen { dst, vec_ref } => {
                self.check_frame_access(Some(pc), vec_ref, 16);
                self.check_frame_access_8(pc, dst);
            },

            MicroOp::HeapMoveFrom8 { dst, heap_ptr, .. } => {
                self.check_frame_access_8(pc, heap_ptr);
                self.check_frame_access_8(pc, dst);
            },

            MicroOp::HeapMoveTo8 { heap_ptr, src, .. } => {
                self.check_frame_access_8(pc, heap_ptr);
                self.check_frame_access_8(pc, src);
            },

            // ----- Vec push/pop: vec_ref (16B fat pointer) + variable-width slot -----
            MicroOp::VecPushBack {
                vec_ref,
                elem,
                elem_size,
                descriptor_id,
            } => {
                self.check_frame_access(Some(pc), vec_ref, 16);
                self.check_nonzero_size(pc, elem_size);
                self.check_frame_access(Some(pc), elem, elem_size);
                self.check_descriptor(pc, descriptor_id);
            },

            MicroOp::VecPopBack {
                dst,
                vec_ref,
                elem_size,
            } => {
                self.check_frame_access(Some(pc), vec_ref, 16);
                self.check_nonzero_size(pc, elem_size);
                self.check_frame_access(Some(pc), dst, elem_size);
            },

            // ----- Vec indexed load/store -----
            MicroOp::VecLoadElem {
                dst,
                vec_ref,
                idx,
                elem_size,
            } => {
                self.check_frame_access(Some(pc), vec_ref, 16);
                self.check_frame_access_8(pc, idx);
                self.check_nonzero_size(pc, elem_size);
                self.check_frame_access(Some(pc), dst, elem_size);
            },

            MicroOp::VecStoreElem {
                vec_ref,
                idx,
                src,
                elem_size,
            } => {
                self.check_frame_access(Some(pc), vec_ref, 16);
                self.check_frame_access_8(pc, idx);
                self.check_nonzero_size(pc, elem_size);
                self.check_frame_access(Some(pc), src, elem_size);
            },

            // ----- Borrow producing fat pointer (16B dst) -----
            MicroOp::VecBorrow {
                dst,
                vec_ref,
                idx,
                elem_size,
            } => {
                self.check_frame_access(Some(pc), vec_ref, 16);
                self.check_frame_access_8(pc, idx);
                self.check_nonzero_size(pc, elem_size);
                self.check_frame_access(Some(pc), dst, 16);
            },

            MicroOp::SlotBorrow { dst, local } => {
                self.check_frame_access_8(pc, local);
                self.check_frame_access(Some(pc), dst, 16);
            },

            MicroOp::HeapBorrow { dst, obj_ref, .. } => {
                self.check_frame_access(Some(pc), obj_ref, 16);
                self.check_frame_access(Some(pc), dst, 16);
            },

            MicroOp::ReadRef { dst, ref_ptr, size } => {
                self.check_frame_access(Some(pc), ref_ptr, 16);
                self.check_nonzero_size(pc, size);
                self.check_frame_access(Some(pc), dst, size);
            },

            MicroOp::WriteRef { ref_ptr, src, size } => {
                self.check_frame_access(Some(pc), ref_ptr, 16);
                self.check_nonzero_size(pc, size);
                self.check_frame_access(Some(pc), src, size);
            },

            // ----- Heap object instructions -----
            MicroOp::HeapNew { dst, descriptor_id } => {
                self.check_frame_access_8(pc, dst);
                self.check_descriptor(pc, descriptor_id);
                if descriptor_id.as_usize() < self.descriptors.len()
                    && !matches!(
                        self.descriptors[descriptor_id.as_usize()],
                        ObjectDescriptor::Struct { .. } | ObjectDescriptor::Enum { .. }
                    )
                {
                    self.err(
                        Some(pc),
                        format!("descriptor_id {} is not a Struct or Enum", descriptor_id),
                    );
                }
            },

            MicroOp::HeapMoveToImm8 { heap_ptr, .. } => {
                self.check_frame_access_8(pc, heap_ptr);
            },

            MicroOp::HeapMoveFrom {
                dst,
                heap_ptr,
                size,
                ..
            } => {
                self.check_frame_access_8(pc, heap_ptr);
                self.check_nonzero_size(pc, size);
                self.check_frame_access(Some(pc), dst, size);
            },

            MicroOp::HeapMoveTo {
                heap_ptr,
                src,
                size,
                ..
            } => {
                self.check_frame_access_8(pc, heap_ptr);
                self.check_nonzero_size(pc, size);
                self.check_frame_access(Some(pc), src, size);
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

    fn check_frame_access(&mut self, pc: Option<usize>, offset: FrameOffset, size: u32) {
        let offset = offset.0 as usize;
        let width = size as usize;
        let end = match offset.checked_add(width) {
            Some(e) => e,
            None => {
                self.err(pc, format!("access at offset {} overflows", offset));
                return;
            },
        };

        if end > self.func.extended_frame_size {
            self.err(
                pc,
                format!(
                    "access [{}, {}) exceeds extended_frame_size {}",
                    offset, end, self.func.extended_frame_size
                ),
            );
            return;
        }

        let meta_start = self.func.args_and_locals_size;
        let meta_end = self.func.args_and_locals_size + FRAME_METADATA_SIZE;
        if offset < meta_end && meta_start < end {
            self.err(
                pc,
                format!(
                    "access [{}, {}) overlaps metadata [{}, {})",
                    offset, end, meta_start, meta_end
                ),
            );
        }
    }

    fn check_frame_access_8(&mut self, pc: usize, offset: FrameOffset) {
        self.check_frame_access(Some(pc), offset, 8);
    }

    fn check_descriptor(&mut self, pc: usize, descriptor_id: DescriptorId) {
        if descriptor_id.as_usize() >= self.descriptors.len() {
            self.err(
                Some(pc),
                format!("descriptor_id {} out of bounds", descriptor_id),
            );
        }
    }

    fn check_jump(&mut self, pc: usize, target: CodeOffset) {
        if (target.0 as usize) >= self.func.code.len() {
            self.err(
                Some(pc),
                format!(
                    "jump target {} out of bounds (code length {})",
                    target.0,
                    self.func.code.len()
                ),
            );
        }
    }

    fn check_nonzero_size(&mut self, pc: usize, size: u32) {
        if size == 0 {
            self.err(Some(pc), "size must be > 0");
        }
    }
}
