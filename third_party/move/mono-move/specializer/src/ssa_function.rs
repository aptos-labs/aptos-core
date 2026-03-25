// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Intermediate SSA representation and pre-allocation fusion passes.

use crate::{
    instr_utils::{extract_imm_value, get_defs_uses, is_commutative, split_into_blocks},
    ir::Instr,
};
use move_vm_types::loaded_data::runtime_types::Type;

/// Intermediate SSA representation of a single function, before slot allocation.
pub(crate) struct SSAFunction {
    /// Stackless instructions in SSA form.
    pub instrs: Vec<Instr>,
    /// Type of each value ID, indexed directly by the value ID number.
    pub vid_types: Vec<Type>,
    /// Types of all locals (params ++ declared locals).
    pub local_types: Vec<Type>,
}

impl SSAFunction {
    /// Fuse consecutive borrow+deref patterns into combined field access instructions.
    ///
    /// Safety: relies on the SSA single-use invariant — each `Vid` produced by a
    /// borrow instruction is consumed exactly once by the immediately following
    /// `ReadRef`/`WriteRef`. This holds for verified stack-machine bytecode.
    pub(crate) fn fuse_field_access_instrs(&mut self) {
        let instrs = &self.instrs;
        let mut result = Vec::with_capacity(instrs.len());
        let mut skip_next = false;

        for w in instrs.windows(2) {
            if skip_next {
                skip_next = false;
                continue;
            }

            let fused = match (&w[0], &w[1]) {
                (Instr::ImmBorrowField(ref_r, fld, src), Instr::ReadRef(dst, read_src))
                    if *ref_r == *read_src =>
                {
                    Some(Instr::ReadField(*dst, *fld, *src))
                },
                (Instr::ImmBorrowFieldGeneric(ref_r, fld, src), Instr::ReadRef(dst, read_src))
                    if *ref_r == *read_src =>
                {
                    Some(Instr::ReadFieldGeneric(*dst, *fld, *src))
                },
                (Instr::MutBorrowField(ref_r, fld, dst_ref), Instr::WriteRef(write_ref, val))
                    if *ref_r == *write_ref =>
                {
                    Some(Instr::WriteField(*fld, *dst_ref, *val))
                },
                (
                    Instr::MutBorrowFieldGeneric(ref_r, fld, dst_ref),
                    Instr::WriteRef(write_ref, val),
                ) if *ref_r == *write_ref => Some(Instr::WriteFieldGeneric(*fld, *dst_ref, *val)),
                (Instr::ImmBorrowVariantField(ref_r, fld, src), Instr::ReadRef(dst, read_src))
                    if *ref_r == *read_src =>
                {
                    Some(Instr::ReadVariantField(*dst, *fld, *src))
                },
                (
                    Instr::ImmBorrowVariantFieldGeneric(ref_r, fld, src),
                    Instr::ReadRef(dst, read_src),
                ) if *ref_r == *read_src => Some(Instr::ReadVariantFieldGeneric(*dst, *fld, *src)),
                (
                    Instr::MutBorrowVariantField(ref_r, fld, dst_ref),
                    Instr::WriteRef(write_ref, val),
                ) if *ref_r == *write_ref => Some(Instr::WriteVariantField(*fld, *dst_ref, *val)),
                (
                    Instr::MutBorrowVariantFieldGeneric(ref_r, fld, dst_ref),
                    Instr::WriteRef(write_ref, val),
                ) if *ref_r == *write_ref => {
                    Some(Instr::WriteVariantFieldGeneric(*fld, *dst_ref, *val))
                },
                _ => None,
            };

            if let Some(fused_instr) = fused {
                result.push(fused_instr);
                skip_next = true;
            } else {
                result.push(w[0].clone());
            }
        }

        // The last instruction is only the second element of the final window,
        // so it's never pushed as w[0]. Emit it unless it was consumed by fusion.
        if !skip_next {
            if let Some(last) = instrs.last() {
                result.push(last.clone());
            }
        }

        self.instrs = result;
    }

    /// Fuse consecutive `Ld*` + `BinaryOp` pairs into `BinaryOpImm` in the SSA IR.
    pub(crate) fn fuse_immediate_binops(&mut self) {
        let instrs = &self.instrs;
        let blocks = split_into_blocks(instrs);
        let mut result = Vec::with_capacity(instrs.len());

        let mut block_idx = 0;
        let mut skip_next = false;

        for i in 0..instrs.len() {
            if skip_next {
                skip_next = false;
                continue;
            }

            // Advance to the current block.
            while block_idx < blocks.len() && i >= blocks[block_idx].end {
                block_idx += 1;
            }

            // Only fuse within the same basic block.
            if i + 1 < instrs.len()
                && block_idx < blocks.len()
                && i + 1 < blocks[block_idx].end
                && let Some((tmp, imm)) = extract_imm_value(&instrs[i])
            {
                let fused = match &instrs[i + 1] {
                    Instr::BinaryOp(dst, op, lhs, rhs) if *rhs == tmp => {
                        Some(Instr::BinaryOpImm(*dst, op.clone(), *lhs, imm.clone()))
                    },
                    Instr::BinaryOp(dst, op, lhs, rhs) if *lhs == tmp && is_commutative(op) => {
                        Some(Instr::BinaryOpImm(*dst, op.clone(), *rhs, imm.clone()))
                    },
                    _ => None,
                };
                if let Some(fused_instr) = fused {
                    debug_assert!(
                        {
                            let block = &blocks[block_idx];
                            instrs[block.clone()]
                                .iter()
                                .enumerate()
                                .filter(|&(j, _)| block.start + j != i + 1)
                                .all(|(_, ins)| {
                                    let (_, uses) = get_defs_uses(ins);
                                    !uses.contains(&tmp)
                                })
                        },
                        "BinaryOpImm SSA fusion: VID {:?} has uses outside the \
                         consecutive Ld+BinaryOp pair — stack machine invariant violated",
                        tmp,
                    );
                    result.push(fused_instr);
                    skip_next = true;
                    continue;
                }
            }

            result.push(instrs[i].clone());
        }

        self.instrs = result;
    }
}
