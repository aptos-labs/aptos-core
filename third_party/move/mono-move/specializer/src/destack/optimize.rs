// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Post-slot allocation optimization passes.
//!
//! Pass: Copy propagation
//! Pass: Identity move elimination
//! Pass: Dead instruction elimination
//! Pass: Slot renumbering

use super::instr_utils::{apply_subst_to_sources, get_defs_uses, remap_instr, split_into_blocks};
use crate::stackless_exec_ir::{FunctionIR, Instr, ModuleIR, Slot};
use move_vm_types::loaded_data::runtime_types::Type;
use std::collections::{BTreeMap, BTreeSet};

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
/// `apply_subst_to_sources` skips BorrowLoc sources to enforce this.
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
    let blocks = split_into_blocks(&func.instrs);

    for block in blocks {
        let mut subst: BTreeMap<Slot, Slot> = BTreeMap::new();

        for i in block.clone() {
            apply_subst_to_sources(&mut func.instrs[i], &subst);

            // MutBorrowLoc kill: the borrowed slot may be silently written
            // through the resulting reference (WriteRef), which is not tracked
            // as a def. Kill any substitution involving the borrowed slot.
            if let Instr::MutBorrowLoc(_, src) = &func.instrs[i] {
                subst.remove(src);
                subst.retain(|_, v| v != src);
            }

            let (defs, _) = get_defs_uses(&func.instrs[i]);
            for d in &defs {
                subst.remove(d);
                subst.retain(|_, v| v != d);
            }

            match &func.instrs[i] {
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
    func.instrs
        .retain(|instr| !matches!(instr, Instr::Move(d, s) | Instr::Copy(d, s) if d == s));
}

/// Pass: Backward dead-code elimination within each basic block.
///
/// Pre: after copy propagation and identity move elimination.
/// Post: dead Copy/Move to unused slots removed.
///
/// Slots that appear in more than one basic block are excluded from
/// removal — their liveness cannot be determined by block-local analysis.
fn dead_instruction_elimination(func: &mut FunctionIR) {
    let blocks = split_into_blocks(&func.instrs);

    // Pre-scan: identify slots that appear in more than one block.
    let mut slot_block: BTreeMap<Slot, usize> = BTreeMap::new();
    let mut cross_block_slots: BTreeSet<Slot> = BTreeSet::new();
    for (block_id, block) in blocks.iter().enumerate() {
        for i in block.clone() {
            let (dsts, srcs) = get_defs_uses(&func.instrs[i]);
            for r in dsts.into_iter().chain(srcs) {
                match slot_block.get(&r) {
                    Some(&prev) if prev != block_id => {
                        cross_block_slots.insert(r);
                    },
                    None => {
                        slot_block.insert(r, block_id);
                    },
                    _ => {},
                }
            }
        }
    }

    let mut dead_indices: BTreeSet<usize> = BTreeSet::new();

    for block in blocks {
        let mut live: BTreeSet<Slot> = BTreeSet::new();

        for i in block.rev() {
            let (dsts, srcs) = get_defs_uses(&func.instrs[i]);

            let is_removable = match &func.instrs[i] {
                Instr::Copy(dst, _) | Instr::Move(dst, _) if !cross_block_slots.contains(dst) => {
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

/// Pass: Compact Home slot indices while preserving param indices.
///
/// Pre: after DCE (some slots may be unused).
/// Post: Params keep indices 0..num_params-1 (calling-convention-visible).
///       Surviving locals and temps are compacted contiguously starting at num_params.
///       `num_locals`, `num_home_slots`, and `home_slot_types` are updated.
fn renumber_slots(func: &mut FunctionIR) {
    let num_params = func.num_params;

    let mut used_home_slots: BTreeSet<u16> = BTreeSet::new();
    for instr in &func.instrs {
        let (defs, uses_) = get_defs_uses(instr);
        for r in defs.into_iter().chain(uses_) {
            if let Slot::Home(i) = r {
                used_home_slots.insert(i);
            }
        }
    }

    let mut remap: BTreeMap<Slot, Slot> = BTreeMap::new();
    let mut next_slot = num_params;
    let mut num_surviving_locals: u16 = 0;
    let old_num_pinned = num_params + func.num_locals;
    for &i in &used_home_slots {
        if i < num_params {
            // Params keep their ABI indices.
            remap.insert(Slot::Home(i), Slot::Home(i));
        } else {
            remap.insert(Slot::Home(i), Slot::Home(next_slot));
            if i < old_num_pinned {
                num_surviving_locals += 1;
            }
            next_slot += 1;
        }
    }

    for instr in &mut func.instrs {
        remap_instr(instr, &remap);
    }

    // Build new home_slot_types. Bool placeholder — every entry is overwritten below:
    // params are always copied (calling convention requires correct types even if
    // unused in the body), and non-param slots are copied from the remap.
    let mut new_slot_types = vec![Type::Bool; next_slot as usize];
    for i in 0..num_params {
        if (i as usize) < func.home_slot_types.len() {
            new_slot_types[i as usize] = func.home_slot_types[i as usize].clone();
        }
    }
    // Non-param slots: copy types according to remap. Params are skipped
    // here — they are already handled by the loop above.
    for (&old, &new) in &remap {
        if let (Slot::Home(old_i), Slot::Home(new_i)) = (old, new)
            && old_i >= num_params
            && (old_i as usize) < func.home_slot_types.len()
        {
            new_slot_types[new_i as usize] = func.home_slot_types[old_i as usize].clone();
        }
    }
    func.home_slot_types = new_slot_types;

    func.num_locals = num_surviving_locals;
    func.num_home_slots = next_slot;
}
