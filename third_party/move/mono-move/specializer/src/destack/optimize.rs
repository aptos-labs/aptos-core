// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Post-slot-allocation optimization passes over the named-slot IR; see
//! [`optimize_module`] for the pass pipeline.

use crate::stackless_exec_ir::{
    instr_utils::{
        for_each_def, for_each_slot, for_each_use, mut_local_borrow_target, remap_all_slots_with,
        remap_source_slots_with,
    },
    FunctionIR, HomeIndex, Instr, ModuleIR, NamedSlot,
};
use mono_move_core::PreparedModule;
use shared_dsa::{UnorderedMap, UnorderedSet};

/// Optimize all functions in a module IR.
pub fn optimize_module(module_ir: &mut ModuleIR) {
    let ModuleIR { module, functions } = module_ir;
    for func in functions.iter_mut().flatten() {
        eliminate_identity_moves(func, module);
        copy_propagation(func, module);
        eliminate_identity_moves(func, module);
        dead_instruction_elimination(func);
        renumber_slots(func);
    }
}

/// Pass: Forward copy propagation within each basic block.
///
/// Pre: allocated instruction stream (named slots).
/// Post: Copy/Move sources propagated to downstream uses; no instructions removed.
///
/// # Correctness
///
/// ## Value uses vs. place uses
/// Propagation is sound for value uses (value equality suffices) but unsound
/// for place uses (slot identity matters) — see "Operand roles" on [`Instr`].
/// `remap_source_slots_with` skips BorrowLoc sources to enforce this.
///
/// ## The MutBorrowLoc hidden-write problem
/// Once a slot is mutably borrowed, it can be silently modified through the
/// reference (via `WriteRef`, function calls, etc.) without appearing as a
/// def. So we kill subst entries for the borrowed slot at `MutBorrowLoc` and
/// `MutBorrowLocField` — conservatively assuming hidden writes may follow. A
/// mut borrow cannot predate an entry: the verifier forbids `MoveLoc` on a
/// borrowed local and `CopyLoc` on a mutably borrowed one. `ImmBorrowLoc`,
/// `ImmBorrowLocField`, and `ReadLocField` need no kill: `StLoc`/`MoveLoc`
/// on a borrowed local are forbidden, so a slot cannot change while
/// immutably borrowed.
///
/// ## Why cross-block mutable borrows are safe
/// Subst is reset at every block boundary. If `MutBorrowLoc` is in the same
/// block as the copy, the kill fires before any hidden write. If it's in a
/// different block, that block's subst is empty — no stale propagation occurs.
///
/// ## Why `Move` sources are safe but `Copy` sources need a type guard
/// See the interchangeability rules on [`Instr::Copy`]/[`Instr::Move`].
/// Applied here: a `Move` entry can never go stale — the verifier makes the
/// source unusable until redefinition, which kills the entry. A `Copy` entry
/// for a heap-owning type has no such protection: the source stays usable, so
/// remapping a consuming use of `dst` would transfer the wrong object, and the
/// source's object may be consumed or mutated while the entry is live. `Copy`
/// entries are therefore only created for bitwise-copy types.
fn copy_propagation(func: &mut FunctionIR, module: &PreparedModule) {
    let FunctionIR {
        blocks,
        home_slot_types,
        ..
    } = func;
    for block in blocks {
        // TODO(perf): value-based kills `retain`-scan all entries; a reverse
        // index (value → keys) would make them O(1).
        let mut subst: UnorderedMap<NamedSlot, NamedSlot> = UnorderedMap::new();

        for instr in block.instrs.iter_mut() {
            // Kill entries invalidated by this def before remapping sources
            // (see the interchangeability rules on `Instr::Copy`/`Instr::Move`):
            // remapping through a stale entry would rewrite a source onto this
            // instruction's own def, manufacturing `dst == src` `Copy` shapes
            // whose elimination is unsound for heap-owning types (see
            // `eliminate_identity_moves`). `WriteLocField`/`WriteLocFieldChain`
            // report the partially written local as a def, so they are covered.
            for_each_def(instr, |d| {
                subst.remove(&d);
                subst.retain(|_, v| *v != d);
            });

            // Mut borrows allow hidden writes that never appear as defs (see
            // the hidden-write section above). The ref-rooted `WriteFieldChain`
            // writes through a reference whose originating mut borrow was
            // already killed here.
            if let Some(src) = mut_local_borrow_target(instr) {
                subst.remove(&src);
                subst.retain(|_, v| *v != src);
            }

            remap_source_slots_with(instr, |s| *subst.get(&s).unwrap_or(&s));

            // Record new entries. Transfer slot values don't survive a call
            // boundary, so never propagate from them; `Copy` additionally
            // needs the bitwise-copy type guard (see "Why `Move` sources are
            // safe but `Copy` sources need a type guard" above).
            match instr {
                Instr::Move { dst, src } => {
                    if !matches!(src, NamedSlot::Transfer(_)) {
                        subst.insert(*dst, *src);
                    }
                },
                Instr::Copy { dst, src } => {
                    // TODO(perf): a non-capturing closure has a null
                    // captured-data pointer, so its deep copy is pure overhead
                    // — but the type-based guard can't tell it from a capturing
                    // one. Recover the propagation by tracking slots defined in
                    // this block by a `PackClosure` with an empty `captured`
                    // list.
                    let interchangeable = match src {
                        NamedSlot::Home(idx) => {
                            module.is_bitwise_copy_type(home_slot_types[idx.0 as usize])
                        },
                        NamedSlot::Transfer(_) => false,
                    };
                    if interchangeable {
                        subst.insert(*dst, *src);
                    }
                },
                _ => {},
            }
        }
    }
}

