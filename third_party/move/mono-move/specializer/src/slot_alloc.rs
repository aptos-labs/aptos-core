// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Greedy slot allocation.
//!
//! Consumes `BlockAnalysis` from `analysis` and maps SSA temp VIDs to
//! physical Home/Xfer slots using liveness-driven type-keyed reuse.

use crate::{
    analysis::analyze_block,
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
    pub slot_types: Vec<Type>,
}

/// Map SSA VIDs to physical slots across all blocks.
///
/// Pre: SSA instruction stream after fusion passes; vid_types maps each `Vid(i)`
///      to its type at index `i`.
/// Post: all VIDs replaced with physical Home/Xfer slots.
pub(crate) fn allocate_slots(ssa: &SSAFunction) -> Result<AllocatedFunction> {
    let num_pinned = ssa.local_types.len() as u16;
    let blocks = split_into_blocks(&ssa.instrs);
    let mut result = Vec::with_capacity(ssa.instrs.len());
    let mut global_next_slot = num_pinned;
    let mut global_num_xfer_slots: u16 = 0;
    let mut free_pool: BTreeMap<Type, Vec<Slot>> = BTreeMap::new();
    let mut phys_slot_types: BTreeMap<Slot, Type> = BTreeMap::new();
    for (i, ty) in ssa.local_types.iter().enumerate() {
        phys_slot_types.insert(Slot::Home(i as u16), ty.clone());
    }

    for (start, end) in blocks {
        let block_instrs = &ssa.instrs[start..end];
        let analysis = analyze_block(block_instrs);
        let (allocated, block_max, block_xfer_slots, returned_pool) = allocate_block(
            block_instrs,
            num_pinned,
            global_next_slot,
            free_pool,
            &ssa.vid_types,
            &mut phys_slot_types,
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

    let mut slot_types = Vec::with_capacity(global_next_slot as usize);
    for i in 0..global_next_slot {
        slot_types.push(
            phys_slot_types
                .get(&Slot::Home(i))
                .cloned()
                .context("missing type for physical slot")?,
        );
    }

    Ok(AllocatedFunction {
        instrs: result,
        num_home_slots: global_next_slot,
        num_xfer_slots: global_num_xfer_slots,
        slot_types,
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

fn allocate_block(
    instrs: &[Instr],
    num_pinned: u16,
    start_slot: u16,
    carry_pool: BTreeMap<Type, Vec<Slot>>,
    vid_types: &[Type],
    phys_slot_types: &mut BTreeMap<Slot, Type>,
    analysis: &crate::analysis::BlockAnalysis,
) -> Result<(Vec<Instr>, u16, u16, BTreeMap<Type, Vec<Slot>>)> {
    if instrs.is_empty() {
        return Ok((Vec::new(), start_slot, 0, carry_pool));
    }

    let is_vid = |r: &Slot| -> bool { r.is_vid() };

    let mut vid_to_phys: BTreeMap<Slot, Slot> = BTreeMap::new();
    for r in 0..num_pinned {
        vid_to_phys.insert(Slot::Home(r), Slot::Home(r));
    }
    let mut free_pool = carry_pool;
    let mut next_slot = start_slot;

    let mut output = Vec::with_capacity(instrs.len());

    for (i, instr) in instrs.iter().enumerate() {
        let mut mapped_instr = instr.clone();
        let (defs, uses) = get_defs_uses(instr);

        // Free use-slots whose last use is this instruction BEFORE allocating
        // for defs. This is safe because IR instruction semantics guarantee all
        // sources are read before any destination is written, so reusing a source
        // slot as a destination (e.g. `r2 := add r2, a0`) is correct: the old
        // value of r2 is consumed before the result overwrites it. In SSA form,
        // a temp VID is defined exactly once and cannot appear as both a def and
        // use of the same instruction, so there is no risk of freeing a slot
        // that is also being defined here.
        for r in &uses {
            if is_vid(r)
                && analysis.last_use.get(r) == Some(&i)
                && !defs.contains(r)
                && let Some(&phys) = vid_to_phys.get(r)
                && matches!(phys, Slot::Home(i) if i >= num_pinned)
            {
                let ty = phys_slot_types.get(&phys).cloned().unwrap_or(Type::Bool);
                free_pool.entry(ty).or_default().push(phys);
            }
        }

        // Allocate physical slots for destination vids
        for d in &defs {
            if is_vid(d) && !vid_to_phys.contains_key(d) {
                if let Some(&xfer_r) = analysis.xfer_precolor.get(d) {
                    vid_to_phys.insert(*d, xfer_r);
                } else if let Some(&local_r) = analysis.stloc_targets.get(d) {
                    vid_to_phys.insert(*d, local_r);
                } else if let Some(&local_r) = analysis.coalesce_to_local.get(d) {
                    vid_to_phys.insert(*d, local_r);
                } else {
                    let ty = vid_type(*d, vid_types)?;
                    let phys = if let Some(slots) = free_pool.get_mut(&ty) {
                        slots.pop()
                    } else {
                        None
                    };
                    let phys = phys.unwrap_or_else(|| {
                        let r = Slot::Home(next_slot);
                        next_slot += 1;
                        phys_slot_types.insert(r, ty);
                        r
                    });
                    vid_to_phys.insert(*d, phys);
                }
            }
        }

        remap_instr(&mut mapped_instr, &vid_to_phys);
        output.push(mapped_instr);

        // Free slots for defs that are never used (last_use == def site).
        for d in &defs {
            if is_vid(d) && analysis.last_use.get(d) == Some(&i) {
                let (_, ref uses_list) = get_defs_uses(instr);
                if !uses_list.contains(d)
                    && let Some(&phys) = vid_to_phys.get(d)
                    && matches!(phys, Slot::Home(i) if i >= num_pinned)
                {
                    let ty = phys_slot_types.get(&phys).cloned().unwrap_or(Type::Bool);
                    free_pool.entry(ty).or_default().push(phys);
                }
            }
        }
    }

    Ok((output, next_slot, analysis.max_xfer_width, free_pool))
}
