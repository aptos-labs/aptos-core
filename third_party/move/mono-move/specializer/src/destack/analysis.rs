// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-basic-block analysis feeding the slot allocator. From a single
//! intra-block SSA slice it produces a [`BlockAnalysis`] — liveness
//! plus three "color this ValueId into that slot" hint maps the allocator
//! uses to elide copies. See the [`BlockAnalysis`] field docs for the
//! precise semantics of each entry.
//!
//! # Transfer slot invariants
//!
//! `analyze` establishes several invariants on the resulting `transfer_precolor`
//! map. The slot allocator follows the precolor map verbatim: every ValueId in it
//! lands at the named Transfer slot, and no other ValueId lands at any Transfer slot. So
//! these are equivalently invariants on each call's args/rets in the
//! transfer-slot-allocated IR.
//!
//! 1. Block-local lifetime. Every Transfer-bound ValueId has its def and last use
//!    within the same basic block. Block or function exit ends the slot's
//!    lifetime.
//! 2. Arg positionality. For any call, if `args[j]` is precolored to `Transfer(i)`,
//!    then `i == j`.
//! 3. Return monotonicity. For any call, if `rets[k1] == Transfer(i1)` and
//!    `rets[k2] == Transfer(i2)` with `k1 < k2`, then `i1 < i2`.
//! 4. Pass-through contiguity. For any pair of consecutive calls in the same
//!    block A then B, the positions in `B.args` whose ValueIds are defined at A and
//!    Transfer-bound form one contiguous interval.
//! 5. Return Transfer prefix. Within any call's `rets` list, all Transfer-bound entries
//!    precede all Home-bound entries.
//! 6. Each Transfer-bound ValueId is read exactly once at or before the call-like
//!    instruction immediately following its def.
//! 7. Not live across calls. No Transfer-bound ValueId's lifetime spans across (defined
//!    before and used after) call-like instruction.
//!
//! Notes:
//!
//! a. Return transfers do not have positionality. A `ret(k)` may be precolored to
//!   `Transfer(j)` with `j ≠ k` when the same ValueId is also `B.args[j]` of the
//!   immediately-following call (the pass-through case).
//! b. No duplicate Transfer indices within a call's args or rets. Arg
//!    positionality forces `args[j] = Transfer(j)` for distinct `j`; return
//!    monotonicity forces rets to take strictly-increasing Transfer indices.
//! c. No cycle detection needed at lowering. Arg positionality and return
//!    monotonicity together make reverse-order emit provably safe by
//!    structural reasoning, with no appeal to bytecode stack semantics or
//!    runtime cycle detection.
//!
//! [`assert_transfer_invariants`] checks all seven invariants at the end of
//! `analyze` in debug builds.
//!
//! # Why `coalesce_to_local` aliasing is safe
//!
//! In SSA, a ValueId is defined once and read-only thereafter — coalescing
//! never introduces ValueId-side writes to the source local's slot. The
//! ValueId's intended value at any read site `t` equals the local's value
//! at the def site, so it equals the local's value at `t` *iff the
//! local's slot has not been mutated in between*. Two channels can
//! mutate it:
//!
//! 1. **Direct writes** — instructions whose `def` set contains the
//!    local. Tracked by `home_def_pos`; checked via `home_redefined`.
//! 2. **Indirect writes through `&mut x`** — `MutBorrowLoc x` produces
//!    a mutable ref, and a subsequent `WriteRef` (or a callee writing
//!    through the ref) mutates `x`'s slot. Move's borrow checker
//!    forbids accessing `x` *under the name `x`* while `&mut x` is
//!    live, but the coalesced ValueId is, at the IR level, an independent
//!    stack value — the borrow checker does not see the aliasing.
//!    Tracked by `home_mut_borrow_pos`; checked via `home_mut_borrowed`.
//!    `MutBorrowLocField` is tracked under the same channel.
//!
//! With both channels clear over `[def_pos(value_id)+1, live_end(value_id))`, the
//! ValueId's snapshot equals the local's slot at every read site and they
//! can share storage with no copy. `MutBorrowField` /
//! `MutBorrowVariantField` / `MutBorrowGlobal` need no tracking here:
//! their `src` is a ref ValueId (or address), never a local, so any
//! local-storage mutation they cascade into was already gated on an
//! upstream `MutBorrowLoc` (or `MutBorrowLocField`). (Field-level
//! coalescing, if ever added, would need to revisit this.)

