// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Instruction utilities for the pipeline.
//!
//! Helpers used by `convert`, `regalloc`, and `optimize`.

use crate::ir::{BinaryOp, ImmValue, Instr, Reg};
use std::collections::BTreeMap;

/// Get registers defined (written) and used (read) by an instruction.
pub(crate) fn get_defs_uses(instr: &Instr) -> (Vec<Reg>, Vec<Reg>) {
    let mut defs = Vec::new();
    let mut uses = Vec::new();

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
        | Instr::LdI256(d, _) => {
            defs.push(*d);
        },

        Instr::Copy(d, s) | Instr::Move(d, s) => {
            defs.push(*d);
            uses.push(*s);
        },

        Instr::UnaryOp(d, _, s) => {
            defs.push(*d);
            uses.push(*s);
        },
        Instr::BinaryOp(d, _, l, r) => {
            defs.push(*d);
            uses.push(*l);
            uses.push(*r);
        },
        Instr::BinaryOpImm(d, _, l, _) => {
            defs.push(*d);
            uses.push(*l);
        },

        Instr::Pack(d, _, fields) | Instr::PackGeneric(d, _, fields) => {
            defs.push(*d);
            uses.extend_from_slice(fields);
        },
        Instr::Unpack(ds, _, s) | Instr::UnpackGeneric(ds, _, s) => {
            defs.extend_from_slice(ds);
            uses.push(*s);
        },

        Instr::PackVariant(d, _, fields) | Instr::PackVariantGeneric(d, _, fields) => {
            defs.push(*d);
            uses.extend_from_slice(fields);
        },
        Instr::UnpackVariant(ds, _, s) | Instr::UnpackVariantGeneric(ds, _, s) => {
            defs.extend_from_slice(ds);
            uses.push(*s);
        },
        Instr::TestVariant(d, _, s) | Instr::TestVariantGeneric(d, _, s) => {
            defs.push(*d);
            uses.push(*s);
        },

        Instr::ImmBorrowLoc(d, s) | Instr::MutBorrowLoc(d, s) => {
            defs.push(*d);
            uses.push(*s);
        },
        Instr::ImmBorrowField(d, _, s)
        | Instr::MutBorrowField(d, _, s)
        | Instr::ImmBorrowFieldGeneric(d, _, s)
        | Instr::MutBorrowFieldGeneric(d, _, s)
        | Instr::ImmBorrowVariantField(d, _, s)
        | Instr::MutBorrowVariantField(d, _, s)
        | Instr::ImmBorrowVariantFieldGeneric(d, _, s)
        | Instr::MutBorrowVariantFieldGeneric(d, _, s) => {
            defs.push(*d);
            uses.push(*s);
        },
        Instr::ReadRef(d, s) => {
            defs.push(*d);
            uses.push(*s);
        },
        Instr::WriteRef(r, v) => {
            uses.push(*r);
            uses.push(*v);
        },

        Instr::ReadField(d, _, s)
        | Instr::ReadFieldGeneric(d, _, s)
        | Instr::ReadVariantField(d, _, s)
        | Instr::ReadVariantFieldGeneric(d, _, s) => {
            defs.push(*d);
            uses.push(*s);
        },
        Instr::WriteField(_, r, v)
        | Instr::WriteFieldGeneric(_, r, v)
        | Instr::WriteVariantField(_, r, v)
        | Instr::WriteVariantFieldGeneric(_, r, v) => {
            uses.push(*r);
            uses.push(*v);
        },

        Instr::Exists(d, _, a)
        | Instr::ExistsGeneric(d, _, a)
        | Instr::MoveFrom(d, _, a)
        | Instr::MoveFromGeneric(d, _, a) => {
            defs.push(*d);
            uses.push(*a);
        },
        Instr::MoveTo(_, s, v) | Instr::MoveToGeneric(_, s, v) => {
            uses.push(*s);
            uses.push(*v);
        },
        Instr::ImmBorrowGlobal(d, _, a)
        | Instr::ImmBorrowGlobalGeneric(d, _, a)
        | Instr::MutBorrowGlobal(d, _, a)
        | Instr::MutBorrowGlobalGeneric(d, _, a) => {
            defs.push(*d);
            uses.push(*a);
        },

        Instr::Call(rets, _, args) | Instr::CallGeneric(rets, _, args) => {
            defs.extend_from_slice(rets);
            uses.extend_from_slice(args);
        },
        Instr::PackClosure(d, _, _, captured)
        | Instr::PackClosureGeneric(d, _, _, captured) => {
            defs.push(*d);
            uses.extend_from_slice(captured);
        },
        Instr::CallClosure(rets, _, args) => {
            defs.extend_from_slice(rets);
            uses.extend_from_slice(args);
        },

        Instr::VecPack(d, _, _, elems) => {
            defs.push(*d);
            uses.extend_from_slice(elems);
        },
        Instr::VecLen(d, _, s) => {
            defs.push(*d);
            uses.push(*s);
        },
        Instr::VecImmBorrow(d, _, v, i) | Instr::VecMutBorrow(d, _, v, i) => {
            defs.push(*d);
            uses.push(*v);
            uses.push(*i);
        },
        Instr::VecPushBack(_, v, val) => {
            uses.push(*v);
            uses.push(*val);
        },
        Instr::VecPopBack(d, _, s) => {
            defs.push(*d);
            uses.push(*s);
        },
        Instr::VecUnpack(ds, _, _, s) => {
            defs.extend_from_slice(ds);
            uses.push(*s);
        },
        Instr::VecSwap(_, v, i, j) => {
            uses.push(*v);
            uses.push(*i);
            uses.push(*j);
        },

        Instr::Label(_) | Instr::Branch(_) => {},
        Instr::BrTrue(_, c) | Instr::BrFalse(_, c) => {
            uses.push(*c);
        },
        Instr::Ret(rets) => {
            uses.extend_from_slice(rets);
        },
        Instr::Abort(c) => {
            uses.push(*c);
        },
        Instr::AbortMsg(c, m) => {
            uses.push(*c);
            uses.push(*m);
        },
    }

    (defs, uses)
}

