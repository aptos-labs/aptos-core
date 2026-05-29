// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Static verifier for `Function` bodies. Checks well-formedness properties
//! that would otherwise cause undefined behavior at runtime: frame bounds,
//! pointer slot validity, invalid jump targets, op/descriptor variant
//! mismatch, etc.
//!
//! Descriptors themselves are not re-verified here; their soundness is
//! enforced by [`mono_move_core::ObjectDescriptor`]'s constructors at
//! publish time.

use mono_move_core::{
    CallClosureOp, ClosureFuncRef, CodeOffset, DescriptorId, DescriptorProvider, FrameOffset,
    Function, IntBinaryOp, MicroOp, ObjectDescriptorInner, PackClosureOp, ShiftOperand,
    CLOSURE_DESCRIPTOR_ID, FRAME_METADATA_SIZE,
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

/// Validate a single function and its pointer slots against the descriptor
/// provider. Returns an empty `Vec` on success.
pub fn verify_function<P: DescriptorProvider + ?Sized>(
    func: &Function,
    provider: &P,
) -> Vec<VerificationError> {
    let mut errors = Vec::new();
    let mut fv = FunctionVerifier {
        func,
        provider,
        errors: &mut errors,
    };
    fv.verify();
    errors
}

/// Validate every function in a program against a shared descriptor
/// provider. Errors from each function are concatenated.
pub fn verify_program<P: DescriptorProvider + ?Sized>(
    funcs: &[&Function],
    provider: &P,
) -> Vec<VerificationError> {
    let mut errors = Vec::new();
    for func in funcs {
        errors.extend(verify_function(func, provider));
    }
    errors
}

// ---------------------------------------------------------------------------
// Per-function verifier — holds shared state so helpers don't need many args
// ---------------------------------------------------------------------------

struct FunctionVerifier<'a, P: DescriptorProvider + ?Sized> {
    func: &'a Function,
    provider: &'a P,
    errors: &'a mut Vec<VerificationError>,
}