use crate::stackless_exec_ir::{
    instr_utils::{clobbers_transfer, for_each_def, for_each_use, mut_local_borrow_target},
    HomeIndex, Instr, SsaSlot, TransferPosition,
};
#[cfg(debug_assertions)]
use crate::validate::transfer::check_call_structural_invariants;
use shared_dsa::{UnorderedMap, UnorderedSet};
use smallbitvec::SmallBitVec;
#[cfg(debug_assertions)]
use std::collections::BTreeMap;

/// Analysis results for a single basic block. All map keys are `ValueId`s.
pub(crate) struct BlockAnalysis {
    /// End of the `ValueId`'s live range — the last instruction index where it
    /// is referenced (def or use). For a ValueId that is defined but never
    /// used, equals its `def_pos`, marking the live range as collapsed to
    /// the def site. Used downstream as the "kill point" beyond which the
    /// ValueId's slot can be reused.
    pub live_end: UnorderedMap<SsaSlot, usize>,
    /// `ValueId` → Home slot index it will later be moved into
    /// (`Move { dst: Home, src: value_id }`, the `st_loc` shape produced by destack),
    /// when the Home slot is not accessed between the ValueId's def and the
    /// store. Sound because destack emits this shape only for `StLoc`,
    /// where the ValueId is popped — the move is its last use, so coloring
    /// the ValueId into the Home slot makes the store a self-move that elides.
    pub stloc_targets: UnorderedMap<SsaSlot, HomeIndex>,
    /// `ValueId` → Home slot index of the local it was copied/moved out of,
    /// when the local is neither redefined nor mut-borrowed during the
    /// `ValueId`'s live range. A `Copy` coalesces only when its type is
    /// bitwise-copy: coalescing elides the copy, which is unsound for a value
    /// owning heap storage (see the equivalence rules on
    /// [`Instr::Copy`]/[`Instr::Move`]).
    pub coalesce_to_local: UnorderedMap<SsaSlot, HomeIndex>,
    /// `ValueId` → `Transfer` position, for call args or rets whose live
    /// range doesn't cross any other call. See the file-header section
    /// "Transfer slot invariants" for the different properties this map
    /// satisfies.
    pub transfer_precolor: UnorderedMap<SsaSlot, TransferPosition>,
    /// Largest `max(args.len(), rets.len())` across all calls in the
    /// block — i.e., the number of distinct `Transfer(j)` positions any
    /// call uses.
    pub max_transfer_positions: u16,
}

