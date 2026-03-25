// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Greedy slot allocation.
//!
//! Consumes `BlockAnalysis` from `analysis` and maps SSA temp `Vid`s to
//! real `Home`/`Xfer` slots using liveness-driven type-keyed reuse.

use crate::{
    analysis::BlockAnalysis,
    instr_utils::{get_defs_uses, remap_instr, split_into_blocks},
    ir::{Instr, Slot},
    ssa_function::SSAFunction,
};
use anyhow::{bail, Context, Result};
use move_vm_types::loaded_data::runtime_types::Type;
use std::collections::BTreeMap;

/// Output of slot allocation for a single function.
pub(crate) struct AllocatedFunction {
    pub instrs: Vec<Instr>,
    pub num_home_slots: u16,
    pub num_xfer_slots: u16,
    pub home_slot_types: Vec<Type>,
}

/// Map SSA `Vid`s to real slots across all blocks.
///
/// Pre: SSA instruction stream after fusion passes; vid_types maps each `Vid(i)`
///      to its type at index `i`.
/// Post: all `Vid`s replaced with real `Home`/`Xfer` slots.
pub(crate) fn allocate_slots(ssa: &SSAFunction) -> Result<AllocatedFunction> {
    let num_pinned = ssa.local_types.len() as u16;
    let blocks = split_into_blocks(&ssa.instrs);
    let mut result = Vec::with_capacity(ssa.instrs.len());
    let mut global_next_slot = num_pinned;
    let mut global_num_xfer_slots: u16 = 0;
    let mut free_pool: BTreeMap<Type, Vec<Slot>> = BTreeMap::new();
    let mut real_slot_types: BTreeMap<Slot, Type> = BTreeMap::new();
    for (i, ty) in ssa.local_types.iter().enumerate() {
        real_slot_types.insert(Slot::Home(i as u16), ty.clone());
    }

    for block in blocks {
        let block_instrs = &ssa.instrs[block.clone()];
        let analysis = BlockAnalysis::analyze(block_instrs);
        let (allocated, block_max, block_xfer_slots, returned_pool) = allocate_block(
            block_instrs,
            num_pinned,
            global_next_slot,
            free_pool,
            &ssa.vid_types,
            &mut real_slot_types,
            &analysis,
        )?;
        free_pool = returned_pool;
        if block_max > global_next_slot {
            global_next_slot = block_max;
        }
        if block_xfer_slots > global_num_xfer_slots {
            global_num_xfer_slots = block_xfer_slots;
        }
        result.extend(allocated);
    }

    let mut home_slot_types = Vec::with_capacity(global_next_slot as usize);
    for i in 0..global_next_slot {
        home_slot_types.push(
            real_slot_types
                .get(&Slot::Home(i))
                .cloned()
                .context("missing type for real slot")?,
        );
    }

    Ok(AllocatedFunction {
        instrs: result,
        num_home_slots: global_next_slot,
        num_xfer_slots: global_num_xfer_slots,
        home_slot_types,
    })
}

fn vid_type(vid: Slot, vid_types: &[Type]) -> Result<Type> {
    match vid {
        Slot::Vid(i) => vid_types
            .get(i as usize)
            .cloned()
            .context("VID type not found during SSA allocation"),
        _ => bail!("vid_type called on non-Vid slot {:?}", vid),
    }
}

/// Allocate real slots for a single basic block.
///
/// For each instruction, in order: free last-use sources, allocate defs, remap, free dead defs.
/// Allocation priority: xfer_precolor > stloc_targets > coalesce_to_local > type-keyed reuse > fresh.
///
/// Returns (remapped instrs, next available slot index, xfer width, updated free pool).
fn allocate_block(
    instrs: &[Instr],
    num_pinned: u16,
    start_slot: u16,
    carry_pool: BTreeMap<Type, Vec<Slot>>,
    vid_types: &[Type],
    real_slot_types: &mut BTreeMap<Slot, Type>,
    analysis: &crate::analysis::BlockAnalysis,
) -> Result<(Vec<Instr>, u16, u16, BTreeMap<Type, Vec<Slot>>)> {
    if instrs.is_empty() {
        return Ok((Vec::new(), start_slot, 0, carry_pool));
    }

    // Identity mapping for pinned locals so remap_instr leaves them unchanged.
    let mut vid_to_real: BTreeMap<Slot, Slot> = BTreeMap::new();
    for r in 0..num_pinned {
        vid_to_real.insert(Slot::Home(r), Slot::Home(r));
    }
    let mut free_pool = carry_pool;
    let mut next_slot = start_slot;

    let mut output = Vec::with_capacity(instrs.len());

    for (i, instr) in instrs.iter().enumerate() {
        let mut mapped_instr = instr.clone();
        let (defs, uses) = get_defs_uses(instr);

        // Phase 1: Free use-slots whose last use is this instruction.
        // Done BEFORE def allocation so the freed slot can be immediately reused.
        // Safe because sources are read before destinations are written.
        // Only non-pinned Home slots are pooled (pinned locals and Xfer slots are not).
        for r in &uses {
            if r.is_vid()
                && analysis.last_use.get(r) == Some(&i)
                && !defs.contains(r)
                && let Some(&real) = vid_to_real.get(r)
                && matches!(real, Slot::Home(i) if i >= num_pinned)
            {
                let ty = real_slot_types.get(&real).cloned().unwrap_or(Type::Bool);
                free_pool.entry(ty).or_default().push(real);
            }
        }

        // Phase 2: Allocate real slots for destination `Vid`s.
        for d in &defs {
            if d.is_vid() && !vid_to_real.contains_key(d) {
                if let Some(&xfer_r) = analysis.xfer_precolor.get(d) {
                    vid_to_real.insert(*d, xfer_r);
                } else if let Some(&local_r) = analysis.stloc_targets.get(d) {
                    vid_to_real.insert(*d, local_r);
                } else if let Some(&local_r) = analysis.coalesce_to_local.get(d) {
                    vid_to_real.insert(*d, local_r);
                } else {
                    // General case: reuse a same-typed slot from the pool, or mint a fresh one.
                    let ty = vid_type(*d, vid_types)?;
                    let real = if let Some(slots) = free_pool.get_mut(&ty) {
                        slots.pop()
                    } else {
                        None
                    };
                    let real = real.unwrap_or_else(|| {
                        let r = Slot::Home(next_slot);
                        next_slot += 1;
                        real_slot_types.insert(r, ty);
                        r
                    });
                    vid_to_real.insert(*d, real);
                }
            }
        }

        // Phase 3: Rewrite the instruction with real slots.
        remap_instr(&mut mapped_instr, &vid_to_real);
        output.push(mapped_instr);

        // Phase 4: Free slots for defs that are never used (last_use == def site).
        for d in &defs {
            if d.is_vid() && analysis.last_use.get(d) == Some(&i) {
                if !uses.contains(d)
                    && let Some(&real) = vid_to_real.get(d)
                    && matches!(real, Slot::Home(i) if i >= num_pinned)
                {
                    let ty = real_slot_types.get(&real).cloned().unwrap_or(Type::Bool);
                    free_pool.entry(ty).or_default().push(real);
                }
            }
        }
    }

    Ok((output, next_slot, analysis.max_xfer_width, free_pool))
}
