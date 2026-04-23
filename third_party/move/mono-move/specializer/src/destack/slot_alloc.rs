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
use mono_move_core::types::InternedType;
use shared_dsa::UnorderedMap;

/// Output of slot allocation for a single function.
pub(crate) struct AllocatedFunction {
    pub blocks: Vec<BasicBlock>,
    pub num_home_slots: u16,
    pub num_xfer_slots: u16,
    pub home_slot_types: Vec<InternedType>,
}

/// SSA `Vid` → real slot mapping with per-slot type tracking.
///
/// Backed by two dense vectors indexed by ordinal:
///   - `real_slot_types[i]` — type of `Slot::Home(i)`.
///   - `vid_to_real[i]` — binding of `Slot::Vid(i)` (`None` if unbound).
///
/// Invariant: every Home slot has a recorded type. Enforced by
/// [`Self::mint_fresh`] being the only slot-introduction path. Pinned-local
/// identity (`Home(i) → Home(i)` for `i < num_pinned`) is inferred, not stored.
struct SlotTable {
    /// Pinned local count. Occupies `0..num_pinned` in `real_slot_types`.
    num_pinned: u16,
    /// Type per Home slot, indexed by ordinal. Grows only via `mint_fresh`.
    real_slot_types: Vec<InternedType>,
    /// Binding per Vid, indexed by ordinal. Cleared per block.
    vid_to_real: Vec<Option<Slot>>,
}

impl SlotTable {
    fn new(local_types: &[InternedType]) -> Self {
        Self {
            num_pinned: local_types.len() as u16,
            real_slot_types: local_types.to_vec(),
            vid_to_real: Vec::new(),
        }
    }

    /// Resets per-block state.
    fn start_block(&mut self) {
        self.vid_to_real.clear();
    }

    /// Mints a fresh Home slot with the given type and binds `vid` to it.
    /// The only path that introduces a new real slot.
    fn mint_fresh(&mut self, vid: Slot, ty: InternedType) -> Slot {
        let real = Slot::Home(self.real_slot_types.len() as u16);
        self.real_slot_types.push(ty);
        self.bind(vid, real);
        real
    }

    /// Binds `vid` to an existing real slot. No-op for non-Vid keys
    /// (pinned identity is implicit; Xfer slots pass through).
    fn bind(&mut self, vid: Slot, real: Slot) {
        if let Slot::Vid(i) = vid {
            let i = i as usize;
            if self.vid_to_real.len() <= i {
                self.vid_to_real.resize(i + 1, None);
            }
            self.vid_to_real[i] = Some(real);
        }
    }

    /// Returns `(real_slot, type)` for a vid bound to a Home slot, or
    /// `None` if unbound or bound to an Xfer slot. Returning both pieces
    /// atomically prevents callers from observing a slot without its type.
    fn lookup(&self, vid: Slot) -> Option<(Slot, InternedType)> {
        let real = self.real_of_opt(vid)?;
        let Slot::Home(i) = real else { return None };
        let ty = *self.real_slot_types.get(i as usize)?;
        Some((real, ty))
    }

    /// Whether `vid` is bound (pinned identity counts).
    fn contains(&self, vid: &Slot) -> bool {
        self.real_of_opt(*vid).is_some()
    }

    /// Returns the real slot for `vid`, or `vid` unchanged if unbound.
    fn real_of(&self, vid: Slot) -> Slot {
        self.real_of_opt(vid).unwrap_or(vid)
    }

    /// Whether `real` is poolable — a non-pinned Home slot.
    fn is_poolable(&self, real: Slot) -> bool {
        matches!(real, Slot::Home(i) if i >= self.num_pinned)
    }

    fn next_slot(&self) -> u16 {
        self.real_slot_types.len() as u16
    }

    /// Consumes the table and returns the per-Home-slot type vector
    /// indexed by ordinal.
    fn into_home_slot_types(self) -> Vec<InternedType> {
        self.real_slot_types
    }

    fn real_of_opt(&self, vid: Slot) -> Option<Slot> {
        match vid {
            // Pinned locals are identity-mapped without an explicit entry.
            Slot::Home(i) if i < self.num_pinned => Some(vid),
            Slot::Vid(i) => self.vid_to_real.get(i as usize).copied().flatten(),
            _ => None,
        }
    }
}