impl BlockAnalysis {
    /// Analyze a basic block and produce hint maps for slot allocation.
    ///
    /// `is_bitwise_copy_value(id)` reports whether `ValueId(id)`'s type is
    /// bitwise-copy (fully captured by its frame bytes); gates `Copy`
    /// coalescing.
    ///
    /// Pre: `instrs` is one basic block's SSA slice; each `ValueId` is
    /// defined exactly once.
    ///
    /// Post: each entry's soundness condition is enforced before insert,
    /// and the three hint maps (`stloc_targets`, `coalesce_to_local`,
    /// `transfer_precolor`) are pairwise disjoint — `coalesce_to_local`
    /// skips ValueIds already in `stloc_targets`, and `transfer_precolor` skips
    /// ValueIds already in either earlier map. (`coalesce_to_local` ∩ call
    /// rets is empty by SSA: a ValueId defined by a `Call` is never the
    /// `dst` of `Copy`/`Move { dst: value_id, src: Home }`.)
    pub(crate) fn analyze(
        instrs: &[Instr<SsaSlot>],
        is_bitwise_copy_value: impl Fn(u16) -> bool,
    ) -> Self {
        // Forward scan: build per-`ValueId` and per-`Home` position indices.
        // `ValueId` -> last instruction index where it appears as def or use.
        let mut live_end: UnorderedMap<SsaSlot, usize> = UnorderedMap::new();
        // `ValueId` -> instruction index where it is defined.
        let mut def_pos: UnorderedMap<SsaSlot, usize> = UnorderedMap::new();
        // `Home` -> sorted positions where the Home slot appears as def or use.
        // Used only for range-existence queries, so any duplicate `i` (from a
        // single instr touching the same Home as both def and use — destack
        // doesn't currently emit such shapes) would be harmless.
        let mut home_touch_pos: UnorderedMap<SsaSlot, Vec<usize>> = UnorderedMap::new();
        // `Home` -> sorted positions where the Home slot appears as def only.
        let mut home_def_pos: UnorderedMap<SsaSlot, Vec<usize>> = UnorderedMap::new();
        // `Home` -> sorted positions of `MutBorrowLoc` whose source is the
        // Home slot (see file header for why `coalesce_to_local` treats these
        // as conflicts).
        let mut home_mut_borrow_pos: UnorderedMap<SsaSlot, Vec<usize>> = UnorderedMap::new();

        // TODO(perf): we can reduce the number of passes over instructions.
        for (i, instr) in instrs.iter().enumerate() {
            for_each_use(instr, |slot| match slot {
                SsaSlot::ValueId(_) => {
                    live_end.insert(slot, i);
                },
                SsaSlot::Home(_) => {
                    home_touch_pos.entry(slot).or_default().push(i);
                },
            });
            for_each_def(instr, |slot| match slot {
                SsaSlot::ValueId(_) => {
                    live_end.entry(slot).or_insert(i);
                    def_pos.entry(slot).or_insert(i);
                },
                SsaSlot::Home(_) => {
                    home_touch_pos.entry(slot).or_default().push(i);
                    home_def_pos.entry(slot).or_default().push(i);
                },
            });
            // A `&mut` into a home local's storage can defer a write through
            // the returned reference, conflicting with coalescing.
            if let Some(local @ SsaSlot::Home(_)) = mut_local_borrow_target(instr) {
                home_mut_borrow_pos.entry(local).or_default().push(i);
            }
        }

        // Call positions — any instruction that clobbers transfer slots.
        // Already sorted since we iterate in order.
        let call_positions: Vec<usize> = instrs
            .iter()
            .enumerate()
            .filter(|(_, ins)| clobbers_transfer(ins))
            .map(|(i, _)| i)
            .collect();

        // StLoc look-ahead: map `ValueId` → Home slot when the `ValueId` is produced
        // and later stored to that Home slot, with the Home slot not accessed
        // in between. Check uses binary search on home_touch_pos: O(log n)
        // per candidate.
        let mut stloc_targets: UnorderedMap<SsaSlot, HomeIndex> = UnorderedMap::new();
        for (i, instr) in instrs.iter().enumerate() {
            if let Instr::Move {
                dst: dst @ SsaSlot::Home(home_idx),
                src,
            } = instr
                && src.is_value_id()
                && !stloc_targets.contains_key(src)
            {
                let dp = def_pos.get(src).copied().unwrap_or(0);
                let touches = home_touch_pos.get(dst).map(|v| v.as_slice()).unwrap_or(&[]);
                let home_touched = has_any_in_range(touches, dp + 1, i);
                if !home_touched {
                    stloc_targets.insert(*src, *home_idx);
                }
            }
        }

        // `CopyLoc` / `MoveLoc` coalescing. Range checks on
        // `home_def_pos` and `home_mut_borrow_pos` are O(log n) each
        // via binary search.
        let mut coalesce_to_local: UnorderedMap<SsaSlot, HomeIndex> = UnorderedMap::new();
        for (i, instr) in instrs.iter().enumerate() {
            if let Instr::Copy { dst, src } | Instr::Move { dst, src } = instr
                && let SsaSlot::ValueId(id) = dst
                && let SsaSlot::Home(home_idx) = src
            {
                // A `Copy` of a value owning heap storage must materialize an
                // independent object; coalescing would elide the deep copy.
                if matches!(instr, Instr::Copy { .. }) && !is_bitwise_copy_value(*id) {
                    continue;
                }
                let value_id = *dst;
                if let Some(&lu) = live_end.get(&value_id)
                    && lu > i
                    && !stloc_targets.contains_key(&value_id)
                {
                    let local = *src;
                    let defs = home_def_pos
                        .get(&local)
                        .map(|v| v.as_slice())
                        .unwrap_or(&[]);
                    let home_redefined = has_any_in_range(defs, i + 1, lu);
                    // `MutBorrowLoc` exposes the local's slot to
                    // indirect writes — see file header.
                    let borrows = home_mut_borrow_pos
                        .get(&local)
                        .map(|v| v.as_slice())
                        .unwrap_or(&[]);
                    let home_mut_borrowed = has_any_in_range(borrows, i + 1, lu);
                    if !home_redefined && !home_mut_borrowed {
                        coalesce_to_local.insert(value_id, *home_idx);
                    }
                }
            }
        }

        // Transfer slot precoloring.
        //
        // Establishes the seven Transfer invariants documented in the file
        // header. See `assert_transfer_invariants` for the debug-build
        // checks that verify them post-precoloring.
        //
        // Three-walk flow:
        //   - Prep walk builds per-call `args_claim` bitmaps (O(1)
        //     collision checks downstream).
        //   - Rets walk visits each call's rets in order. Each ret's
        //     decision folds in two cascade rules:
        //       * return Transfer prefix: once a ret resolves to Home, all
        //         later rets in this call's list cascade to Home too.
        //       * return monotonicity: a candidate `Transfer(i)` is accepted
        //         only if `i` strictly exceeds the previous Transfer index
        //         in this list; non-strict candidates cascade to Home.
        //     A separate per-ret collision check (lifetime overlap vs.
        //     the next call's args claim) covers the cross-call
        //     same-slot case that return monotonicity alone cannot see.
        //   - Args walk fills in remaining args-side precolors for
        //     ValueIds not handled by the rets walk (params, locals,
        //     computed values).

        // `ValueId` -> Transfer position `j` for the call arg/ret that this
        // ValueId will be bound to; absence means the ValueId is not
        // Transfer-eligible and will get a Home slot.
        let mut transfer_precolor: UnorderedMap<SsaSlot, TransferPosition> = UnorderedMap::new();
        let mut max_transfer_positions: u16 = 0;

        let args_walk_eligible = |value_id: &SsaSlot, call_pos: usize| -> bool {
            if !value_id.is_value_id() {
                return false;
            }
            if stloc_targets.contains_key(value_id) || coalesce_to_local.contains_key(value_id) {
                return false;
            }
            let Some(&dp) = def_pos.get(value_id) else {
                return false;
            };
            // Live range crosses an earlier call — transfer slot would be clobbered.
            if has_any_in_range(&call_positions, dp + 1, call_pos) {
                return false;
            }
            // ValueId must end its life at this call (used as one of its args).
            live_end.get(value_id) == Some(&call_pos)
        };

        let rets_walk_eligible = |value_id: &SsaSlot, call_pos: usize, next_call: usize| -> bool {
            if !value_id.is_value_id() {
                return false;
            }
            // Caller passes ret ValueIds of `call_pos`; SSA single-def then
            // implies they're absent from `coalesce_to_local` and have
            // entries in both `def_pos` and `live_end`. Make those
            // implicit preconditions load-bearing so a future SSA
            // regression trips here instead of producing wrong precolors.
            debug_assert!(
                !coalesce_to_local.contains_key(value_id),
                "ret ValueId {:?} is in coalesce_to_local, violates SSA single-def",
                value_id
            );
            debug_assert!(
                live_end.contains_key(value_id),
                "ret ValueId {:?} missing from live_end",
                value_id
            );
            if stloc_targets.contains_key(value_id) {
                return false;
            }
            // Dead-on-arrival rets must go to Home.
            if live_end.get(value_id) == Some(&call_pos) {
                return false;
            }
            if let Some(&lu) = live_end.get(value_id)
                && lu >= next_call
            {
                return false;
            }
            true
        };

        // Prep walk — `args_claim` bitmaps. One bit per arg position, sized
        // exactly to `args.len()` so wide call signatures don't need a fixed
        // cap. `SmallBitVec` keeps storage inline for typical small calls and
        // only spills to the heap for genuinely wide signatures.
        let mut args_claim: Vec<SmallBitVec> = Vec::with_capacity(call_positions.len());
        for &call_pos in &call_positions {
            let Some((rets, args)) = direct_call_rets_and_args(&instrs[call_pos]) else {
                args_claim.push(SmallBitVec::new());
                continue;
            };
            let call_width = std::cmp::max(args.len(), rets.len()) as u16;
            max_transfer_positions = max_transfer_positions.max(call_width);
            let mut bits = SmallBitVec::from_elem(args.len(), false);
            for (j, value_id) in args.iter().enumerate() {
                if args_walk_eligible(value_id, call_pos) {
                    bits.set(j, true);
                }
            }
            args_claim.push(bits);
        }

        // Rets walk — per-call rets visit.
        //
        // `handled_rets` records every ret ValueId we visit. The args
        // walk uses this to skip ValueIds that are rets of a prev call:
        // cascaded rets are *not* in `transfer_precolor` after the rets
        // walk, but the args walk must still skip them (else it would
        // re-insert an args-side precolor and undo the cascade).
        let mut handled_rets: UnorderedSet<SsaSlot> = UnorderedSet::new();
        for (call_idx, &call_pos) in call_positions.iter().enumerate() {
            let Some((rets, _)) = direct_call_rets_and_args(&instrs[call_pos]) else {
                continue;
            };
            let next_call_opt = call_positions.get(call_idx + 1).copied();
            let next_call = next_call_opt.unwrap_or(instrs.len());
            let next_args = next_call_opt
                .and_then(|pos| direct_call_rets_and_args(&instrs[pos]).map(|(_, args)| args));
            let next_args_claim = args_claim.get(call_idx + 1);

            let mut found_home = false;
            // Tracks the highest Transfer index inserted so far in this
            // call's rets list. Anchors the return-monotonicity
            // cascade: subsequent rets must take strictly larger
            // indices, else they cascade to Home.
            let mut last_transfer_index: i32 = -1;
            for (k, &value_id) in rets.iter().enumerate() {
                handled_rets.insert(value_id);
                if found_home || !value_id.is_value_id() {
                    continue;
                }
                let k = k as u16;

                // Decide value_id's slot. Three cases, in priority order:
                // (a) Pass-through: value_id is also at next call's args[j],
                //     args-walk-eligible there. Land at Transfer(j) and let
                //     args walk skip it via `handled_rets`.
                // (b) Rets-walk precolor at Transfer(k), unless next call's
                //     args[k] would claim the same slot AND lifetimes
                //     overlap. Slot reuse is fine when the next-call
                //     arg's def_pos > value_id's live_end.
                // (c) Otherwise Home, which trips the cascade flag.
                //
                // `bit(next_args_claim, j)` is the cached
                // `args_walk_eligible(next_args[j], next_call_pos)`
                // result from the prep walk; no need to recompute
                // the predicate here.
                let target: Option<TransferPosition> = if let Some(args) = next_args
                    && let Some(j) = args.iter().position(|a| *a == value_id)
                    && bit(next_args_claim, j)
                {
                    Some(TransferPosition(j as u16))
                } else if rets_walk_eligible(&value_id, call_pos, next_call) {
                    let collides = bit(next_args_claim, k as usize)
                        && next_arg_lifetime_overlaps(value_id, k, next_args, &def_pos, &live_end);
                    (!collides).then_some(TransferPosition(k))
                } else {
                    None
                };

                // Enforce return monotonicity: accept Transfer(i) only if
                // i > last_transfer_index. Non-strict candidates cascade
                // to Home.
                if let Some(i) = target
                    && (i.0 as i32) > last_transfer_index
                {
                    transfer_precolor.insert(value_id, i);
                    last_transfer_index = i.0 as i32;
                } else {
                    found_home = true;
                }
            }
        }

        // Args walk — args fill-in.
        for (call_idx, &call_pos) in call_positions.iter().enumerate() {
            let Some((_, args)) = direct_call_rets_and_args(&instrs[call_pos]) else {
                continue;
            };
            let claim = &args_claim[call_idx];
            for (j, &value_id) in args.iter().enumerate() {
                if !claim
                    .get(j)
                    .expect("args_claim sized to args.len() in prep walk")
                {
                    continue;
                }
                if handled_rets.contains(&value_id) {
                    continue;
                }
                transfer_precolor.insert(value_id, TransferPosition(j as u16));
            }
        }

        #[cfg(debug_assertions)]
        assert_transfer_invariants(
            instrs,
            &call_positions,
            &transfer_precolor,
            &def_pos,
            &live_end,
        );

        Self {
            live_end,
            stloc_targets,
            coalesce_to_local,
            transfer_precolor,
            max_transfer_positions,
        }
    }
}