/// Apply register renaming to all operands of an instruction.
pub(crate) fn rename_instr(instr: &mut Instr, map: &BTreeMap<Reg, Reg>) {
    fn r(reg: &mut Reg, map: &BTreeMap<Reg, Reg>) {
        if let Some(&new) = map.get(reg) {
            *reg = new;
        }
    }

    fn r_vec(regs: &mut [Reg], map: &BTreeMap<Reg, Reg>) {
        for reg in regs.iter_mut() {
            r(reg, map);
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

/// Apply register substitution to source operands only (for copy propagation).
pub(crate) fn apply_subst_to_sources(instr: &mut Instr, subst: &BTreeMap<Reg, Reg>) {
    fn s(r: &mut Reg, subst: &BTreeMap<Reg, Reg>) {
        if let Some(&replacement) = subst.get(r) {
            *r = replacement;
        }
    }

    fn s_vec(regs: &mut [Reg], subst: &BTreeMap<Reg, Reg>) {
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

        // Binary immediate: substitute lhs only (immediate has no register)
        Instr::BinaryOpImm(_, _, lhs, _) => s(lhs, subst),

        // Struct ops
        Instr::Pack(_, _, fields) | Instr::PackGeneric(_, _, fields) => s_vec(fields, subst),
        Instr::Unpack(_, _, src) | Instr::UnpackGeneric(_, _, src) => s(src, subst),

        // Variant ops
        Instr::PackVariant(_, _, fields) | Instr::PackVariantGeneric(_, _, fields) => {
            s_vec(fields, subst)
        },
        Instr::UnpackVariant(_, _, src) | Instr::UnpackVariantGeneric(_, _, src) => {
            s(src, subst)
        },
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
/// Returns (start, end) index pairs (exclusive end).
pub(crate) fn split_into_blocks(instrs: &[Instr]) -> Vec<(usize, usize)> {
    let mut blocks = Vec::new();
    let mut start = 0;

    for (i, instr) in instrs.iter().enumerate() {
        match instr {
            Instr::Label(_) if i > start => {
                blocks.push((start, i));
                start = i;
            },
            Instr::Branch(_)
            | Instr::BrTrue(_, _)
            | Instr::BrFalse(_, _)
            | Instr::Ret(_)
            | Instr::Abort(_)
            | Instr::AbortMsg(_, _) => {
                blocks.push((start, i + 1));
                start = i + 1;
            },
            _ => {},
        }
    }

    if start < instrs.len() {
        blocks.push((start, instrs.len()));
    }

    blocks
}

/// Extract the destination register and immediate value from a load instruction.
/// Returns None for LdU128/LdU256/LdI128/LdI256/LdConst (too large or non-numeric).
pub(crate) fn extract_imm_value(instr: &Instr) -> Option<(Reg, ImmValue)> {
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
