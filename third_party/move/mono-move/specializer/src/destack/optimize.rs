// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Post-slot allocation optimization passes.
//!
//! Pass: Copy propagation
//! Pass: Identity move elimination
//! Pass: Dead instruction elimination
//! Pass: Slot renumbering

use super::instr_utils::{
    for_each_def, for_each_slot, for_each_use, remap_all_slots_with, remap_source_slots_with,
};
use crate::stackless_exec_ir::{FunctionIR, Instr, ModuleIR, Slot};
use shared_dsa::{UnorderedMap, UnorderedSet};

/// Optimize all functions in a module IR.
/// Pre: slot allocation complete — no `Vid`s remain.
pub fn optimize_module(module_ir: &mut ModuleIR) {
    for func in &mut module_ir.functions {
        eliminate_identity_moves(func);
        copy_propagation(func);
        eliminate_identity_moves(func);
        dead_instruction_elimination(func);
        renumber_slots(func);
    }
}

/// Pass: Forward copy propagation within each basic block.
///
/// Pre: allocated instruction stream (real slots).
/// Post: Copy/Move sources propagated to downstream uses; no instructions removed.
///
/// # Correctness
///
/// ## Move verifier guarantees we rely on
/// - **StLoc(L) forbidden while L is borrowed** (immutably or mutably)
/// - **MoveLoc(L) forbidden while L is borrowed**
/// - **CopyLoc(L) forbidden while L is mutably borrowed**
///
/// ## Two kinds of slot use
/// 1. **Value use** — reads the slot's current value (e.g., `Add(dst, r, #5)`)
/// 2. **Storage-location use** — takes a reference to the slot's storage location
///    (only `ImmBorrowLoc` and `MutBorrowLoc`)
///
/// Copy propagation is sound for value uses (value equality suffices) but
/// unsound for storage-location uses (identity of the slot matters).
/// `remap_source_slots_with` skips BorrowLoc sources to enforce this.
///
/// ## The MutBorrowLoc hidden-write problem
/// Once a slot is mutably borrowed, it can be silently modified through the
/// reference (via `WriteRef`, function calls, etc.) without appearing as a
/// def in `get_defs_uses`. So we kill subst entries for the borrowed slot
/// at `MutBorrowLoc` — conservatively assuming hidden writes may follow.
///
/// `ImmBorrowLoc` does NOT need this kill — the verifier guarantees the
/// borrowed slot cannot be modified while immutably borrowed.
///
/// ## Why cross-block mutable borrows are safe
///
/// Subst is reset at every block boundary. If `MutBorrowLoc` is in the same
/// block as the copy, the kill fires before any hidden write. If it's in a
/// different block, that block's subst is empty — no stale propagation occurs.
fn copy_propagation(func: &mut FunctionIR) {
    for block in &mut func.blocks {
        // [TODO]: `retain` scans all entries to kill by value, making each kill O(|subst|).
        // For typical small blocks this is fine, but if subst grows large, consider a
        // reverse index (value → keys) for O(1) value-based kills.
        let mut subst: UnorderedMap<Slot, Slot> = UnorderedMap::new();

        for instr in &mut block.instrs {
            remap_source_slots_with(instr, |s| *subst.get(&s).unwrap_or(&s));

            // MutBorrowLoc kill: the borrowed slot may be silently written
            // through the resulting reference (WriteRef), which is not tracked
            // as a def. Kill any substitution involving the borrowed slot.
            if let Instr::MutBorrowLoc(_, src) = instr {
                subst.remove(src);
                subst.retain(|_, v| v != src);
            }

            for_each_def(instr, |d| {
                subst.remove(&d);
                subst.retain(|_, v| *v != d);
            });

            match instr {
                Instr::Copy(dst, src) | Instr::Move(dst, src) => {
                    subst.insert(*dst, *src);
                },
                _ => {},
            }
        }
    }
}

/// Pass: Remove `Move(r, r)` and `Copy(r, r)` instructions.
fn eliminate_identity_moves(func: &mut FunctionIR) {
    for block in &mut func.blocks {
        block
            .instrs
            .retain(|instr| !matches!(instr, Instr::Move(d, s) | Instr::Copy(d, s) if d == s));
    }
}

