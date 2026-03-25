// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Instruction utilities for the pipeline.
//!
//! Helpers used by `[ssa_conversion]`, `[slot_alloc]`, and `[optimize]`.

use crate::ir::{BinaryOp, ImmValue, Instr, Slot};
use std::{collections::BTreeMap, ops::Range};

/// Get named slots defined (written) and used (read) by an instruction.
pub(crate) fn get_defs_uses(instr: &Instr) -> (Vec<Slot>, Vec<Slot>) {
    match instr {
        Instr::LdConst(dst, _)
        | Instr::LdTrue(dst)
        | Instr::LdFalse(dst)
        | Instr::LdU8(dst, _)
        | Instr::LdU16(dst, _)
        | Instr::LdU32(dst, _)
        | Instr::LdU64(dst, _)
        | Instr::LdU128(dst, _)
        | Instr::LdU256(dst, _)
        | Instr::LdI8(dst, _)
        | Instr::LdI16(dst, _)
        | Instr::LdI32(dst, _)
        | Instr::LdI64(dst, _)
        | Instr::LdI128(dst, _)
        | Instr::LdI256(dst, _) => (vec![*dst], vec![]),

        Instr::Copy(dst, src) | Instr::Move(dst, src) => (vec![*dst], vec![*src]),

        Instr::UnaryOp(dst, _, src) => (vec![*dst], vec![*src]),
        Instr::BinaryOp(dst, _, lhs, rhs) => (vec![*dst], vec![*lhs, *rhs]),
        Instr::BinaryOpImm(dst, _, lhs, _) => (vec![*dst], vec![*lhs]),

        Instr::Pack(dst, _, fields) | Instr::PackGeneric(dst, _, fields) => {
            (vec![*dst], fields.to_vec())
        },
        Instr::Unpack(dsts, _, src) | Instr::UnpackGeneric(dsts, _, src) => {
            (dsts.to_vec(), vec![*src])
        },

        Instr::PackVariant(dst, _, fields) | Instr::PackVariantGeneric(dst, _, fields) => {
            (vec![*dst], fields.to_vec())
        },
        Instr::UnpackVariant(dsts, _, src) | Instr::UnpackVariantGeneric(dsts, _, src) => {
            (dsts.to_vec(), vec![*src])
        },
        Instr::TestVariant(dst, _, src) | Instr::TestVariantGeneric(dst, _, src) => {
            (vec![*dst], vec![*src])
        },

        Instr::ImmBorrowLoc(dst, src) | Instr::MutBorrowLoc(dst, src) => (vec![*dst], vec![*src]),
        Instr::ImmBorrowField(dst, _, src)
        | Instr::MutBorrowField(dst, _, src)
        | Instr::ImmBorrowFieldGeneric(dst, _, src)
        | Instr::MutBorrowFieldGeneric(dst, _, src)
        | Instr::ImmBorrowVariantField(dst, _, src)
        | Instr::MutBorrowVariantField(dst, _, src)
        | Instr::ImmBorrowVariantFieldGeneric(dst, _, src)
        | Instr::MutBorrowVariantFieldGeneric(dst, _, src) => (vec![*dst], vec![*src]),
        Instr::ReadRef(dst, src) => (vec![*dst], vec![*src]),
        Instr::WriteRef(dst_ref, val) => (vec![], vec![*dst_ref, *val]),

        Instr::ReadField(dst, _, src)
        | Instr::ReadFieldGeneric(dst, _, src)
        | Instr::ReadVariantField(dst, _, src)
        | Instr::ReadVariantFieldGeneric(dst, _, src) => (vec![*dst], vec![*src]),
        Instr::WriteField(_, dst_ref, val)
        | Instr::WriteFieldGeneric(_, dst_ref, val)
        | Instr::WriteVariantField(_, dst_ref, val)
        | Instr::WriteVariantFieldGeneric(_, dst_ref, val) => (vec![], vec![*dst_ref, *val]),

        Instr::Exists(dst, _, addr)
        | Instr::ExistsGeneric(dst, _, addr)
        | Instr::MoveFrom(dst, _, addr)
        | Instr::MoveFromGeneric(dst, _, addr) => (vec![*dst], vec![*addr]),
        Instr::MoveTo(_, signer, val) | Instr::MoveToGeneric(_, signer, val) => {
            (vec![], vec![*signer, *val])
        },
        Instr::ImmBorrowGlobal(dst, _, addr)
        | Instr::ImmBorrowGlobalGeneric(dst, _, addr)
        | Instr::MutBorrowGlobal(dst, _, addr)
        | Instr::MutBorrowGlobalGeneric(dst, _, addr) => (vec![*dst], vec![*addr]),

        Instr::Call(rets, _, args) | Instr::CallGeneric(rets, _, args) => {
            (rets.to_vec(), args.to_vec())
        },
        Instr::PackClosure(dst, _, _, captured)
        | Instr::PackClosureGeneric(dst, _, _, captured) => (vec![*dst], captured.to_vec()),
        Instr::CallClosure(rets, _, args) => (rets.to_vec(), args.to_vec()),

        Instr::VecPack(dst, _, _, elems) => (vec![*dst], elems.to_vec()),
        Instr::VecLen(dst, _, src) => (vec![*dst], vec![*src]),
        Instr::VecImmBorrow(dst, _, vec_ref, idx) | Instr::VecMutBorrow(dst, _, vec_ref, idx) => {
            (vec![*dst], vec![*vec_ref, *idx])
        },
        Instr::VecPushBack(_, vec_ref, val) => (vec![], vec![*vec_ref, *val]),
        Instr::VecPopBack(dst, _, src) => (vec![*dst], vec![*src]),
        Instr::VecUnpack(dsts, _, _, src) => (dsts.to_vec(), vec![*src]),
        Instr::VecSwap(_, vec_ref, idx_a, idx_b) => (vec![], vec![*vec_ref, *idx_a, *idx_b]),

        Instr::Label(_) | Instr::Branch(_) => (vec![], vec![]),
        Instr::BrTrue(_, cond) | Instr::BrFalse(_, cond) => (vec![], vec![*cond]),
        Instr::Ret(rets) => (vec![], rets.to_vec()),
        Instr::Abort(code) => (vec![], vec![*code]),
        Instr::AbortMsg(code, msg) => (vec![], vec![*code, *msg]),
    }
}

