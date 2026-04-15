// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Greedy slot allocation.
//!
//! Consumes `BlockAnalysis` from `analysis` and maps SSA temp `Vid`s to
//! real `Home`/`Xfer` slots using liveness-driven type-keyed reuse.

use super::{
    analysis::BlockAnalysis,
    instr_utils::{collect_defs_and_uses, remap_all_slots_with},
    ssa_function::SSAFunction,
};
use crate::stackless_exec_ir::{BasicBlock, Instr, Slot};
use anyhow::{bail, Context, Result};
use move_vm_types::loaded_data::runtime_types::Type;
use shared_dsa::UnorderedMap;

/// Output of slot allocation for a single function.
pub(crate) struct AllocatedFunction {
    pub blocks: Vec<BasicBlock>,
    pub num_home_slots: u16,
    pub num_xfer_slots: u16,
    pub home_slot_types: Vec<Type>,
}

/// Map SSA `Vid`s to real slots across all blocks.
///
/// Consumes the SSAFunction and remaps instructions in-place.
///
/// Pre: SSA blocks after fusion passes; vid_types maps each `Vid(i)`
///      to its type at index `i`.
/// Post: all `Vid`s replaced with real `Home`/`Xfer` slots.
pub(crate) fn allocate_slots(ssa: SSAFunction) -> Result<AllocatedFunction> {
    let num_pinned = ssa.local_types.len() as u16;
    let mut result_blocks = Vec::with_capacity(ssa.blocks.len());
    let mut global_next_slot = num_pinned;
    let mut global_num_xfer_slots: u16 = 0;
    let mut free_pool: UnorderedMap<Type, Vec<Slot>> = UnorderedMap::new();
    let mut real_slot_types: UnorderedMap<Slot, Type> = UnorderedMap::new();
    for (i, ty) in ssa.local_types.iter().enumerate() {
        real_slot_types.insert(Slot::Home(i as u16), ty.clone());
    }

    for mut block in ssa.blocks {
        let analysis = BlockAnalysis::analyze(&block.instrs);
        let (block_max, block_xfer_slots, returned_pool) = allocate_block_in_place(
            &mut block.instrs,
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
        result_blocks.push(block);
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
        blocks: result_blocks,
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

/// Allocate real slots for a single basic block, remapping instructions in-place.
///
/// For each instruction, in order: free last-use sources, allocate defs, remap, free dead defs.
/// Allocation priority: xfer_precolor > stloc_targets > coalesce_to_local > type-keyed reuse > fresh.
///
/// Returns (next available slot index, xfer width, updated free pool).
fn allocate_block_in_place(
    instrs: &mut [Instr],
    num_pinned: u16,
    start_slot: u16,
    carry_pool: UnorderedMap<Type, Vec<Slot>>,
    vid_types: &[Type],
    real_slot_types: &mut UnorderedMap<Slot, Type>,
    analysis: &BlockAnalysis,
) -> Result<(u16, u16, UnorderedMap<Type, Vec<Slot>>)> {
    if instrs.is_empty() {
        return Ok((start_slot, 0, carry_pool));
    }

    // Identity mapping for pinned locals so remap_all_slots leaves them unchanged.
    let mut vid_to_real: UnorderedMap<Slot, Slot> = UnorderedMap::new();
    for r in 0..num_pinned {
        vid_to_real.insert(Slot::Home(r), Slot::Home(r));
    }
    let mut free_pool = carry_pool;
    let mut next_slot = start_slot;

    for (i, instr) in instrs.iter_mut().enumerate() {
        let (defs, uses) = collect_defs_and_uses(instr);

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

        // Phase 3: Rewrite the instruction with real slots — in-place.
        remap_all_slots_with(instr, |s| *vid_to_real.get(&s).unwrap_or(&s));

        // Phase 4: Free slots for defs that are never used (last_use == def site).
        for d in &defs {
            if d.is_vid()
                && analysis.last_use.get(d) == Some(&i)
                && !uses.contains(d)
                && let Some(&real) = vid_to_real.get(d)
                && matches!(real, Slot::Home(i) if i >= num_pinned)
            {
                let ty = real_slot_types.get(&real).cloned().unwrap_or(Type::Bool);
                free_pool.entry(ty).or_default().push(real);
            }
        }
    }

    Ok((next_slot, analysis.max_xfer_width, free_pool))
}