/// Tests bit `j` of an optional `SmallBitVec`. Returns false if the
/// bitmap is missing or `j` is out of range. Centralizes the lookup
/// pattern used in both branches of the rets-walk decision.
fn bit(bv: Option<&SmallBitVec>, j: usize) -> bool {
    bv.and_then(|b| b.get(j)).unwrap_or(false)
}

/// True iff `value_id`'s lifetime overlaps the lifetime of the ValueId at
/// `next_args[k]` (if any).
///
/// `a_lu == b_def` is treated as disjoint: at a single instruction,
/// uses are read before defs are written, so the kill→redefine
/// boundary can safely share a slot.
///
/// Precondition: callers gate this with the next-call's args-claim
/// bit, which only fires for next-args[k] satisfying
/// `args_walk_eligible` — that means `next_args[k]` is a ValueId present
/// in `def_pos` and `live_end`. `value_id` is a ret of the current call,
/// so the same SSA invariant applies. Both lookups must hit.
fn next_arg_lifetime_overlaps(
    value_id: SsaSlot,
    k: u16,
    next_args: Option<&[SsaSlot]>,
    def_pos: &UnorderedMap<SsaSlot, usize>,
    live_end: &UnorderedMap<SsaSlot, usize>,
) -> bool {
    let Some(other) = next_args.and_then(|a| a.get(k as usize).copied()) else {
        return false;
    };
    let a_def = *def_pos
        .get(&value_id)
        .expect("ValueId in transfer_precolor missing from def_pos");
    let a_lu = *live_end
        .get(&value_id)
        .expect("ValueId in transfer_precolor missing from live_end");
    let b_def = *def_pos
        .get(&other)
        .expect("next-call arg-walk-eligible ValueId missing from def_pos");
    let b_lu = *live_end
        .get(&other)
        .expect("next-call arg-walk-eligible ValueId missing from live_end");
    !(a_lu <= b_def || b_lu <= a_def)
}