/// Apply named-slot remapping to all operands of an instruction.
pub(crate) fn remap_instr(instr: &mut Instr, map: &BTreeMap<Slot, Slot>) {
    fn r(slot: &mut Slot, map: &BTreeMap<Slot, Slot>) {
        if let Some(&new) = map.get(slot) {
            *slot = new;
        }
    }

    fn r_vec(slots: &mut [Slot], map: &BTreeMap<Slot, Slot>) {
        for slot in slots.iter_mut() {
            r(slot, map);
        }
    }

    match instr {
        Instr::LdConst(d, _)
        | Instr::LdTrue(d)
        | Instr::LdFalse(d)
        | Instr::LdU8(d, _)
        | Instr::LdU16(d, _)
        | Instr::LdU32(d, _)
        | Instr::LdU64(d, _)
        | Instr::LdU128(d, _)
        | Instr::LdU256(d, _)
        | Instr::LdI8(d, _)
        | Instr::LdI16(d, _)
        | Instr::LdI32(d, _)
        | Instr::LdI64(d, _)
        | Instr::LdI128(d, _)
        | Instr::LdI256(d, _) => r(d, map),

        Instr::Copy(d, s) | Instr::Move(d, s) => {
            r(d, map);
            r(s, map);
        },
        Instr::UnaryOp(d, _, s) => {
            r(d, map);
            r(s, map);
        },
        Instr::BinaryOp(d, _, l, rr) => {
            r(d, map);
            r(l, map);
            r(rr, map);
        },
        Instr::BinaryOpImm(d, _, l, _) => {
            r(d, map);
            r(l, map);
        },

        Instr::Pack(d, _, fields) | Instr::PackGeneric(d, _, fields) => {
            r(d, map);
            r_vec(fields, map);
        },
        Instr::Unpack(ds, _, s) | Instr::UnpackGeneric(ds, _, s) => {
            r_vec(ds, map);
            r(s, map);
        },
        Instr::PackVariant(d, _, fields) | Instr::PackVariantGeneric(d, _, fields) => {
            r(d, map);
            r_vec(fields, map);
        },
        Instr::UnpackVariant(ds, _, s) | Instr::UnpackVariantGeneric(ds, _, s) => {
            r_vec(ds, map);
            r(s, map);
        },
        Instr::TestVariant(d, _, s) | Instr::TestVariantGeneric(d, _, s) => {
            r(d, map);
            r(s, map);
        },

        Instr::ImmBorrowLoc(d, s) | Instr::MutBorrowLoc(d, s) => {
            r(d, map);
            r(s, map);
        },
        Instr::ImmBorrowField(d, _, s)
        | Instr::MutBorrowField(d, _, s)
        | Instr::ImmBorrowFieldGeneric(d, _, s)
        | Instr::MutBorrowFieldGeneric(d, _, s)
        | Instr::ImmBorrowVariantField(d, _, s)
        | Instr::MutBorrowVariantField(d, _, s)
        | Instr::ImmBorrowVariantFieldGeneric(d, _, s)
        | Instr::MutBorrowVariantFieldGeneric(d, _, s) => {
            r(d, map);
            r(s, map);
        },
        Instr::ReadRef(d, s) => {
            r(d, map);
            r(s, map);
        },
        Instr::WriteRef(d, v) => {
            r(d, map);
            r(v, map);
        },

        Instr::ReadField(d, _, s)
        | Instr::ReadFieldGeneric(d, _, s)
        | Instr::ReadVariantField(d, _, s)
        | Instr::ReadVariantFieldGeneric(d, _, s) => {
            r(d, map);
            r(s, map);
        },
        Instr::WriteField(_, dr, v)
        | Instr::WriteFieldGeneric(_, dr, v)
        | Instr::WriteVariantField(_, dr, v)
        | Instr::WriteVariantFieldGeneric(_, dr, v) => {
            r(dr, map);
            r(v, map);
        },

        Instr::Exists(d, _, a)
        | Instr::ExistsGeneric(d, _, a)
        | Instr::MoveFrom(d, _, a)
        | Instr::MoveFromGeneric(d, _, a) => {
            r(d, map);
            r(a, map);
        },
        Instr::MoveTo(_, s, v) | Instr::MoveToGeneric(_, s, v) => {
            r(s, map);
            r(v, map);
        },
        Instr::ImmBorrowGlobal(d, _, a)
        | Instr::ImmBorrowGlobalGeneric(d, _, a)
        | Instr::MutBorrowGlobal(d, _, a)
        | Instr::MutBorrowGlobalGeneric(d, _, a) => {
            r(d, map);
            r(a, map);
        },

        Instr::Call(rets, _, args) | Instr::CallGeneric(rets, _, args) => {
            r_vec(rets, map);
            r_vec(args, map);
        },
        Instr::PackClosure(d, _, _, captured) | Instr::PackClosureGeneric(d, _, _, captured) => {
            r(d, map);
            r_vec(captured, map);
        },
        Instr::CallClosure(rets, _, args) => {
            r_vec(rets, map);
            r_vec(args, map);
        },

        Instr::VecPack(d, _, _, elems) => {
            r(d, map);
            r_vec(elems, map);
        },
        Instr::VecLen(d, _, s) => {
            r(d, map);
            r(s, map);
        },
        Instr::VecImmBorrow(d, _, v, i) | Instr::VecMutBorrow(d, _, v, i) => {
            r(d, map);
            r(v, map);
            r(i, map);
        },
        Instr::VecPushBack(_, v, val) => {
            r(v, map);
            r(val, map);
        },
        Instr::VecPopBack(d, _, s) => {
            r(d, map);
            r(s, map);
        },
        Instr::VecUnpack(ds, _, _, s) => {
            r_vec(ds, map);
            r(s, map);
        },
        Instr::VecSwap(_, v, i, j) => {
            r(v, map);
            r(i, map);
            r(j, map);
        },

        Instr::Label(_) | Instr::Branch(_) => {},
        Instr::BrTrue(_, c) | Instr::BrFalse(_, c) => r(c, map),
        Instr::Ret(rets) => r_vec(rets, map),
        Instr::Abort(c) => r(c, map),
        Instr::AbortMsg(c, m) => {
            r(c, map);
            r(m, map);
        },
    }
}

