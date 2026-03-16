// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! V2 post-allocation optimization passes.
//!
//! Self-contained — uses only `instr_utils_v2`, no dependency on optimize_v1.
//!
//! Pass 3: Copy propagation
//! Pass 4: Identity move elimination
//! Pass 5: Dead instruction elimination
//! Pass 6: Register renumbering

use crate::instr_utils_v2::{
    apply_subst_to_sources, get_defs_uses, rename_instr, split_into_blocks,
};
use crate::ir::{FunctionIR, Instr, ModuleIR, Reg};
use std::collections::{BTreeMap, BTreeSet};

/// Optimize all functions in a module IR using the v2 pipeline.
pub fn optimize_module_v2(module_ir: &mut ModuleIR) {
    for func in &mut module_ir.functions {
        eliminate_identity_moves(func);
        copy_propagation(func);
        eliminate_identity_moves(func);
        dead_instruction_elimination(func);
        renumber_registers(func);
    }
}

/// Pass 3: Forward copy propagation within each basic block.
///
/// Pre: allocated instruction stream (physical registers).
/// Post: Copy/Move sources propagated to downstream uses; no instructions removed.
fn copy_propagation(func: &mut FunctionIR) {
    let num_pinned = func.num_params + func.num_locals;
    let blocks = split_into_blocks(&func.instrs);

    for (start, end) in blocks {
        let mut subst: BTreeMap<Reg, Reg> = BTreeMap::new();

        for i in start..end {
            apply_subst_to_sources(&mut func.instrs[i], &subst);

            let (defs, _) = get_defs_uses(&func.instrs[i]);
            for d in &defs {
                subst.remove(d);
                subst.retain(|_, v| v != d);
            }

            match &func.instrs[i] {
                Instr::Copy(dst, src) | Instr::Move(dst, src)
                    if matches!(dst, Reg::Home(i) if *i >= num_pinned) || dst.is_arg() =>
                {
                    subst.insert(*dst, *src);
                },
                _ => {},
            }
        }
    }
}

/// Pass 4: Remove `Move(r, r)` and `Copy(r, r)` instructions.
fn eliminate_identity_moves(func: &mut FunctionIR) {
    func.instrs.retain(|instr| {
        !matches!(instr, Instr::Move(d, s) | Instr::Copy(d, s) if d == s)
    });
}

/// Pass 5: Backward dead-code elimination within each basic block.
///
/// Pre: after copy propagation and identity move elimination.
/// Post: dead Copy/Move to non-pinned registers removed.
fn dead_instruction_elimination(func: &mut FunctionIR) {
    let num_pinned = func.num_params + func.num_locals;
    let blocks = split_into_blocks(&func.instrs);

    let mut dead_indices: BTreeSet<usize> = BTreeSet::new();

    for (start, end) in blocks {
        let mut live: BTreeSet<Reg> = BTreeSet::new();

        for i in (start..end).rev() {
            let (dsts, srcs) = get_defs_uses(&func.instrs[i]);

            let is_removable = match &func.instrs[i] {
                Instr::Copy(dst, _) | Instr::Move(dst, _)
                    if matches!(dst, Reg::Home(i) if *i >= num_pinned) || dst.is_arg() =>
                {
                    !live.contains(dst)
                },
                _ => false,
            };

            if is_removable {
                dead_indices.insert(i);
            } else {
                for d in &dsts {
                    live.remove(d);
                }
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

/// Pass 6: Compact Home register indices while preserving pinned registers.
///
/// Pre: after DCE (some registers may be unused).
/// Post: Home registers renumbered contiguously starting at num_pinned;
///       reg_types and num_regs updated.
fn renumber_registers(func: &mut FunctionIR) {
    let num_pinned = func.num_params + func.num_locals;

    let mut used_home_regs: BTreeSet<u16> = BTreeSet::new();
    for instr in &func.instrs {
        let (defs, uses_) = get_defs_uses(instr);
        for r in defs.into_iter().chain(uses_) {
            if let Reg::Home(i) = r {
                used_home_regs.insert(i);
            }
        }
    }

    let mut rename_map: BTreeMap<Reg, Reg> = BTreeMap::new();
    let mut next_reg = num_pinned;
    for &i in &used_home_regs {
        if i < num_pinned {
            rename_map.insert(Reg::Home(i), Reg::Home(i));
        } else {
            rename_map.insert(Reg::Home(i), Reg::Home(next_reg));
            next_reg += 1;
        }
    }

    for instr in &mut func.instrs {
        rename_instr(instr, &rename_map);
    }

    let mut new_reg_types = vec![move_vm_types::loaded_data::runtime_types::Type::Bool; next_reg as usize];
    for (&old, &new) in &rename_map {
        if let (Reg::Home(old_i), Reg::Home(new_i)) = (old, new)
            && (old_i as usize) < func.reg_types.len()
        {
            new_reg_types[new_i as usize] = func.reg_types[old_i as usize].clone();
        }
    }
    func.reg_types = new_reg_types;

    func.num_regs = next_reg;
}