/// Verifies the seven Transfer invariants from the file header on the
/// just-built `transfer_precolor` map. Per-call invariants (arg
/// positionality, return monotonicity, pass-through contiguity, return
/// Transfer prefix, and the per-ValueId lifetime checks for block-local
/// lifetime, single use, not live across calls) are checked in a
/// single forward pass over `call_positions`. A final cross-call pass
/// checks slot-sharing — the no-overlapping-lifetimes property is
/// implied by arg positionality + return monotonicity + single use +
/// not live across calls but checked directly to localize any upstream
/// regression that violates it.
#[cfg(debug_assertions)]
fn assert_transfer_invariants(
    instrs: &[Instr<SsaSlot>],
    call_positions: &[usize],
    transfer_precolor: &UnorderedMap<SsaSlot, TransferPosition>,
    def_pos: &UnorderedMap<SsaSlot, usize>,
    live_end: &UnorderedMap<SsaSlot, usize>,
) {
    let is_transfer = |value_id: &SsaSlot| transfer_precolor.contains_key(value_id);
    let lifetime_of = |value_id: &SsaSlot| -> (usize, usize) {
        let dp = *def_pos
            .get(value_id)
            .expect("[block-local lifetime] Transfer-bound ValueId missing from def_pos");
        let lu = *live_end
            .get(value_id)
            .expect("[block-local lifetime] Transfer-bound ValueId missing from live_end");
        (dp, lu)
    };

    for (call_idx, &call_pos) in call_positions.iter().enumerate() {
        let Some((rets, args)) = direct_call_rets_and_args(&instrs[call_pos]) else {
            continue;
        };
        let prev_call = (call_idx > 0).then(|| call_positions[call_idx - 1]);
        let next_call = call_positions
            .get(call_idx + 1)
            .copied()
            .unwrap_or(instrs.len());

        // Structural invariants (arg positionality, return Transfer prefix, return
        // monotonicity).
        check_call_structural_invariants(args, rets, |slot| {
            if !slot.is_value_id() {
                return None;
            }
            transfer_precolor
                .get(slot)
                .copied()
                .map(|position| position.0)
        })
        .unwrap_or_else(|e| panic!("[transfer structural invariant] {e}"));

        // Pass-through contiguity: among args of `call_pos` defined at the
        // immediately-preceding call and bound to Transfer, the positions form
        // a contiguous interval. (Move's stack discipline only allows a
        // prefix of A's rets to feed B at consecutive positions of B.args
        // — see file header.)
        if let Some(prev) = prev_call {
            let from_prev: Vec<usize> = args
                .iter()
                .enumerate()
                .filter(|(_, value_id)| {
                    value_id.is_value_id()
                        && def_pos.get(value_id) == Some(&prev)
                        && is_transfer(value_id)
                })
                .map(|(j, _)| j)
                .collect();
            for w in from_prev.windows(2) {
                assert_eq!(
                    w[1],
                    w[0] + 1,
                    "[pass-through contiguity] args of call at {} from prev call at {} not contiguous: {:?}",
                    call_pos,
                    prev,
                    from_prev,
                );
            }
        }

        // For Transfer-bound args: def_pos and live_end exist (block-local
        // lifetime), live_end lands at this call (single use), and no
        // call sits strictly between def and live_end (not live across
        // calls).
        for value_id in args {
            if !value_id.is_value_id() || !is_transfer(value_id) {
                continue;
            }
            let (dp, lu) = lifetime_of(value_id);
            assert_eq!(
                lu, call_pos,
                "[single use] Transfer-bound arg ValueId {:?} live_end {} != call {}",
                value_id, lu, call_pos
            );
            assert!(
                !has_any_in_range(call_positions, dp + 1, lu),
                "[not live across calls] Transfer-bound arg ValueId {:?} live across a call (def {}, lu {})",
                value_id,
                dp,
                lu,
            );
        }

        // For Transfer-bound rets: def at `call_pos` (block-local lifetime),
        // live_end strictly after `call_pos` and at or before the next call
        // (single use; block end if no next call). The strict lower bound
        // rules out dead-on-arrival rets (def == lu == call_pos, zero uses),
        // which upstream eligibility checks already reject.
        for value_id in rets {
            if !value_id.is_value_id() || !is_transfer(value_id) {
                continue;
            }
            let (dp, lu) = lifetime_of(value_id);
            assert_eq!(
                dp, call_pos,
                "[block-local lifetime] Transfer-bound ret ValueId {:?} def {} != call {}",
                value_id, dp, call_pos
            );
            assert!(
                call_pos < lu && lu <= next_call,
                "[single use] Transfer-bound ret ValueId {:?} live_end {} not in (call {}, next call {}]",
                value_id,
                lu,
                call_pos,
                next_call,
            );
            // No call strictly between def and live_end. For the case
            // lu == next_call (pass-through), the next call is at the
            // boundary, not strictly inside.
            assert!(
                !has_any_in_range(call_positions, dp + 1, lu),
                "[not live across calls] Transfer-bound ret ValueId {:?} live across a call (def {}, lu {})",
                value_id,
                dp,
                lu,
            );
        }
    }

    // Cross-check (implied by arg positionality, return monotonicity,
    // single use, not live across calls): no two Transfer-bound ValueIds
    // with overlapping lifetimes share an Transfer slot. Cross-call reuse
    // with disjoint lifetimes is fine (the ld→call pattern around
    // Transfer(0) is the canonical example).
    //
    // Walks the IR rather than iterating `transfer_precolor` (UnorderedMap
    // doesn't expose iter()): every entry in `transfer_precolor` is either
    // a ret or an arg of some call, so this finds them all. A
    // pass-through ValueId that appears as both ret(k) of call A and
    // args[j] of call B is deduplicated via `seen_value_ids`. `BTreeMap`
    // for `by_position` keeps the failing-assertion error message
    // deterministic.
    let mut by_position: BTreeMap<TransferPosition, Vec<SsaSlot>> = BTreeMap::new();
    let mut seen_value_ids: UnorderedSet<SsaSlot> = UnorderedSet::new();
    for &call_pos in call_positions {
        let Some((rets, args)) = direct_call_rets_and_args(&instrs[call_pos]) else {
            continue;
        };
        for value_id in rets.iter().chain(args.iter()) {
            if !seen_value_ids.insert(*value_id) {
                continue;
            }
            if let Some(&position) = transfer_precolor.get(value_id) {
                by_position.entry(position).or_default().push(*value_id);
            }
        }
    }
    for (position, value_ids) in by_position {
        for i in 0..value_ids.len() {
            for j in (i + 1)..value_ids.len() {
                let a = value_ids[i];
                let b = value_ids[j];
                let (a_def, a_lu) = lifetime_of(&a);
                let (b_def, b_lu) = lifetime_of(&b);
                // `a_lu == b_def` is disjoint — within an instruction,
                // uses read before defs write.
                assert!(
                    a_lu <= b_def || b_lu <= a_def,
                    "Transfer slot reuse with overlapping lifetimes: {:?} \
                     (live [{}, {}]) and {:?} (live [{}, {}]) at Transfer({})",
                    a,
                    a_def,
                    a_lu,
                    b,
                    b_def,
                    b_lu,
                    position.0,
                );
            }
        }
    }
}

