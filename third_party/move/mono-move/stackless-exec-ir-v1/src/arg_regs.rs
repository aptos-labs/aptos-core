// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Post-optimization pass that introduces arg registers for V1.
//!
//! Scans each basic block for `Call`/`CallGeneric` instructions and promotes
//! eligible temp registers to `Reg::Arg(j)` so that values produced directly
//! into arg slots avoid a copy at call sites in the downstream micro-op
//! translation.

use crate::{
    ir::{FunctionIR, Instr, ModuleIR, Reg},
    optimize_v1::{get_defs_uses, rename_instr, renumber_registers, split_into_blocks},
};
use std::collections::{BTreeMap, BTreeSet};

/// Introduce arg registers for all functions in a module (V1 pipeline).
pub fn introduce_arg_registers_module(module_ir: &mut ModuleIR) {
    for func in &mut module_ir.functions {
        introduce_arg_registers(func);
    }
}

fn introduce_arg_registers(func: &mut FunctionIR) {
    let num_pinned = func.num_params + func.num_locals;
    let blocks = split_into_blocks(&func.instrs);

    let mut global_rename: BTreeMap<Reg, Reg> = BTreeMap::new();
    let mut num_arg_regs: u16 = 0;

    for (start, end) in &blocks {
        let block = &func.instrs[*start..*end];
        if block.is_empty() {
            continue;
        }

        // Compute def_pos, last_use, and multi_def for temp Home registers.
        // V1's register allocator can reuse a physical register for distinct
        // lifetimes within a block. A global rename would conflate those
        // lifetimes, so we must exclude multiply-defined registers.
        let mut def_pos: BTreeMap<Reg, usize> = BTreeMap::new();
        let mut last_use: BTreeMap<Reg, usize> = BTreeMap::new();
        let mut multi_def: BTreeSet<Reg> = BTreeSet::new();
        for (i, instr) in block.iter().enumerate() {
            let (defs, uses) = get_defs_uses(instr);
            for r in &defs {
                if r.is_temp(num_pinned) {
                    if def_pos.contains_key(r) {
                        multi_def.insert(*r);
                    }
                    def_pos.entry(*r).or_insert(i);
                    last_use.entry(*r).or_insert(i);
                }
            }
            for r in &uses {
                if r.is_temp(num_pinned) {
                    last_use.insert(*r, i);
                }
            }
        }

        // Collect Call/CallGeneric positions (skip CallClosure).
        let call_positions: Vec<usize> = block
            .iter()
            .enumerate()
            .filter(|(_, ins)| matches!(ins, Instr::Call(..) | Instr::CallGeneric(..)))
            .map(|(i, _)| i)
            .collect();

        let has_call_between = |pos_a: usize, pos_b: usize| -> bool {
            call_positions.iter().any(|&cp| cp > pos_a && cp < pos_b)
        };

        for (ci_idx, &ci) in call_positions.iter().enumerate() {
            let (rets, args) = match &block[ci] {
                Instr::Call(rets, _, args) | Instr::CallGeneric(rets, _, args) => {
                    (rets.clone(), args.clone())
                },
                _ => continue,
            };

            let next_call = call_positions
                .get(ci_idx + 1)
                .copied()
                .unwrap_or(block.len());

            let call_width = std::cmp::max(args.len(), rets.len()) as u16;
            if call_width > num_arg_regs {
                num_arg_regs = call_width;
            }

            // Arg precoloring: promote args[j] to Arg(j)
            for (j, vid) in args.iter().enumerate() {
                if !vid.is_temp(num_pinned) {
                    continue;
                }
                if multi_def.contains(vid) {
                    continue;
                }
                if global_rename.contains_key(vid) {
                    continue;
                }
                let dp = match def_pos.get(vid) {
                    Some(&p) => p,
                    None => continue,
                };
                if has_call_between(dp, ci) {
                    continue;
                }
                if last_use.get(vid) != Some(&ci) {
                    continue;
                }
                global_rename.insert(*vid, Reg::Arg(j as u16));
            }

            // Ret precoloring: promote rets[k] to Arg(k)
            for (k, vid) in rets.iter().enumerate() {
                if !vid.is_temp(num_pinned) {
                    continue;
                }
                if multi_def.contains(vid) {
                    continue;
                }
                if global_rename.contains_key(vid) {
                    continue;
                }
                if let Some(&lu) = last_use.get(vid)
                    && lu >= next_call
                {
                    continue;
                }
                global_rename.insert(*vid, Reg::Arg(k as u16));
            }
        }
    }

    if global_rename.is_empty() {
        return;
    }

    // Apply the rename map to all instructions.
    for instr in &mut func.instrs {
        rename_instr(instr, &global_rename);
    }

    func.num_arg_regs = num_arg_regs;

    // Re-run register renumbering to compact Home registers that became unused.
    renumber_registers(func);
}
