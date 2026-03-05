// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Pass 2: Intra-basic-block optimizations on the stackless IR.
//!
//! - Step A: Instruction fusion (borrow+deref → fused field access)
//! - Step B: Copy propagation (forward scan)
//! - Step C: Dead instruction elimination (backward scan)

use crate::ir::{BinaryOp, FunctionIR, ImmValue, Instr, ModuleIR, Reg};
use move_vm_types::loaded_data::runtime_types::Type;
use std::collections::{BTreeMap, BTreeSet};

/// Optimize all functions in a module IR.
pub fn optimize_module_v1(module_ir: &mut ModuleIR) {
    for func in &mut module_ir.functions {
        optimize_function(func);
    }
}

fn optimize_function(func: &mut FunctionIR) {
    fuse_field_access(func);
    fuse_immediate_binops(func);
    copy_propagation(func);
    dead_instruction_elimination(func);
    renumber_registers(func);
}

// ================================================================================================
// Step A: Instruction Fusion
// ================================================================================================

/// Fuse consecutive borrow+deref patterns into combined field access instructions.
pub(crate) fn fuse_field_access(func: &mut FunctionIR) {
    let mut i = 0;
    while i + 1 < func.instrs.len() {
        let fused = match (&func.instrs[i], &func.instrs[i + 1]) {
            // ImmBorrowField + ReadRef → ReadField
            (Instr::ImmBorrowField(ref_r, fld, src), Instr::ReadRef(dst, read_src))
                if *ref_r == *read_src =>
            {
                Some(Instr::ReadField(*dst, *fld, *src))
            },
            // ImmBorrowFieldGeneric + ReadRef → ReadFieldGeneric
            (Instr::ImmBorrowFieldGeneric(ref_r, fld, src), Instr::ReadRef(dst, read_src))
                if *ref_r == *read_src =>
            {
                Some(Instr::ReadFieldGeneric(*dst, *fld, *src))
            },
            // MutBorrowField + WriteRef → WriteField
            (Instr::MutBorrowField(ref_r, fld, dst_ref), Instr::WriteRef(write_ref, val))
                if *ref_r == *write_ref =>
            {
                Some(Instr::WriteField(*fld, *dst_ref, *val))
            },
            // MutBorrowFieldGeneric + WriteRef → WriteFieldGeneric
            (Instr::MutBorrowFieldGeneric(ref_r, fld, dst_ref), Instr::WriteRef(write_ref, val))
                if *ref_r == *write_ref =>
            {
                Some(Instr::WriteFieldGeneric(*fld, *dst_ref, *val))
            },
            // ImmBorrowVariantField + ReadRef → ReadVariantField
            (
                Instr::ImmBorrowVariantField(ref_r, fld, src),
                Instr::ReadRef(dst, read_src),
            ) if *ref_r == *read_src => {
                Some(Instr::ReadVariantField(*dst, *fld, *src))
            },
            // ImmBorrowVariantFieldGeneric + ReadRef → ReadVariantFieldGeneric
            (
                Instr::ImmBorrowVariantFieldGeneric(ref_r, fld, src),
                Instr::ReadRef(dst, read_src),
            ) if *ref_r == *read_src => {
                Some(Instr::ReadVariantFieldGeneric(*dst, *fld, *src))
            },
            // MutBorrowVariantField + WriteRef → WriteVariantField
            (
                Instr::MutBorrowVariantField(ref_r, fld, dst_ref),
                Instr::WriteRef(write_ref, val),
            ) if *ref_r == *write_ref => {
                Some(Instr::WriteVariantField(*fld, *dst_ref, *val))
            },
            // MutBorrowVariantFieldGeneric + WriteRef → WriteVariantFieldGeneric
            (
                Instr::MutBorrowVariantFieldGeneric(ref_r, fld, dst_ref),
                Instr::WriteRef(write_ref, val),
            ) if *ref_r == *write_ref => {
                Some(Instr::WriteVariantFieldGeneric(*fld, *dst_ref, *val))
            },
            _ => None,
        };

        if let Some(fused_instr) = fused {
            func.instrs[i] = fused_instr;
            func.instrs.remove(i + 1);
            // Don't advance i, check if the new instruction can fuse further
        } else {
            i += 1;
        }
    }
}

// ================================================================================================
// Step A2: Immediate Binary Op Fusion
// ================================================================================================

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

/// Fuse consecutive `Ld*` + `BinaryOp` pairs into `BinaryOpImm`.
///
/// Safety: no explicit single-use guard is needed. The stack machine guarantees
/// that consecutive `Ld + BinaryOp` implies the loaded register was pushed and
/// immediately consumed (Move has no dup instruction; reuse requires
/// `st_loc`+`copy_loc` which inserts intervening `Move`/`Copy` IR instructions
/// that break consecutiveness). Block terminators (Branch/Ret/Abort) prevent
/// cross-block fusion even when in-place removal shifts instruction indices.
pub(crate) fn fuse_immediate_binops(func: &mut FunctionIR) {
    let mut i = 0;
    while i + 1 < func.instrs.len() {
        let fused = if let Some((tmp, imm)) = extract_imm_value(&func.instrs[i]) {
            match &func.instrs[i + 1] {
                Instr::BinaryOp(dst, op, _lhs, rhs) if *rhs == tmp => {
                    Some(Instr::BinaryOpImm(*dst, op.clone(), *_lhs, imm))
                },
                Instr::BinaryOp(dst, op, lhs, _rhs)
                    if *lhs == tmp && is_commutative(op) =>
                {
                    Some(Instr::BinaryOpImm(*dst, op.clone(), *_rhs, imm))
                },
                _ => None,
            }
        } else {
            None
        };

        if let Some(fused_instr) = fused {
            func.instrs[i] = fused_instr;
            func.instrs.remove(i + 1);
        } else {
            i += 1;
        }
    }
}