/// Returns `(rets, args)` for `Call`. `CallClosure` is
/// intentionally excluded: Transfer precoloring leaves closure calls alone
/// (they still count as call boundaries via `clobbers_transfer`, just not
/// destructured for slot inspection). For the boundary-wide counterpart
/// covering both variants, see `instr_utils::call_boundary_rets_and_args`.
#[inline]
fn direct_call_rets_and_args(instr: &Instr<SsaSlot>) -> Option<(&[SsaSlot], &[SsaSlot])> {
    if let Instr::Call { data } = instr {
        Some((&data.rets, &data.args))
    } else {
        None
    }
}

/// Returns true if `sorted` contains any element in the half-open range [lo, hi).
fn has_any_in_range(sorted: &[usize], lo: usize, hi: usize) -> bool {
    if lo >= hi {
        return false;
    }
    // Find the first element >= lo.
    let idx = sorted.partition_point(|&x| x < lo);
    idx < sorted.len() && sorted[idx] < hi
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stackless_exec_ir::CallData;
    use mono_move_core::types::EMPTY_TYPE_LIST;
    use move_binary_format::file_format::FunctionHandleIndex;

    /// Wide call signatures (past `SmallBitVec`'s inline-storage
    /// limit) must analyze without panicking.
    #[test]
    fn analyze_handles_wide_call_signatures() {
        // 200 args exercises `SmallBitVec`'s heap-allocated path.
        let args: Box<[SsaSlot]> = (0..200).map(SsaSlot::ValueId).collect();
        let instrs = vec![Instr::Call {
            data: Box::new(CallData {
                rets: Box::new([]),
                function_handle: FunctionHandleIndex(0),
                ty_args: EMPTY_TYPE_LIST,
                args,
            }),
        }];
        let analysis = BlockAnalysis::analyze(&instrs, |_| true);
        assert_eq!(analysis.max_transfer_positions, 200);
    }
}