/// Apply slot substitution to source operands only (for copy propagation).
pub(crate) fn apply_subst_to_sources(instr: &mut Instr, subst: &BTreeMap<Slot, Slot>) {
    fn s(r: &mut Slot, subst: &BTreeMap<Slot, Slot>) {
        if let Some(&replacement) = subst.get(r) {
            *r = replacement;
        }
    }

    fn s_vec(regs: &mut [Slot], subst: &BTreeMap<Slot, Slot>) {
        for r in regs.iter_mut() {
            s(r, subst);
        }
    }

    match instr {
        // For Copy/Move, substitute source only
        Instr::Copy(_, src) | Instr::Move(_, src) => s(src, subst),

        // Unary: substitute source
        Instr::UnaryOp(_, _, src) => s(src, subst),

        // Binary: substitute both sources
        Instr::BinaryOp(_, _, lhs, rhs) => {
            s(lhs, subst);
            s(rhs, subst);
        },

        // Binary immediate: substitute lhs only (immediate has no slot)
        Instr::BinaryOpImm(_, _, lhs, _) => s(lhs, subst),

        // Struct ops
        Instr::Pack(_, _, fields) | Instr::PackGeneric(_, _, fields) => s_vec(fields, subst),
        Instr::Unpack(_, _, src) | Instr::UnpackGeneric(_, _, src) => s(src, subst),

        // Variant ops
        Instr::PackVariant(_, _, fields) | Instr::PackVariantGeneric(_, _, fields) => {
            s_vec(fields, subst)
        },
        Instr::UnpackVariant(_, _, src) | Instr::UnpackVariantGeneric(_, _, src) => s(src, subst),
        Instr::TestVariant(_, _, src) | Instr::TestVariantGeneric(_, _, src) => s(src, subst),

        // References — BorrowLoc sources are storage-location uses (identity of the
        // slot matters), NOT value uses. Substituting them would change which frame
        // slot the reference points to, which is unsound.
        Instr::ImmBorrowLoc(_, _) | Instr::MutBorrowLoc(_, _) => {},
        Instr::ImmBorrowField(_, _, src)
        | Instr::MutBorrowField(_, _, src)
        | Instr::ImmBorrowFieldGeneric(_, _, src)
        | Instr::MutBorrowFieldGeneric(_, _, src)
        | Instr::ImmBorrowVariantField(_, _, src)
        | Instr::MutBorrowVariantField(_, _, src)
        | Instr::ImmBorrowVariantFieldGeneric(_, _, src)
        | Instr::MutBorrowVariantFieldGeneric(_, _, src) => s(src, subst),
        Instr::ReadRef(_, src) => s(src, subst),
        Instr::WriteRef(dst_ref, val) => {
            s(dst_ref, subst);
            s(val, subst);
        },

        // Fused field access
        Instr::ReadField(_, _, src)
        | Instr::ReadFieldGeneric(_, _, src)
        | Instr::ReadVariantField(_, _, src)
        | Instr::ReadVariantFieldGeneric(_, _, src) => s(src, subst),
        Instr::WriteField(_, dst_ref, val)
        | Instr::WriteFieldGeneric(_, dst_ref, val)
        | Instr::WriteVariantField(_, dst_ref, val)
        | Instr::WriteVariantFieldGeneric(_, dst_ref, val) => {
            s(dst_ref, subst);
            s(val, subst);
        },

        // Globals
        Instr::Exists(_, _, addr)
        | Instr::ExistsGeneric(_, _, addr)
        | Instr::MoveFrom(_, _, addr)
        | Instr::MoveFromGeneric(_, _, addr) => s(addr, subst),
        Instr::MoveTo(_, signer, val) | Instr::MoveToGeneric(_, signer, val) => {
            s(signer, subst);
            s(val, subst);
        },
        Instr::ImmBorrowGlobal(_, _, addr)
        | Instr::ImmBorrowGlobalGeneric(_, _, addr)
        | Instr::MutBorrowGlobal(_, _, addr)
        | Instr::MutBorrowGlobalGeneric(_, _, addr) => s(addr, subst),

        // Calls
        Instr::Call(_, _, args) | Instr::CallGeneric(_, _, args) => s_vec(args, subst),
        Instr::PackClosure(_, _, _, captured) | Instr::PackClosureGeneric(_, _, _, captured) => {
            s_vec(captured, subst)
        },
        Instr::CallClosure(_, _, args) => s_vec(args, subst),

        // Vector
        Instr::VecPack(_, _, _, elems) => s_vec(elems, subst),
        Instr::VecLen(_, _, src) => s(src, subst),
        Instr::VecImmBorrow(_, _, vec_ref, idx) | Instr::VecMutBorrow(_, _, vec_ref, idx) => {
            s(vec_ref, subst);
            s(idx, subst);
        },
        Instr::VecPushBack(_, vec_ref, val) => {
            s(vec_ref, subst);
            s(val, subst);
        },
        Instr::VecPopBack(_, _, src) => s(src, subst),
        Instr::VecUnpack(_, _, _, src) => s(src, subst),
        Instr::VecSwap(_, vec_ref, i, j) => {
            s(vec_ref, subst);
            s(i, subst);
            s(j, subst);
        },

        // Control flow
        Instr::BrTrue(_, cond) | Instr::BrFalse(_, cond) => s(cond, subst),
        Instr::Ret(rets) => s_vec(rets, subst),
        Instr::Abort(code) => s(code, subst),
        Instr::AbortMsg(code, msg) => {
            s(code, subst);
            s(msg, subst);
        },

        // No sources to substitute
        Instr::Label(_)
        | Instr::Branch(_)
        | Instr::LdConst(_, _)
        | Instr::LdTrue(_)
        | Instr::LdFalse(_)
        | Instr::LdU8(_, _)
        | Instr::LdU16(_, _)
        | Instr::LdU32(_, _)
        | Instr::LdU64(_, _)
        | Instr::LdU128(_, _)
        | Instr::LdU256(_, _)
        | Instr::LdI8(_, _)
        | Instr::LdI16(_, _)
        | Instr::LdI32(_, _)
        | Instr::LdI64(_, _)
        | Instr::LdI128(_, _)
        | Instr::LdI256(_, _) => {},
    }
}

