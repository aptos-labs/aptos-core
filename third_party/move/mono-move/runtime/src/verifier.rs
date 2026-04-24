// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Static verifier for `Function` bodies. Checks well-formedness properties
//! that would otherwise cause undefined behavior at runtime: frame bounds,
//! pointer slot validity, invalid jump targets, etc.

use crate::types::ObjectDescriptor;
use mono_move_core::{
    CodeOffset, DescriptorId, FrameOffset, Function, MicroOp, FRAME_METADATA_SIZE,
};
use std::fmt;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct VerificationError {
    pub func_name: String,
    pub pc: Option<usize>,
    pub message: String,
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.pc {
            Some(pc) => write!(f, "'{}', pc {}: {}", self.func_name, pc, self.message),
            None => write!(f, "'{}': {}", self.func_name, self.message),
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Validate a single function and its pointer slots against the descriptor table.
/// Returns an empty `Vec` on success.
pub fn verify_function(
    func: &Function,
    descriptors: &[ObjectDescriptor],
) -> Vec<VerificationError> {
    let mut errors = Vec::new();
    let mut fv = FunctionVerifier {
        func,
        descriptors,
        errors: &mut errors,
    };
    fv.verify();
    errors
}

// ---------------------------------------------------------------------------
// Per-function verifier — holds shared state so helpers don't need many args
// ---------------------------------------------------------------------------

struct FunctionVerifier<'a> {
    func: &'a Function,
    descriptors: &'a [ObjectDescriptor],
    errors: &'a mut Vec<VerificationError>,
}

impl FunctionVerifier<'_> {
    fn verify(&mut self) {
        // SAFETY: The function's code is allocated in an executable arena that
        // is alive for the duration of verification.
        let code = unsafe { self.func.code.as_ref_unchecked() };
        let base_offsets = unsafe { self.func.frame_layout.heap_ptr_offsets.as_ref_unchecked() };
        let safe_point_layouts = unsafe { self.func.safe_point_layouts.entries() };

        // --- Function-level sanity ---
        // Code must be non-empty (at minimum a Return).
        if code.is_empty() {
            self.err(None, "code must be non-empty");
        }

        // --- Frame geometry ---
        // extended_frame_size must be large enough to hold locals + metadata.
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
        // args_size must fit within the data region.
        if self.func.args_size > self.func.args_and_locals_size {
            self.err(
                None,
                format!(
                    "args_size ({}) must be <= args_and_locals_size ({})",
                    self.func.args_size, self.func.args_and_locals_size
                ),
            );
        }

        // --- Base frame_layout: pointer offsets valid at every PC ---
        // Each offset must be in-bounds, not overlap metadata, and sorted.
        self.check_pointer_offsets(None, base_offsets);

        // --- Safe-point layouts: per-PC pointer offsets ---

        // Entries must be strictly sorted by code_offset.
        for w in safe_point_layouts.windows(2) {
            if w[0].code_offset.0 >= w[1].code_offset.0 {
                self.err(
                    None,
                    format!(
                        "safe_point_layouts: entries not strictly sorted (code_offset {} >= {})",
                        w[0].code_offset.0, w[1].code_offset.0
                    ),
                );
                break;
            }
        }

        // Per-entry: valid code_offset, pointer offsets in-bounds and
        // sorted, disjoint from frame_layout.
        for entry in safe_point_layouts {
            let co = entry.code_offset.0;

            if (co as usize) >= code.len() {
                self.err(
                    None,
                    format!(
                        "safe_point_layouts: code_offset {} out of bounds (code length {})",
                        co,
                        code.len()
                    ),
                );
            }

            let sp_offsets = unsafe { entry.layout.heap_ptr_offsets.as_ref_unchecked() };
            self.check_pointer_offsets(Some(co as usize), sp_offsets);

            for &off in sp_offsets {
                if base_offsets.contains(&off) {
                    self.err(
                        Some(co as usize),
                        format!(
                            "safe_point_layouts: offset {} duplicates frame_layout",
                            off.0
                        ),
                    );
                }
            }
        }

        // --- Per-instruction checks ---
        // Frame access bounds, jump targets, descriptor validity, etc.
        for (pc, instr) in code.iter().enumerate() {
            self.verify_instruction(pc, instr);
        }
    }

    /// Validate a set of pointer offsets: each must be within the extended
    /// frame, not overlap the metadata segment, and be strictly sorted.
    fn check_pointer_offsets(&mut self, pc: Option<usize>, offsets: &[FrameOffset]) {
        for &off in offsets {
            self.check_frame_access(pc, off, 8);
        }
        for w in offsets.windows(2) {
            if w[0].0 >= w[1].0 {
                self.err(
                    pc,
                    format!(
                        "pointer_offsets not strictly sorted ({} >= {})",
                        w[0].0, w[1].0
                    ),
                );
                break;
            }
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

            MicroOp::JumpGreaterU64Imm {
                target,
                src,
                imm: _,
            } => {
                self.check_frame_access_8(pc, src);
                self.check_jump(pc, target);
            },

            MicroOp::JumpLessEqualU64Imm {
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

            MicroOp::CallFunc { .. } => {
                // TODO: Verify that func_id is a valid function handle
                // index. Requires passing the function table (or its length)
                // into the verifier so we can bounds-check the callee index.
            },
            MicroOp::CallIndirect { .. } | MicroOp::CallDirect { .. } => {},

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

            // Inserted by the instrumentation pass; no frame accesses to verify.
            MicroOp::Charge { .. } => {},
        }
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn err(&mut self, pc: Option<usize>, msg: impl Into<String>) {
        // SAFETY: The function name is allocated in an arena alive during verification.
        let func_name = unsafe { self.func.name.as_ref_unchecked() }.to_string();
        self.errors.push(VerificationError {
            func_name,
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
        // SAFETY: code arena pointer is valid during verification.
        let code = unsafe { self.func.code.as_ref_unchecked() };
        if (target.0 as usize) >= code.len() {
            self.err(
                Some(pc),
                format!(
                    "jump target {} out of bounds (code length {})",
                    target.0,
                    code.len()
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