// ================================================================================================
// Step B: Copy Propagation
// ================================================================================================

fn copy_propagation(func: &mut FunctionIR) {
    let num_params = func.num_params;
    let blocks = split_into_blocks(&func.instrs);

    for (start, end) in blocks {
        let mut subst: BTreeMap<Reg, Reg> = BTreeMap::new();

        for i in start..end {
            // Apply substitution to source operands.
            apply_subst_to_sources(&mut func.instrs[i], &subst);

            // Invalidate any substitution whose key is redefined by this instruction.
            let (defs, _) = get_defs_uses(&func.instrs[i]);
            for d in &defs {
                subst.remove(d);
                // Also invalidate any entry that maps to this register,
                // since the value it pointed to is being overwritten.
                subst.retain(|_, v| v != d);
            }

            // Record new substitutions for Copy/Move of non-param registers.
            match &func.instrs[i] {
                Instr::Copy(dst, src) | Instr::Move(dst, src)
                    if *dst >= num_params =>
                {
                    subst.insert(*dst, *src);
                },
                _ => {},
            }
        }
    }
}

/// Apply register substitution to source operands of an instruction.
fn apply_subst_to_sources(instr: &mut Instr, subst: &BTreeMap<Reg, Reg>) {
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

        // References
        Instr::ImmBorrowLoc(_, src) | Instr::MutBorrowLoc(_, src) => s(src, subst),
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

// ================================================================================================
// Step C: Dead Instruction Elimination
// ================================================================================================

pub(crate) fn dead_instruction_elimination(func: &mut FunctionIR) {
    let num_params = func.num_params;
    let blocks = split_into_blocks(&func.instrs);

    let mut dead_indices: BTreeSet<usize> = BTreeSet::new();

    for (start, end) in blocks {
        // Backward scan: track which registers are read before being written.
        let mut live: BTreeSet<Reg> = BTreeSet::new();

        // At block exit, all registers are potentially live (conservative).
        // Only mark Copy/Move to non-param registers as dead if the dst is never read.
        for i in (start..end).rev() {
            let (dsts, srcs) = get_defs_uses(&func.instrs[i]);

            // Check if instruction only writes to dead non-param registers.
            let is_removable = match &func.instrs[i] {
                Instr::Copy(dst, _) | Instr::Move(dst, _) if *dst >= num_params => {
                    !live.contains(dst)
                },
                _ => false,
            };

            if is_removable {
                dead_indices.insert(i);
            } else {
                // Mark destinations as not live (they're being defined here).
                for d in &dsts {
                    live.remove(d);
                }
                // Mark sources as live.
                for s in &srcs {
                    live.insert(*s);
                }
            }
        }
    }

    if !dead_indices.is_empty() {
        let mut new_instrs = Vec::with_capacity(func.instrs.len());
        for (i, instr) in func.instrs.drain(..).enumerate() {
            if !dead_indices.contains(&i) {
                new_instrs.push(instr);
            }
        }
        func.instrs = new_instrs;
    }
}

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

// ================================================================================================
// Register Renumbering
// ================================================================================================

pub(crate) fn renumber_registers(func: &mut FunctionIR) {
    let num_params = func.num_params;

    // Find all registers used.
    let mut used_regs: BTreeSet<Reg> = BTreeSet::new();
    for instr in &func.instrs {
        let (defs, uses_) = get_defs_uses(instr);
        for r in defs {
            used_regs.insert(r);
        }
        for r in uses_ {
            used_regs.insert(r);
        }
    }

    // Params keep their indices (0..num_params-1).
    // All other registers are renumbered contiguously starting at num_params.
    let mut rename_map: BTreeMap<Reg, Reg> = BTreeMap::new();
    let mut next_reg = num_params;
    for &r in &used_regs {
        if r < num_params {
            rename_map.insert(r, r);
        } else {
            rename_map.insert(r, next_reg);
            next_reg += 1;
        }
    }

    // Apply renaming.
    for instr in &mut func.instrs {
        rename_instr(instr, &rename_map);
    }

    // Remap reg_types using the rename map.
    let mut new_reg_types = vec![Type::Bool; next_reg as usize];
    for (&old, &new) in &rename_map {
        if (old as usize) < func.reg_types.len() {
            new_reg_types[new as usize] = func.reg_types[old as usize].clone();
        }
    }
    func.reg_types = new_reg_types;

    func.num_regs = next_reg;
}

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

// ================================================================================================
// Utilities
// ================================================================================================

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