/// Split instructions into basic blocks (label to branch/label).
/// Returns half-open `start..end` ranges.
pub(crate) fn split_into_blocks(instrs: &[Instr]) -> Vec<Range<usize>> {
    let mut blocks = Vec::new();
    let mut start = 0;

    for (i, instr) in instrs.iter().enumerate() {
        match instr {
            Instr::Label(_) if i > start => {
                blocks.push(start..i);
                start = i;
            },
            Instr::Branch(_)
            | Instr::BrTrue(_, _)
            | Instr::BrFalse(_, _)
            | Instr::Ret(_)
            | Instr::Abort(_)
            | Instr::AbortMsg(_, _) => {
                blocks.push(start..i + 1);
                start = i + 1;
            },
            _ => {},
        }
    }

    if start < instrs.len() {
        blocks.push(start..instrs.len());
    }

    blocks
}

/// Extract the destination slot and immediate value from a load instruction.
/// Returns None for LdU128/LdU256/LdI128/LdI256/LdConst (too large or non-numeric).
pub(crate) fn extract_imm_value(instr: &Instr) -> Option<(Slot, ImmValue)> {
    match instr {
        Instr::LdTrue(d) => Some((*d, ImmValue::Bool(true))),
        Instr::LdFalse(d) => Some((*d, ImmValue::Bool(false))),
        Instr::LdU8(d, v) => Some((*d, ImmValue::U8(*v))),
        Instr::LdU16(d, v) => Some((*d, ImmValue::U16(*v))),
        Instr::LdU32(d, v) => Some((*d, ImmValue::U32(*v))),
        Instr::LdU64(d, v) => Some((*d, ImmValue::U64(*v))),
        Instr::LdI8(d, v) => Some((*d, ImmValue::I8(*v))),
        Instr::LdI16(d, v) => Some((*d, ImmValue::I16(*v))),
        Instr::LdI32(d, v) => Some((*d, ImmValue::I32(*v))),
        Instr::LdI64(d, v) => Some((*d, ImmValue::I64(*v))),
        _ => None,
    }
}

/// Whether a binary operation is commutative (operands can be swapped).
pub(crate) fn is_commutative(op: &BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Add
            | BinaryOp::Mul
            | BinaryOp::BitOr
            | BinaryOp::BitAnd
            | BinaryOp::Xor
            | BinaryOp::Eq
            | BinaryOp::Neq
            | BinaryOp::Or
            | BinaryOp::And
    )
}