/// Map SSA `Vid`s to real slots across all blocks.
///
/// Consumes the SSAFunction and remaps instructions in-place.
///
/// Pre: SSA blocks after fusion passes; vid_types maps each `Vid(i)`
///      to its type at index `i`.
/// Post: all `Vid`s replaced with real `Home`/`Xfer` slots.
pub(crate) fn allocate_slots(ssa: SSAFunction) -> Result<AllocatedFunction> {
    let mut table = SlotTable::new(&ssa.local_types);
    let mut result_blocks = Vec::with_capacity(ssa.blocks.len());
    let mut global_num_xfer_slots: u16 = 0;
    let mut free_pool: UnorderedMap<InternedType, Vec<Slot>> = UnorderedMap::new();

    for mut block in ssa.blocks {
        let analysis = BlockAnalysis::analyze(&block.instrs);
        let (block_xfer_slots, returned_pool) = allocate_block_in_place(
            &mut block.instrs,
            &mut table,
            free_pool,
            &ssa.vid_types,
            &analysis,
        )?;
        free_pool = returned_pool;
        if block_xfer_slots > global_num_xfer_slots {
            global_num_xfer_slots = block_xfer_slots;
        }
        result_blocks.push(block);
    }

    let num_home_slots = table.next_slot();
    let home_slot_types = table.into_home_slot_types();

    Ok(AllocatedFunction {
        blocks: result_blocks,
        num_home_slots,
        num_xfer_slots: global_num_xfer_slots,
        home_slot_types,
    })
}

fn vid_type(vid: Slot, vid_types: &[InternedType]) -> Result<InternedType> {
    match vid {
        Slot::Vid(i) => vid_types
            .get(i as usize)
            .copied()
            .context("VID type not found during SSA allocation"),
        _ => bail!("vid_type called on non-Vid slot {:?}", vid),
    }
}

/// Allocate real slots for a single basic block, remapping instructions in-place.
///
/// For each instruction, in order: free last-use sources, allocate defs, remap, free dead defs.
/// Allocation priority: xfer_precolor > stloc_targets > coalesce_to_local > type-keyed reuse > fresh.
///
/// Returns (xfer width, updated free pool).
fn allocate_block_in_place(
    instrs: &mut [Instr],
    table: &mut SlotTable,
    carry_pool: UnorderedMap<InternedType, Vec<Slot>>,
    vid_types: &[InternedType],
    analysis: &BlockAnalysis,
) -> Result<(u16, UnorderedMap<InternedType, Vec<Slot>>)> {
    if instrs.is_empty() {
        return Ok((0, carry_pool));
    }
    table.start_block();
    let mut free_pool = carry_pool;

    for (i, instr) in instrs.iter_mut().enumerate() {
        let (defs, uses) = collect_defs_and_uses(instr);

        // Phase 1: Free use-slots whose last use is this instruction.
        // Done BEFORE def allocation so the freed slot can be immediately reused.
        // Safe because sources are read before destinations are written.
        for r in &uses {
            if r.is_vid()
                && analysis.last_use.get(r) == Some(&i)
                && !defs.contains(r)
                && let Some((real, ty)) = table.lookup(*r)
                && table.is_poolable(real)
            {
                free_pool.entry(ty).or_default().push(real);
            }
        }

        // Phase 2: Allocate real slots for destination `Vid`s.
        for d in &defs {
            if d.is_vid() && !table.contains(d) {
                if let Some(&xfer_r) = analysis.xfer_precolor.get(d) {
                    table.bind(*d, xfer_r);
                } else if let Some(&local_r) = analysis.stloc_targets.get(d) {
                    table.bind(*d, local_r);
                } else if let Some(&local_r) = analysis.coalesce_to_local.get(d) {
                    table.bind(*d, local_r);
                } else {
                    // General case: reuse a same-typed slot from the pool, or mint a fresh one.
                    let ty = vid_type(*d, vid_types)?;
                    if let Some(real) = free_pool.get_mut(&ty).and_then(|slots| slots.pop()) {
                        table.bind(*d, real);
                    } else {
                        table.mint_fresh(*d, ty);
                    }
                }
            }
        }

        // Phase 3: Rewrite the instruction with real slots — in-place.
        remap_all_slots_with(instr, |s| table.real_of(s));

        // Phase 4: Free slots for defs that are never used (last_use == def site).
        for d in &defs {
            if d.is_vid()
                && analysis.last_use.get(d) == Some(&i)
                && !uses.contains(d)
                && let Some((real, ty)) = table.lookup(*d)
                && table.is_poolable(real)
            {
                free_pool.entry(ty).or_default().push(real);
            }
        }
    }

    Ok((analysis.max_xfer_width, free_pool))
}
