// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Greedy register allocation.
//!
//! Consumes `BlockAnalysis` from `analysis` and maps SSA temp VIDs to
//! physical Home/Arg registers using liveness-driven type-keyed reuse.

use anyhow::{bail, Context, Result};
use crate::analysis::analyze_block;
use crate::instr_utils::{get_defs_uses, rename_instr, split_into_blocks};
use crate::ir::{Instr, Reg};
use move_vm_types::loaded_data::runtime_types::Type;
use std::collections::BTreeMap;

/// Map SSA VIDs to physical registers across all blocks.
///
/// Pre: SSA instruction stream after Pass 1.5; vid_types maps each temp VID
///      (Home(num_pinned + i)) to its type at index i.
/// Post: all temp VIDs replaced with physical Home/Arg registers.
pub(crate) fn allocate_registers(
    instrs: &[Instr],
    num_pinned: u16,
    local_types: &[Type],
    vid_types: &[Type],
) -> Result<(Vec<Instr>, u16, u16, Vec<Type>)> {
    let blocks = split_into_blocks(instrs);
    let mut result = Vec::with_capacity(instrs.len());
    let mut global_next_reg = num_pinned;
    let mut global_num_arg_regs: u16 = 0;
    let mut free_pool: BTreeMap<Type, Vec<Reg>> = BTreeMap::new();
    let mut phys_reg_types: BTreeMap<Reg, Type> = BTreeMap::new();
    for (i, ty) in local_types.iter().enumerate() {
        phys_reg_types.insert(Reg::Home(i as u16), ty.clone());
    }

    for (start, end) in blocks {
        let block_instrs = &instrs[start..end];
        let analysis = analyze_block(block_instrs, num_pinned);
        let (allocated, block_max, block_arg_regs, returned_pool) = allocate_block(
            block_instrs,
            num_pinned,
            global_next_reg,
            free_pool,
            vid_types,
            &mut phys_reg_types,
            &analysis,
        )?;
        free_pool = returned_pool;
        if block_max > global_next_reg {
            global_next_reg = block_max;
        }
        if block_arg_regs > global_num_arg_regs {
            global_num_arg_regs = block_arg_regs;
        }
        result.extend(allocated);
    }

    let mut reg_types = Vec::with_capacity(global_next_reg as usize);
    for i in 0..global_next_reg {
        reg_types.push(
            phys_reg_types
                .get(&Reg::Home(i))
                .cloned()
                .context("missing type for physical register")?,
        );
    }

    Ok((result, global_next_reg, global_num_arg_regs, reg_types))
}

fn vid_type(vid: Reg, num_pinned: u16, vid_types: &[Type]) -> Result<Type> {
    match vid {
        Reg::Home(i) if i >= num_pinned => vid_types
            .get((i - num_pinned) as usize)
            .cloned()
            .context("VID type not found during SSA allocation"),
        _ => bail!("vid_type called on non-temp register {:?}", vid),
    }
}

fn allocate_block(
    instrs: &[Instr],
    num_pinned: u16,
    start_reg: u16,
    carry_pool: BTreeMap<Type, Vec<Reg>>,
    vid_types: &[Type],
    phys_reg_types: &mut BTreeMap<Reg, Type>,
    analysis: &crate::analysis::BlockAnalysis,
) -> Result<(Vec<Instr>, u16, u16, BTreeMap<Type, Vec<Reg>>)> {
    if instrs.is_empty() {
        return Ok((Vec::new(), start_reg, 0, carry_pool));
    }

    let is_temp_vid = |r: &Reg| -> bool { r.is_temp(num_pinned) };

    let mut vid_to_phys: BTreeMap<Reg, Reg> = BTreeMap::new();
    for r in 0..num_pinned {
        vid_to_phys.insert(Reg::Home(r), Reg::Home(r));
    }
    let mut free_pool = carry_pool;
    let mut next_reg = start_reg;

    let mut output = Vec::with_capacity(instrs.len());

    for (i, instr) in instrs.iter().enumerate() {
        let mut mapped_instr = instr.clone();
        let (defs, uses) = get_defs_uses(instr);

        // Free use-registers whose last use is this instruction BEFORE allocating
        // for defs. This is safe because IR instruction semantics guarantee all
        // sources are read before any destination is written, so reusing a source
        // register as a destination (e.g. `r2 := add r2, a0`) is correct: the old
        // value of r2 is consumed before the result overwrites it. In SSA form,
        // a temp VID is defined exactly once and cannot appear as both a def and
        // use of the same instruction, so there is no risk of freeing a register
        // that is also being defined here.
        for r in &uses {
            if is_temp_vid(r)
                && analysis.last_use.get(r) == Some(&i)
                && !defs.contains(r)
                && let Some(&phys) = vid_to_phys.get(r)
                && phys.is_temp(num_pinned)
            {
                let ty = phys_reg_types
                    .get(&phys)
                    .cloned()
                    .unwrap_or(Type::Bool);
                free_pool.entry(ty).or_default().push(phys);
            }
        }

        // Allocate physical registers for destination vids
        for d in &defs {
            if is_temp_vid(d) && !vid_to_phys.contains_key(d) {
                if let Some(&arg_r) = analysis.arg_precolor.get(d) {
                    vid_to_phys.insert(*d, arg_r);
                } else if let Some(&local_r) = analysis.stloc_targets.get(d) {
                    vid_to_phys.insert(*d, local_r);
                } else if let Some(&local_r) = analysis.coalesce_to_local.get(d) {
                    vid_to_phys.insert(*d, local_r);
                } else {
                    let ty = vid_type(*d, num_pinned, vid_types)?;
                    let phys = if let Some(regs) = free_pool.get_mut(&ty) {
                        regs.pop()
                    } else {
                        None
                    };
                    let phys = phys.unwrap_or_else(|| {
                        let r = Reg::Home(next_reg);
                        next_reg += 1;
                        phys_reg_types.insert(r, ty);
                        r
                    });
                    vid_to_phys.insert(*d, phys);
                }
            }
        }

        rename_instr(&mut mapped_instr, &vid_to_phys);
        output.push(mapped_instr);

        // Free registers for defs that are never used (last_use == def site).
        for d in &defs {
            if is_temp_vid(d)
                && analysis.last_use.get(d) == Some(&i)
            {
                let (_, ref uses_list) = get_defs_uses(instr);
                if !uses_list.contains(d)
                    && let Some(&phys) = vid_to_phys.get(d)
                    && phys.is_temp(num_pinned)
                {
                    let ty = phys_reg_types
                        .get(&phys)
                        .cloned()
                        .unwrap_or(Type::Bool);
                    free_pool.entry(ty).or_default().push(phys);
                }
            }
        }
    }

    Ok((output, next_reg, analysis.max_arg_width, free_pool))
}