impl<P: DescriptorProvider + ?Sized> FunctionVerifier<'_, P> {
    fn verify(&mut self) {
        let code_guard = self.func.code.load();
        let code = code_guard.as_slice();

        let base_offsets = &self.func.frame_layout.heap_ptr_offsets;
        let safe_point_layouts = self.func.safe_point_layouts.entries();

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
                    "extended_frame_size ({}) must be >= frame_size() (param_and_local_sizes_sum {} + FRAME_METADATA_SIZE {} = {})",
                    self.func.extended_frame_size,
                    self.func.param_and_local_sizes_sum,
                    FRAME_METADATA_SIZE,
                    self.func.frame_size()
                ),
            );
        }
        // param_sizes_sum must fit within the data region.
        if self.func.param_sizes_sum > self.func.param_and_local_sizes_sum {
            self.err(
                None,
                format!(
                    "param_sizes_sum ({}) must be <= param_and_local_sizes_sum ({})",
                    self.func.param_sizes_sum, self.func.param_and_local_sizes_sum
                ),
            );
        }
        // param_and_local_sizes_sum must be 8-byte aligned. The runtime writes
        // frame metadata (saved pc/fp/func_ptr) at `fp + param_and_local_sizes_sum`
        // via `write_u64`, which requires 8-byte alignment, and the callee
        // frame pointer (`fp + param_and_local_sizes_sum + FRAME_METADATA_SIZE`)
        // inherits this alignment for the callee's slot accesses.
        if !self.func.param_and_local_sizes_sum.is_multiple_of(8) {
            self.err(
                None,
                format!(
                    "param_and_local_sizes_sum ({}) must be 8-byte aligned",
                    self.func.param_and_local_sizes_sum
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

        // Per-entry: valid code_offset, op-kind matches top-frame-only
        // contract, pointer offsets in-bounds and sorted, disjoint
        // from frame_layout.
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
            } else {
                // Top-frame-only contract: an entry must sit at the
                // PC of an allocating op (see `MicroOp::is_allocating`).
                let op = &code[co as usize];
                if !op.is_allocating() {
                    self.err(
                        Some(co as usize),
                        format!(
                            "safe_point_layouts: code_offset {} is not at an allocating op; \
                             top-frame-only contract — see `SafePointEntry`",
                            co
                        ),
                    );
                }
            }

            let sp_offsets = &entry.layout.heap_ptr_offsets;
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

            MicroOp::StoreImm16 { dst, imm: _ } => {
                self.check_frame_access(Some(pc), dst, 16);
            },

            MicroOp::StoreImm32 { dst, imm: _ } => {
                self.check_frame_access(Some(pc), dst, 32);
            },

            MicroOp::StoreRandomU64 { dst } => {
                self.check_frame_access_8(pc, dst);
            },

            MicroOp::AddU64Imm { dst, src, imm: _ }
            | MicroOp::SubU64Imm { dst, src, imm: _ }
            | MicroOp::RSubU64Imm { dst, src, imm: _ }
            | MicroOp::MulU64Imm { dst, src, imm: _ } => {
                self.check_frame_access_8(pc, src);
                self.check_frame_access_8(pc, dst);
            },

            // Div / Mod imm: reject `imm == 0` statically — at runtime it
            // would always abort, so this is dead-code-with-a-bomb.
            //
            // TODO: this changes the surface vs the old VM, which aborted
            // at runtime with a `DIV_BY_ZERO` status code. The cleanest
            // fix is probably for the specializer to detect `imm == 0` and
            // emit an explicit `Abort(DIV_BY_ZERO)` instead of `*U64Imm`,
            // so the verifier never sees the bad op. Open question: can
            // the specializer report any error post-bytecode-verification,
            // and if so should it fail the whole module or only the
            // function (or only the basic block)? The branch containing
            // `imm == 0` may not even be reachable at runtime. Revisit
            // once the abort/error story is settled.
            MicroOp::DivU64Imm { dst, src, imm } | MicroOp::ModU64Imm { dst, src, imm } => {
                self.check_frame_access_8(pc, src);
                self.check_frame_access_8(pc, dst);
                if imm == 0 {
                    self.err(Some(pc), "division by zero (imm)");
                }
            },

            // Shift imm: reject `imm >= 64` statically — same reason as
            // div by zero (the runtime would always abort).
            MicroOp::ShlU64Imm { dst, src, imm } | MicroOp::ShrU64Imm { dst, src, imm } => {
                self.check_frame_access_8(pc, src);
                self.check_frame_access_8(pc, dst);
                if imm >= 64 {
                    self.err(Some(pc), format!("shift amount {} exceeds 63 (imm)", imm));
                }
            },

            MicroOp::Move8 { dst, src } => {
                self.check_frame_access_8(pc, src);
                self.check_frame_access_8(pc, dst);
            },

            MicroOp::AddU64 { dst, lhs, rhs }
            | MicroOp::SubU64 { dst, lhs, rhs }
            | MicroOp::MulU64 { dst, lhs, rhs }
            | MicroOp::DivU64 { dst, lhs, rhs }
            | MicroOp::ModU64 { dst, lhs, rhs }
            | MicroOp::BitAndU64 { dst, lhs, rhs }
            | MicroOp::BitOrU64 { dst, lhs, rhs }
            | MicroOp::BitXorU64 { dst, lhs, rhs } => {
                self.check_frame_access_8(pc, lhs);
                self.check_frame_access_8(pc, rhs);
                self.check_frame_access_8(pc, dst);
            },

            // Shifts: `rhs` is a 1-byte slot (the Move shift amount is u8).
            MicroOp::ShlU64 { dst, lhs, rhs } | MicroOp::ShrU64 { dst, lhs, rhs } => {
                self.check_frame_access_8(pc, lhs);
                self.check_frame_access_1(pc, rhs);
                self.check_frame_access_8(pc, dst);
            },

            // Unspecialized integer binary ops. Checks:
            //   - `dst`, `lhs`, and (if `rhs` is a slot) `rhs` are all
            //     in-bounds slots of width `op.rhs.byte_width()`.
            //   - Bitwise ops reject signed operands.
            //
            // TODO: also statically reject `IntDiv`/`IntMod` with an
            // imm-zero rhs. Same for u64 variants (currently the u64
            // variants statically error out) and shifts. Revisit once we
            // have a clearer policy on what the specializer is allowed
            // to reject statically — turning runtime aborts into
            // verification errors makes the specializer's
            // constant-folding observable in the error type.
            MicroOp::IntAdd(ref op)
            | MicroOp::IntSub(ref op)
            | MicroOp::IntMul(ref op)
            | MicroOp::IntDiv(ref op)
            | MicroOp::IntMod(ref op) => {
                self.check_int_binop_frame_access(pc, op);
            },
            MicroOp::IntBitAnd(ref op) | MicroOp::IntBitOr(ref op) | MicroOp::IntBitXor(ref op) => {
                self.check_int_binop_frame_access(pc, op);
                if op.rhs.is_signed() {
                    self.err(Some(pc), "bitwise on signed type");
                }
            },

            // Shifts: `lhs` / `dst` are slots of width `op.ty.byte_width()`;
            // `rhs` is either a 1-byte slot or an inline u8. The shift
            // amount is statically range-checked for the imm form, and
            // signedness of `ty` is checked at runtime via the dispatcher.
            //
            // TODO: as noted above for div/mod, the static imm range check
            // turns a runtime abort into a verification error — revisit.
            MicroOp::IntShl(op) | MicroOp::IntShr(op) => {
                let size = op.ty.byte_width() as u32;
                self.check_frame_access(Some(pc), op.lhs, size);
                self.check_frame_access(Some(pc), op.dst, size);
                match op.rhs {
                    ShiftOperand::SlotU8(rhs) => self.check_frame_access_1(pc, rhs),
                    ShiftOperand::ImmU8(imm) => {
                        if (imm as usize) >= op.ty.bit_width() {
                            self.err(
                                Some(pc),
                                format!(
                                    "shift amount {} exceeds bit width {} (imm)",
                                    imm,
                                    op.ty.bit_width()
                                ),
                            );
                        }
                    },
                }
            },

            // `IntNegate` is signed-only — checked at runtime by the
            // dispatcher. The `src == MIN` overflow case is also a
            // runtime abort.
            MicroOp::IntNegate(op) => {
                let size = op.ty.byte_width() as u32;
                self.check_frame_access(Some(pc), op.src, size);
                self.check_frame_access(Some(pc), op.dst, size);
            },

            MicroOp::IntCast(op) => {
                // Note: the Move bytecode permits casting from one integer type to self, effectively a no-op.
                // Therefore we must NOT ban it here.
                self.check_frame_access(Some(pc), op.src, op.from.byte_width() as u32);
                self.check_frame_access(Some(pc), op.dst, op.to.byte_width() as u32);
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

            MicroOp::Abort { code } => {
                self.check_frame_access_8(pc, code);
            },

            MicroOp::AbortMsg { code, message } => {
                self.check_frame_access_8(pc, code);
                self.check_frame_access_8(pc, message);
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
                self.check_descriptor_variant(
                    pc,
                    "VecPushBack",
                    descriptor_id,
                    |inner| matches!(inner, ObjectDescriptorInner::Vector { .. }),
                    "a Vector",
                );
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

            MicroOp::DeriveRefOffsetImm {
                dst_ref, src_ref, ..
            } => {
                self.check_frame_access(Some(pc), src_ref, 16);
                self.check_frame_access(Some(pc), dst_ref, 16);
            },

            MicroOp::ReadRefOffset {
                dst,
                ref_ptr,
                offset,
                size,
            } => {
                self.check_frame_access(Some(pc), ref_ptr, 16);
                self.check_nonzero_size(pc, size);
                self.check_ref_offset_size_no_overflow(pc, offset, size);
                self.check_frame_access(Some(pc), dst, size);
            },

            MicroOp::WriteRefOffset {
                ref_ptr,
                offset,
                src,
                size,
            } => {
                self.check_frame_access(Some(pc), ref_ptr, 16);
                self.check_nonzero_size(pc, size);
                self.check_ref_offset_size_no_overflow(pc, offset, size);
                self.check_frame_access(Some(pc), src, size);
            },

            // ----- Heap object instructions -----
            MicroOp::HeapNew { dst, descriptor_id } => {
                self.check_frame_access_8(pc, dst);
                self.check_descriptor_variant(
                    pc,
                    "HeapNew",
                    descriptor_id,
                    |inner| {
                        matches!(
                            inner,
                            ObjectDescriptorInner::Struct { .. }
                                | ObjectDescriptorInner::Enum { .. }
                        )
                    },
                    "a Struct or Enum",
                );
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

            MicroOp::PackClosure(ref op) => self.verify_pack_closure(pc, op),
            MicroOp::CallClosure(ref op) => self.verify_call_closure(pc, op),

            MicroOp::Exists { addr, ty: _, dst } | MicroOp::MoveFrom { addr, ty: _, dst } => {
                // Exists writes a bool (currently widened to 8 bytes); MoveFrom
                // writes an 8-byte owned heap pointer.
                self.check_frame_access(Some(pc), addr, 32);
                self.check_frame_access_8(pc, dst);
            },
            MicroOp::BorrowGlobal { addr, ty: _, dst }
            | MicroOp::BorrowGlobalMut { addr, ty: _, dst } => {
                // Both produce a reference, i.e. a 16-byte fat pointer.
                self.check_frame_access(Some(pc), addr, 32);
                self.check_frame_access(Some(pc), dst, 16);
            },
            MicroOp::MoveTo { addr, ty: _, src } => {
                // TODO(correctness):
                //   Move use signer reference, so we need 16 bytes if we no longer use address.
                self.check_frame_access(Some(pc), addr, 32);
                self.check_frame_access_8(pc, src);
            },
        }
    }

    fn verify_pack_closure(&mut self, pc: usize, op: &PackClosureOp) {
        // Destination: 8-byte heap pointer slot for the closure heap object.
        self.check_frame_access_8(pc, op.dst);
        // The closure heap object uses the implicit reserved
        // `CLOSURE_DESCRIPTOR_ID` (no per-op field). Every provider installs
        // `Closure` at this slot; assert to catch internal regressions.
        debug_assert!(
            matches!(
                self.provider
                    .descriptor(CLOSURE_DESCRIPTOR_ID)
                    .map(|d| d.inner()),
                Some(ObjectDescriptorInner::Closure)
            ),
            "reserved descriptor[{}] must be Closure",
            CLOSURE_DESCRIPTOR_ID
        );
        // captured_data_descriptor_id and `captured` must agree on emptiness:
        // a captured-data object is allocated iff at least one value is
        // captured.
        match (op.captured_data_descriptor_id, op.captured.is_empty()) {
            (None, true) => {},
            (Some(id), false) => {
                self.check_descriptor_variant(
                    pc,
                    "PackClosure",
                    id,
                    |inner| matches!(inner, ObjectDescriptorInner::CapturedData { .. }),
                    "a CapturedData",
                );
            },
            (Some(id), true) => {
                self.err(
                    Some(pc),
                    format!(
                        "PackClosure: captured_data_descriptor_id {} provided but no captures",
                        id
                    ),
                );
            },
            (None, false) => {
                self.err(
                    Some(pc),
                    "PackClosure: captured non-empty but captured_data_descriptor_id is None"
                        .to_string(),
                );
            },
        }
        // Captured sources: verify each (offset, size) is in-bounds.
        for slot in &op.captured {
            self.check_nonzero_size(pc, slot.size);
            self.check_frame_access(Some(pc), slot.offset, slot.size);
        }
        // Captured count must match the mask.
        let captured_count = op.mask.count_ones() as usize;
        if op.captured.len() != captured_count {
            self.err(
                Some(pc),
                format!(
                    "PackClosure: captured list length {} does not match mask captured count {}",
                    op.captured.len(),
                    captured_count
                ),
            );
        }
        // For Resolved targets the mask must not set bits beyond the
        // callee's parameter count, and the callee's parameter count must
        // fit in the u64 mask.
        match &op.func_ref {
            ClosureFuncRef::Resolved(func_ptr) => {
                let callee = unsafe { func_ptr.as_ref_unchecked() };
                let param_count = callee.param_sizes.len();
                if param_count > 64 {
                    self.err(
                        Some(pc),
                        format!(
                            "PackClosure: callee has {} params, exceeds 64-bit mask capacity",
                            param_count
                        ),
                    );
                }
                if param_count < 64 && op.mask >> param_count != 0 {
                    self.err(
                        Some(pc),
                        format!(
                            "PackClosure: mask 0x{:x} sets bits beyond callee param count {}",
                            op.mask, param_count
                        ),
                    );
                }
                // Each captured slot's size must match the corresponding
                // callee parameter's size. The captured list is in
                // mask-bit-set order through the param list.
                let mut k = 0usize;
                for (i, &param_size) in callee.param_sizes.iter().enumerate() {
                    if (op.mask >> i) & 1 != 0 {
                        if let Some(slot) = op.captured.get(k) {
                            if slot.size != param_size {
                                self.err(
                                    Some(pc),
                                    format!(
                                        "PackClosure: captured[{}].size {} != callee param_sizes[{}] {}",
                                        k, slot.size, i, param_size,
                                    ),
                                );
                            }
                        }
                        k += 1;
                    }
                }
            },
        }
        // The captured-data descriptor's values region must be exactly
        // the materialized captured values — no padding, no extras.
        // Together with the descriptor self-soundness pass, this is
        // sufficient to ensure the runtime's fixed-offset writes stay in
        // bounds.
        let Some(id) = op.captured_data_descriptor_id else {
            // The `None` case is already validated by the (None, _) arms of the
            // match above.
            return;
        };
        let expected_values_size: u32 = op.captured.iter().map(|s| s.size).sum();
        let Some(desc) = self.provider.descriptor(id) else {
            self.err(
                Some(pc),
                format!(
                    "PackClosure: captured_data descriptor {} not found",
                    id.as_u32()
                ),
            );
            return;
        };
        let ObjectDescriptorInner::CapturedData { size: actual, .. } = desc.inner() else {
            self.err(
                Some(pc),
                format!(
                    "PackClosure: captured_data descriptor {} is not a CapturedData descriptor",
                    id.as_u32()
                ),
            );
            return;
        };
        if *actual != expected_values_size {
            self.err(
                Some(pc),
                format!(
                    "PackClosure: captured_data values size {} != expected {}",
                    actual, expected_values_size
                ),
            );
        }
    }

    fn verify_call_closure(&mut self, pc: usize, op: &CallClosureOp) {
        // Closure source: 8-byte heap pointer slot.
        self.check_frame_access_8(pc, op.closure_src);
        // Provided arg sources: each (offset, size) in-bounds.
        for slot in &op.provided_args {
            self.check_nonzero_size(pc, slot.size);
            self.check_frame_access(Some(pc), slot.offset, slot.size);
        }
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn err(&mut self, pc: Option<usize>, msg: impl Into<String>) {
        self.errors.push(VerificationError {
            func_name: self.func.name().to_string(),
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

        let meta_start = self.func.param_and_local_sizes_sum;
        let meta_end = self.func.param_and_local_sizes_sum + FRAME_METADATA_SIZE;
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

    fn check_frame_access_1(&mut self, pc: usize, offset: FrameOffset) {
        self.check_frame_access(Some(pc), offset, 1);
    }

    /// Verify an [`IntBinaryOp`]: dst and lhs are slots of width
    /// `op.rhs.byte_width()`; if rhs is a slot arm, its slot is checked too.
    fn check_int_binop_frame_access(&mut self, pc: usize, op: &IntBinaryOp) {
        let size = op.rhs.byte_width() as u32;
        self.check_frame_access(Some(pc), op.lhs, size);
        self.check_frame_access(Some(pc), op.dst, size);
        if let Some(rhs_off) = op.rhs.slot_offset() {
            self.check_frame_access(Some(pc), rhs_off, size);
        }
    }

    /// Check that `descriptor_id` resolves and its variant satisfies `pred`.
    /// `op` names the calling micro-op and `expected` names the expected
    /// variant, both for the error message.
    fn check_descriptor_variant(
        &mut self,
        pc: usize,
        op: &str,
        descriptor_id: DescriptorId,
        pred: impl FnOnce(&ObjectDescriptorInner) -> bool,
        expected: &str,
    ) {
        match self.provider.descriptor(descriptor_id) {
            None => self.err(
                Some(pc),
                format!("{}: unknown descriptor_id {}", op, descriptor_id),
            ),
            Some(desc) if !pred(desc.inner()) => self.err(
                Some(pc),
                format!(
                    "{}: descriptor_id {} is not {}",
                    op, descriptor_id, expected
                ),
            ),
            Some(_) => {},
        }
    }

    fn check_jump(&mut self, pc: usize, target: CodeOffset) {
        // TODO: avoid reloading code.
        let code_len = self.func.code.load().len();
        if (target.0 as usize) >= code_len {
            self.err(
                Some(pc),
                format!(
                    "jump target {} out of bounds (code length {})",
                    target.0, code_len,
                ),
            );
        }
    }

    fn check_nonzero_size(&mut self, pc: usize, size: u32) {
        if size == 0 {
            self.err(Some(pc), "size must be > 0");
        }
    }

    /// Requires `offset + size` to fit in `u32`, so the access window
    /// `[offset, offset + size)` cannot wrap.
    fn check_ref_offset_size_no_overflow(&mut self, pc: usize, offset: u32, size: u32) {
        if offset.checked_add(size).is_none() {
            self.err(
                Some(pc),
                format!("offset {} + size {} overflows u32", offset, size),
            );
        }
    }
}