/// Pass: Backward dead-code elimination within each basic block.
///
/// Pre: after copy propagation and identity move elimination.
/// Post: dead Copy/Move to unused slots removed.
///
/// Slots that appear in more than one basic block are excluded from
/// removal — their liveness cannot be determined by block-local analysis.
fn dead_instruction_elimination(func: &mut FunctionIR) {
    // Pre-scan: identify Home slots that appear in more than one block.
    // (Vid and Xfer slots are intra-block and never cross block boundaries.)
    let mut slot_block: UnorderedMap<Slot, usize> = UnorderedMap::new();
    let mut cross_block_slots: UnorderedSet<Slot> = UnorderedSet::new();
    for (block_id, block) in func.blocks.iter().enumerate() {
        for instr in &block.instrs {
            for_each_slot(instr, |r| match slot_block.get(&r) {
                Some(&prev) if prev != block_id => {
                    cross_block_slots.insert(r);
                },
                None => {
                    slot_block.insert(r, block_id);
                },
                _ => {},
            });
        }
    }

    for block in &mut func.blocks {
        let mut live: UnorderedSet<Slot> = UnorderedSet::new();
        let mut dead_indices: UnorderedSet<usize> = UnorderedSet::new();

        for (i, instr) in block.instrs.iter().enumerate().rev() {
            let is_removable = match instr {
                Instr::Copy(dst, _) | Instr::Move(dst, _) if !cross_block_slots.contains(dst) => {
                    !live.contains(dst)
                },
                _ => false,
            };

            if is_removable {
                dead_indices.insert(i);
            } else {
                for_each_def(instr, |d| {
                    live.remove(&d);
                });
                for_each_use(instr, |s| {
                    live.insert(s);
                });
            }
        }

        if !dead_indices.is_empty() {
            // `retain` visits elements in order, so `idx` tracks the original
            // pre-retain index of each instruction.
            let mut idx = 0;
            block.instrs.retain(|_| {
                let keep = !dead_indices.contains(&idx);
                idx += 1;
                keep
            });
        }
    }
}

/// Pass: Compact Home slot indices while preserving param indices.
///
/// Pre: after DCE (some slots may be unused).
/// Post: Params keep indices 0..num_params-1 (calling-convention-visible).
///       Surviving locals and temps are compacted contiguously starting at num_params.
///       `num_locals`, `num_home_slots`, and `home_slot_types` are updated.
fn renumber_slots(func: &mut FunctionIR) {
    let num_params = func.num_params;
    let old_num_pinned = num_params + func.num_locals;
    let old_num_home = func.num_home_slots;

    // Pass 1: mark which Home slots are used.
    let mut used = vec![false; old_num_home as usize];
    for instr in func.instrs() {
        for_each_slot(instr, |r| {
            if let Slot::Home(i) = r {
                used[i as usize] = true;
            }
        });
    }

    // Build remap[old_index] = Some(new_index) or None if unused.
    // Params keep their ABI indices; non-params are compacted sequentially.
    let mut remap: Vec<Option<u16>> = vec![None; old_num_home as usize];
    let mut next_slot = num_params;
    let mut num_surviving_locals: u16 = 0;
    for i in 0..num_params {
        remap[i as usize] = Some(i);
    }
    for i in num_params..old_num_home {
        if used[i as usize] {
            remap[i as usize] = Some(next_slot);
            if i < old_num_pinned {
                num_surviving_locals += 1;
            }
            next_slot += 1;
        }
    }

    // Pass 2: apply remap to every instruction. O(1) per slot via direct indexing.
    for instr in func.instrs_mut() {
        let remap_ref = &remap;
        remap_all_slots_with(instr, |slot| match slot {
            Slot::Home(i) => remap_ref[i as usize].map(Slot::Home).unwrap_or(slot),
            other => other,
        });
    }

    // Compact home_slot_types in-place. Since new_i <= old_i (compaction only
    // moves slots down), a forward sweep never overwrites unread entries.
    for (old_i, mapped) in remap.iter().enumerate().skip(num_params as usize) {
        if let &Some(new_i) = mapped {
            let new_i = new_i as usize;
            if new_i != old_i {
                func.home_slot_types.swap(new_i, old_i);
            }
        }
    }
    func.home_slot_types.truncate(next_slot as usize);

    func.num_locals = num_surviving_locals;
    func.num_home_slots = next_slot;
}