/// Pass: Remove `Move` and `Copy` instructions whose `dst == src`, except
/// `Copy`s of heap-owning types.
///
/// An identity `Move` is a no-op for every type, and so is an identity
/// `Copy` of a bitwise-copy type (no deep copy is emitted). An identity
/// `Copy` of a heap-owning type is a real effect — an in-place deep copy,
/// "replace the slot's object with a fresh copy" — whose deletion is sound
/// only when no other slot co-owns the object, which is not locally
/// checkable. Thus, it is kept.
fn eliminate_identity_moves(func: &mut FunctionIR, module: &PreparedModule) {
    let FunctionIR {
        blocks,
        home_slot_types,
        ..
    } = func;
    for block in blocks {
        block.instrs.retain(|instr| {
            let (is_copy, dst) = match instr {
                Instr::Move { dst, src } if dst == src => (false, dst),
                Instr::Copy { dst, src } if dst == src => (true, dst),
                _ => return true,
            };
            // A Transfer identity can't currently arise: SSA `Copy`/`Move`
            // always carry a Home slot on one side, and copy propagation
            // never remaps a source onto a Transfer slot.
            let NamedSlot::Home(idx) = dst else {
                // A Transfer identity would resolve to distinct frame offsets
                // at lowering, so deleting one would be wrong: keep it.
                debug_assert!(false, "identity Copy/Move on a Transfer slot");
                return true;
            };
            let removable =
                !is_copy || module.is_bitwise_copy_type(home_slot_types[idx.0 as usize]);
            !removable
        });
    }
}

/// Pass: Backward dead-code elimination within each basic block.
///
/// Pre: after copy propagation and identity move elimination.
/// Post: dead Copy/Move to unused slots removed.
///
/// Slots that appear in more than one basic block are excluded from
/// removal — their liveness cannot be determined by block-local analysis.
///
/// Deleting a *dead* Copy is sound for every type, including a heap-typed
/// identity Copy kept by `eliminate_identity_moves`: a Copy's only effect
/// is its dst slot, and dead means no instruction reads it.
fn dead_instruction_elimination(func: &mut FunctionIR) {
    // Pre-scan: identify Home slots that appear in more than one block.
    // (Transfer slots are intra-block and never cross block boundaries.)
    let mut slot_block: UnorderedMap<NamedSlot, usize> = UnorderedMap::new();
    let mut cross_block_slots: UnorderedSet<NamedSlot> = UnorderedSet::new();
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
        let mut live: UnorderedSet<NamedSlot> = UnorderedSet::new();
        let mut dead_indices: UnorderedSet<usize> = UnorderedSet::new();

        for (i, instr) in block.instrs.iter().enumerate().rev() {
            let is_removable = match instr {
                Instr::Copy { dst, .. } | Instr::Move { dst, .. }
                    if !cross_block_slots.contains(dst) =>
                {
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
            block
                .instrs
                .retain_indexed(|index, _| !dead_indices.contains(&index));
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
            if let NamedSlot::Home(i) = r {
                used[i.0 as usize] = true;
            }
        });
    }

    // Build remap[old_index] = Some(new_index) or None if unused.
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
            NamedSlot::Home(i) => remap_ref[i.0 as usize]
                .map(|new_idx| NamedSlot::Home(HomeIndex(new_idx)))
                .unwrap_or(slot),
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
