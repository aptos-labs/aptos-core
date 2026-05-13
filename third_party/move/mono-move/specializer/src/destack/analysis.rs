// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-basic-block analysis feeding the slot allocator. From a single
//! intra-block SSA slice it produces a [`BlockAnalysis`] — liveness
//! plus three "color this Vid into that slot" hint maps the allocator
//! uses to elide copies. See the [`BlockAnalysis`] field docs for the
//! precise semantics of each entry.
//!
//! # Xfer slot invariants
//!
//! `analyze` establishes several invariants on the resulting `xfer_precolor`
//! map. The slot allocator follows the precolor map verbatim: every Vid in it
//! lands at the named Xfer slot, and no other Vid lands at any Xfer slot. So
//! these are equivalently invariants on each call's args/rets in the
//! xfer-slot-allocated IR.
//!
//! 1. Block-local lifetime. Every Xfer-bound Vid has its def and last use
//!    within the same basic block. Block or function exit ends the slot's
//!    lifetime.
//! 2. Arg positionality. For any call, if `args[j]` is precolored to `Xfer(i)`,
//!    then `i == j`.
//! 3. Return monotonicity. For any call, if `rets[k1] == Xfer(i1)` and
//!    `rets[k2] == Xfer(i2)` with `k1 < k2`, then `i1 < i2`.
//! 4. Pass-through contiguity. For any pair of consecutive calls in the same
//!    block A then B, the positions in `B.args` whose Vids are defined at A and
//!    Xfer-bound form one contiguous interval.
//! 5. Return Xfer prefix. Within any call's `rets` list, all Xfer-bound entries
//!    precede all Home-bound entries.
//! 6. Each Xfer-bound Vid is read exactly once at or before the call-like
//!    instruction immediately following its def.
//! 7. Not live across calls. No Xfer-bound Vid's lifetime spans across (defined
//!    before and used after) call-like instruction.
//!
//! Notes:
//!
//! a. Return xfers don't have positionality. A `ret(k)` may be precolored to
//!   `Xfer(j)` with `j ≠ k` when the same Vid is also `B.args[j]` of the
//!   immediately-following call (the pass-through case).
//! b. No duplicate Xfer indices within a call's args or rets. Arg
//!    positionality forces `args[j] = Xfer(j)` for distinct `j`; return
//!    monotonicity forces rets to take strictly-increasing Xfer indices.
//! c. No cycle detection needed at lowering. Arg positionality and return
//!    monotonicity together make reverse-order emit provably safe by
//!    structural reasoning, with no appeal to bytecode stack semantics or
//!    runtime cycle detection.
//!
//! [`assert_xfer_invariants`] checks all seven invariants at the end of
//! `analyze` in debug builds.
//!
//! # Why `coalesce_to_local` aliasing is safe
//!
//! In SSA, a Vid is defined once and read-only thereafter — coalescing
//! never introduces Vid-side writes to the source local's slot. The
//! Vid's intended value at any read site `t` equals the local's value
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
//!    live, but the coalesced Vid is, at the IR level, an independent
//!    stack value — the borrow checker does not see the aliasing.
//!    Tracked by `home_mut_borrow_pos`; checked via `home_mut_borrowed`.
//!
//! With both channels clear over `[def_pos(vid)+1, live_end(vid))`, the
//! Vid's snapshot equals the local's slot at every read site and they
//! can share storage with no copy. `MutBorrowField` /
//! `MutBorrowVariantField` / `MutBorrowGlobal` need no tracking here:
//! their `src` is a ref Vid (or address), never a local, so any
//! local-storage mutation they cascade into was already gated on an
//! upstream `MutBorrowLoc`. (Field-level coalescing, if ever added,
//! would need to revisit this.)

use super::instr_utils::{for_each_def, for_each_use};
use crate::stackless_exec_ir::{Instr, Slot};
use shared_dsa::{UnorderedMap, UnorderedSet};
use smallbitvec::SmallBitVec;
#[cfg(debug_assertions)]
use std::collections::BTreeMap;

/// Analysis results for a single basic block. All map keys are `Vid`s.
pub(crate) struct BlockAnalysis {
    /// End of the `Vid`'s live range — the last instruction index where it
    /// is referenced (def or use). For a Vid that is defined but never
    /// used, equals its `def_pos`, marking the live range as collapsed to
    /// the def site. Used downstream as the "kill point" beyond which the
    /// Vid's slot can be reused.
    pub live_end: UnorderedMap<Slot, usize>,
    /// `Vid` → Home slot, when the Vid will later be moved into the Home
    /// slot (`Move(Home, vid)`, the `st_loc` shape produced by destack)
    /// and the Home slot is not accessed between the Vid's def and the
    /// store. Sound because destack emits this shape only for `StLoc`,
    /// where the Vid is popped — the move is its last use, so coloring
    /// the Vid into the Home slot makes the store a self-move that elides.
    pub stloc_targets: UnorderedMap<Slot, Slot>,
    /// `Vid` → local it was copied/moved out of, when the local is
    /// neither redefined nor mut-borrowed during the `Vid`'s live range.
    pub coalesce_to_local: UnorderedMap<Slot, Slot>,
    /// `Vid` → `Xfer` slot, for call args or rets whose live range
    /// doesn't cross any other call. See the file-header section
    /// "Xfer slot invariants" for the different properties this map
    /// satisfies.
    pub xfer_precolor: UnorderedMap<Slot, Slot>,
    /// Largest `max(args.len(), rets.len())` across all calls in the
    /// block — i.e., the number of distinct `Xfer(j)` positions any
    /// call uses.
    pub max_xfer_positions: u16,
}

impl BlockAnalysis {
    /// Analyze a basic block and produce hint maps for slot allocation.
    ///
    /// Pre: `instrs` is one basic block's SSA slice; each `Vid` is
    /// defined exactly once.
    ///
    /// Post: each entry's soundness condition is enforced before insert,
    /// and the three hint maps (`stloc_targets`, `coalesce_to_local`,
    /// `xfer_precolor`) are pairwise disjoint — `coalesce_to_local`
    /// skips Vids already in `stloc_targets`, and `xfer_precolor` skips
    /// Vids already in either earlier map. (`coalesce_to_local` ∩ call
    /// rets is empty by SSA: a Vid defined by a `Call` is never the
    /// `dst` of `Copy/Move(vid, Home)`.)
    pub(crate) fn analyze(instrs: &[Instr]) -> Self {
        // Forward scan: build per-`Vid` and per-`Home` position indices.
        // `Vid` -> last instruction index where it appears as def or use.
        let mut live_end: UnorderedMap<Slot, usize> = UnorderedMap::new();
        // `Vid` -> instruction index where it is defined.
        let mut def_pos: UnorderedMap<Slot, usize> = UnorderedMap::new();
        // `Home` -> sorted positions where the Home slot appears as def or use.
        // Used only for range-existence queries, so any duplicate `i` (from a
        // single instr touching the same Home as both def and use — destack
        // doesn't currently emit such shapes) would be harmless.
        let mut home_touch_pos: UnorderedMap<Slot, Vec<usize>> = UnorderedMap::new();
        // `Home` -> sorted positions where the Home slot appears as def only.
        let mut home_def_pos: UnorderedMap<Slot, Vec<usize>> = UnorderedMap::new();
        // `Home` -> sorted positions of `MutBorrowLoc` whose source is the
        // Home slot (see file header for why `coalesce_to_local` treats these
        // as conflicts).
        let mut home_mut_borrow_pos: UnorderedMap<Slot, Vec<usize>> = UnorderedMap::new();

        // [TODO]: we can reduce the number of passes over instructions.
        for (i, instr) in instrs.iter().enumerate() {
            for_each_use(instr, |slot| match slot {
                Slot::Vid(_) => {
                    live_end.insert(slot, i);
                },
                Slot::Home(_) => {
                    home_touch_pos.entry(slot).or_default().push(i);
                },
                Slot::Xfer(_) => {
                    // cannot appear
                },
            });
            for_each_def(instr, |slot| match slot {
                Slot::Vid(_) => {
                    live_end.entry(slot).or_insert(i);
                    def_pos.entry(slot).or_insert(i);
                },
                Slot::Home(_) => {
                    home_touch_pos.entry(slot).or_default().push(i);
                    home_def_pos.entry(slot).or_default().push(i);
                },
                Slot::Xfer(_) => {
                    // cannot appear
                },
            });
            if let Instr::MutBorrowLoc(_, src @ Slot::Home(_)) = instr {
                home_mut_borrow_pos.entry(*src).or_default().push(i);
            }
        }

        // Call positions — any instruction that clobbers xfer slots.
        // Already sorted since we iterate in order.
        let call_positions: Vec<usize> = instrs
            .iter()
            .enumerate()
            .filter(|(_, ins)| clobbers_xfer(ins))
            .map(|(i, _)| i)
            .collect();

        // StLoc look-ahead: map `Vid` → Home slot when the `Vid` is produced
        // and later stored to that Home slot, with the Home slot not accessed
        // in between. Check uses binary search on home_touch_pos: O(log n)
        // per candidate.
        let mut stloc_targets: UnorderedMap<Slot, Slot> = UnorderedMap::new();
        for (i, instr) in instrs.iter().enumerate() {
            if let Instr::Move(dst, src) = instr
                && dst.is_home()
                && src.is_vid()
                && !stloc_targets.contains_key(src)
            {
                let dp = def_pos.get(src).copied().unwrap_or(0);
                let touches = home_touch_pos.get(dst).map(|v| v.as_slice()).unwrap_or(&[]);
                let home_touched = has_any_in_range(touches, dp + 1, i);
                if !home_touched {
                    stloc_targets.insert(*src, *dst);
                }
            }
        }

        // `CopyLoc` / `MoveLoc` coalescing. Range checks on
        // `home_def_pos` and `home_mut_borrow_pos` are O(log n) each
        // via binary search.
        let mut coalesce_to_local: UnorderedMap<Slot, Slot> = UnorderedMap::new();
        for (i, instr) in instrs.iter().enumerate() {
            if let Instr::Copy(dst @ Slot::Vid(_), src @ Slot::Home(_))
            | Instr::Move(dst @ Slot::Vid(_), src @ Slot::Home(_)) = instr
            {
                let vid = *dst;
                if let Some(&lu) = live_end.get(&vid)
                    && lu > i
                    && !stloc_targets.contains_key(&vid)
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
                        coalesce_to_local.insert(vid, local);
                    }
                }
            }
        }

        // Xfer slot precoloring.
        //
        // Establishes the seven Xfer invariants documented in the file
        // header. See `assert_xfer_invariants` for the debug-build
        // checks that verify them post-precoloring.
        //
        // Three-walk flow:
        //   - Prep walk builds per-call `args_claim` bitmaps (O(1)
        //     collision checks downstream).
        //   - Rets walk visits each call's rets in order. Each ret's
        //     decision folds in two cascade rules:
        //       * return Xfer prefix: once a ret resolves to Home, all
        //         later rets in this call's list cascade to Home too.
        //       * return monotonicity: a candidate `Xfer(i)` is accepted
        //         only if `i` strictly exceeds the previous Xfer index
        //         in this list; non-strict candidates cascade to Home.
        //     A separate per-ret collision check (lifetime overlap vs.
        //     the next call's args claim) covers the cross-call
        //     same-slot case that return monotonicity alone cannot see.
        //   - Args walk fills in remaining args-side precolors for
        //     Vids not handled by the rets walk (params, locals,
        //     computed values).

        // `Vid` -> `Xfer(j)` for the call arg/ret that this Vid will be
        // bound to. Every value is a `Slot::Xfer(_)`; absence means the
        // Vid is not Xfer-eligible and will get a Home slot.
        let mut xfer_precolor: UnorderedMap<Slot, Slot> = UnorderedMap::new();
        let mut max_xfer_positions: u16 = 0;

        let args_walk_eligible = |vid: &Slot, call_pos: usize| -> bool {
            if !vid.is_vid() {
                return false;
            }
            if stloc_targets.contains_key(vid) || coalesce_to_local.contains_key(vid) {
                return false;
            }
            let Some(&dp) = def_pos.get(vid) else {
                return false;
            };
            // Live range crosses an earlier call — xfer slot would be clobbered.
            if has_any_in_range(&call_positions, dp + 1, call_pos) {
                return false;
            }
            // Vid must end its life at this call (used as one of its args).
            live_end.get(vid) == Some(&call_pos)
        };

        let rets_walk_eligible = |vid: &Slot, call_pos: usize, next_call: usize| -> bool {
            if !vid.is_vid() {
                return false;
            }
            // Caller passes ret Vids of `call_pos`; SSA single-def then
            // implies they're absent from `coalesce_to_local` and have
            // entries in both `def_pos` and `live_end`. Make those
            // implicit preconditions load-bearing so a future SSA
            // regression trips here instead of producing wrong precolors.
            debug_assert!(
                !coalesce_to_local.contains_key(vid),
                "ret Vid {:?} is in coalesce_to_local, violates SSA single-def",
                vid
            );
            debug_assert!(
                live_end.contains_key(vid),
                "ret Vid {:?} missing from live_end",
                vid
            );
            if stloc_targets.contains_key(vid) {
                return false;
            }
            // Dead-on-arrival rets must go to Home.
            if live_end.get(vid) == Some(&call_pos) {
                return false;
            }
            if let Some(&lu) = live_end.get(vid)
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
            let Some((rets, args)) = call_rets_and_args(&instrs[call_pos]) else {
                args_claim.push(SmallBitVec::new());
                continue;
            };
            let call_width = std::cmp::max(args.len(), rets.len()) as u16;
            max_xfer_positions = max_xfer_positions.max(call_width);
            let mut bits = SmallBitVec::from_elem(args.len(), false);
            for (j, vid) in args.iter().enumerate() {
                if args_walk_eligible(vid, call_pos) {
                    bits.set(j, true);
                }
            }
            args_claim.push(bits);
        }

        // Rets walk — per-call rets visit.
        //
        // `handled_rets` records every ret Vid we visit. The args
        // walk uses this to skip Vids that are rets of a prev call:
        // cascaded rets are *not* in `xfer_precolor` after the rets
        // walk, but the args walk must still skip them (else it would
        // re-insert an args-side precolor and undo the cascade).
        let mut handled_rets: UnorderedSet<Slot> = UnorderedSet::new();
        for (call_idx, &call_pos) in call_positions.iter().enumerate() {
            let Some((rets, _)) = call_rets_and_args(&instrs[call_pos]) else {
                continue;
            };
            let next_call_opt = call_positions.get(call_idx + 1).copied();
            let next_call = next_call_opt.unwrap_or(instrs.len());
            let next_args =
                next_call_opt.and_then(|p| call_rets_and_args(&instrs[p]).map(|(_, a)| a));
            let next_args_claim = args_claim.get(call_idx + 1);

            let mut found_home = false;
            // Tracks the highest Xfer index inserted so far in this
            // call's rets list. Anchors the return-monotonicity
            // cascade: subsequent rets must take strictly larger
            // indices, else they cascade to Home.
            let mut last_xfer_index: i32 = -1;
            for (k, &vid) in rets.iter().enumerate() {
                handled_rets.insert(vid);
                if found_home || !vid.is_vid() {
                    continue;
                }
                let k = k as u16;

                // Decide vid's slot. Three cases, in priority order:
                // (a) Pass-through: vid is also at next call's args[j],
                //     args-walk-eligible there. Land at Xfer(j) and let
                //     args walk skip it via `handled_rets`.
                // (b) Rets-walk precolor at Xfer(k), unless next call's
                //     args[k] would claim the same slot AND lifetimes
                //     overlap. Slot reuse is fine when the next-call
                //     arg's def_pos > vid's live_end.
                // (c) Otherwise Home, which trips the cascade flag.
                //
                // `bit(next_args_claim, j)` is the cached
                // `args_walk_eligible(next_args[j], next_call_pos)`
                // result from the prep walk; no need to recompute
                // the predicate here.
                let target: Option<Slot> = if let Some(args) = next_args
                    && let Some(j) = args.iter().position(|a| *a == vid)
                    && bit(next_args_claim, j)
                {
                    Some(Slot::Xfer(j as u16))
                } else if rets_walk_eligible(&vid, call_pos, next_call) {
                    let collides = bit(next_args_claim, k as usize)
                        && next_arg_lifetime_overlaps(vid, k, next_args, &def_pos, &live_end);
                    (!collides).then_some(Slot::Xfer(k))
                } else {
                    None
                };

                // Enforce return monotonicity: accept Xfer(i) only if
                // i > last_xfer_index. Non-strict candidates cascade
                // to Home.
                if let Some(Slot::Xfer(i)) = target
                    && (i as i32) > last_xfer_index
                {
                    xfer_precolor.insert(vid, Slot::Xfer(i));
                    last_xfer_index = i as i32;
                } else {
                    found_home = true;
                }
            }
        }

        // Args walk — args fill-in.
        for (call_idx, &call_pos) in call_positions.iter().enumerate() {
            let Some((_, args)) = call_rets_and_args(&instrs[call_pos]) else {
                continue;
            };
            let claim = &args_claim[call_idx];
            for (j, &vid) in args.iter().enumerate() {
                if !claim
                    .get(j)
                    .expect("args_claim sized to args.len() in prep walk")
                {
                    continue;
                }
                if handled_rets.contains(&vid) {
                    continue;
                }
                xfer_precolor.insert(vid, Slot::Xfer(j as u16));
            }
        }

        #[cfg(debug_assertions)]
        assert_xfer_invariants(instrs, &call_positions, &xfer_precolor, &def_pos, &live_end);

        Self {
            live_end,
            stloc_targets,
            coalesce_to_local,
            xfer_precolor,
            max_xfer_positions,
        }
    }
}

/// Tests bit `j` of an optional `SmallBitVec`. Returns false if the
/// bitmap is missing or `j` is out of range. Centralizes the lookup
/// pattern used in both branches of the rets-walk decision.
fn bit(bv: Option<&SmallBitVec>, j: usize) -> bool {
    bv.and_then(|b| b.get(j)).unwrap_or(false)
}

/// True iff `vid`'s lifetime overlaps the lifetime of the Vid at
/// `next_args[k]` (if any).
///
/// `a_lu == b_def` is treated as disjoint: at a single instruction,
/// uses are read before defs are written, so the kill→redefine
/// boundary can safely share a slot.
///
/// Precondition: callers gate this with the next-call's args-claim
/// bit, which only fires for next-args[k] satisfying
/// `args_walk_eligible` — that means `next_args[k]` is a Vid present
/// in `def_pos` and `live_end`. `vid` is a ret of the current call,
/// so the same SSA invariant applies. Both lookups must hit.
fn next_arg_lifetime_overlaps(
    vid: Slot,
    k: u16,
    next_args: Option<&[Slot]>,
    def_pos: &UnorderedMap<Slot, usize>,
    live_end: &UnorderedMap<Slot, usize>,
) -> bool {
    let Some(other) = next_args.and_then(|a| a.get(k as usize).copied()) else {
        return false;
    };
    let a_def = *def_pos
        .get(&vid)
        .expect("Vid in xfer_precolor missing from def_pos");
    let a_lu = *live_end
        .get(&vid)
        .expect("Vid in xfer_precolor missing from live_end");
    let b_def = *def_pos
        .get(&other)
        .expect("next-call arg-walk-eligible Vid missing from def_pos");
    let b_lu = *live_end
        .get(&other)
        .expect("next-call arg-walk-eligible Vid missing from live_end");
    !(a_lu <= b_def || b_lu <= a_def)
}

/// Verifies the seven Xfer invariants from the file header on the
/// just-built `xfer_precolor` map. Per-call invariants (arg
/// positionality, return monotonicity, pass-through contiguity, return
/// Xfer prefix, and the per-Vid lifetime checks for block-local
/// lifetime, single use, not live across calls) are checked in a
/// single forward pass over `call_positions`. A final cross-call pass
/// checks slot-sharing — the no-overlapping-lifetimes property is
/// implied by arg positionality + return monotonicity + single use +
/// not live across calls but checked directly to localize any upstream
/// regression that violates it.
#[cfg(debug_assertions)]
fn assert_xfer_invariants(
    instrs: &[Instr],
    call_positions: &[usize],
    xfer_precolor: &UnorderedMap<Slot, Slot>,
    def_pos: &UnorderedMap<Slot, usize>,
    live_end: &UnorderedMap<Slot, usize>,
) {
    let is_xfer = |vid: &Slot| matches!(xfer_precolor.get(vid), Some(Slot::Xfer(_)));
    let lifetime_of = |vid: &Slot| -> (usize, usize) {
        let dp = *def_pos
            .get(vid)
            .expect("[block-local lifetime] Xfer-bound Vid missing from def_pos");
        let lu = *live_end
            .get(vid)
            .expect("[block-local lifetime] Xfer-bound Vid missing from live_end");
        (dp, lu)
    };

    for (call_idx, &call_pos) in call_positions.iter().enumerate() {
        let Some((rets, args)) = call_rets_and_args(&instrs[call_pos]) else {
            continue;
        };
        let prev_call = (call_idx > 0).then(|| call_positions[call_idx - 1]);
        let next_call = call_positions
            .get(call_idx + 1)
            .copied()
            .unwrap_or(instrs.len());

        // Arg positionality: args[j] precolored to Xfer must be Xfer(j).
        for (j, vid) in args.iter().enumerate() {
            if let Some(&Slot::Xfer(i)) = xfer_precolor.get(vid) {
                assert_eq!(
                    i, j as u16,
                    "[arg positionality] args[{}] precolored to Xfer({})",
                    j, i
                );
            }
        }

        // Return Xfer prefix: rets are an Xfer prefix followed by a Home
        // suffix. Return monotonicity: within the prefix, Xfer indices
        // strictly increase with k.
        let mut seen_home = false;
        let mut last_xfer: Option<u16> = None;
        for (k, vid) in rets.iter().enumerate() {
            match xfer_precolor.get(vid) {
                Some(&Slot::Xfer(i)) if vid.is_vid() => {
                    assert!(
                        !seen_home,
                        "[return Xfer prefix] rets[{}] precolored to Xfer follows a Home ret",
                        k
                    );
                    if let Some(prev) = last_xfer {
                        assert!(
                            i > prev,
                            "[return monotonicity] rets[{}] = Xfer({}) ≤ prev Xfer({})",
                            k,
                            i,
                            prev
                        );
                    }
                    last_xfer = Some(i);
                },
                _ => seen_home = true,
            }
        }

        // Pass-through contiguity: among args of `call_pos` defined at the
        // immediately-preceding call and bound to Xfer, the positions form
        // a contiguous interval. (Move's stack discipline only allows a
        // prefix of A's rets to feed B at consecutive positions of B.args
        // — see file header.)
        if let Some(prev) = prev_call {
            let from_prev: Vec<usize> = args
                .iter()
                .enumerate()
                .filter(|(_, vid)| vid.is_vid() && def_pos.get(vid) == Some(&prev) && is_xfer(vid))
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

        // For Xfer-bound args: def_pos and live_end exist (block-local
        // lifetime), live_end lands at this call (single use), and no
        // call sits strictly between def and live_end (not live across
        // calls).
        for vid in args {
            if !vid.is_vid() || !is_xfer(vid) {
                continue;
            }
            let (dp, lu) = lifetime_of(vid);
            assert_eq!(
                lu, call_pos,
                "[single use] Xfer-bound arg Vid {:?} live_end {} != call {}",
                vid, lu, call_pos
            );
            assert!(
                !has_any_in_range(call_positions, dp + 1, lu),
                "[not live across calls] Xfer-bound arg Vid {:?} live across a call (def {}, lu {})",
                vid,
                dp,
                lu,
            );
        }

        // For Xfer-bound rets: def at `call_pos` (block-local lifetime),
        // live_end strictly after `call_pos` and at or before the next call
        // (single use; block end if no next call). The strict lower bound
        // rules out dead-on-arrival rets (def == lu == call_pos, zero uses),
        // which upstream eligibility checks already reject.
        for vid in rets {
            if !vid.is_vid() || !is_xfer(vid) {
                continue;
            }
            let (dp, lu) = lifetime_of(vid);
            assert_eq!(
                dp, call_pos,
                "[block-local lifetime] Xfer-bound ret Vid {:?} def {} != call {}",
                vid, dp, call_pos
            );
            assert!(
                call_pos < lu && lu <= next_call,
                "[single use] Xfer-bound ret Vid {:?} live_end {} not in (call {}, next call {}]",
                vid,
                lu,
                call_pos,
                next_call,
            );
            // No call strictly between def and live_end. For the case
            // lu == next_call (pass-through), the next call is at the
            // boundary, not strictly inside.
            assert!(
                !has_any_in_range(call_positions, dp + 1, lu),
                "[not live across calls] Xfer-bound ret Vid {:?} live across a call (def {}, lu {})",
                vid,
                dp,
                lu,
            );
        }
    }

    // Cross-check (implied by arg positionality, return monotonicity,
    // single use, not live across calls): no two Xfer-bound Vids
    // with overlapping lifetimes share an Xfer slot. Cross-call reuse
    // with disjoint lifetimes is fine (the ld→call pattern around
    // Xfer(0) is the canonical example).
    //
    // Walks the IR rather than iterating `xfer_precolor` (UnorderedMap
    // doesn't expose iter()): every entry in `xfer_precolor` is either
    // a ret or an arg of some call, so this finds them all. A
    // pass-through Vid that appears as both ret(k) of call A and
    // args[j] of call B is deduplicated via `seen_vids`. `BTreeMap`
    // for `by_slot` keeps the failing-assertion error message
    // deterministic.
    let mut by_slot: BTreeMap<Slot, Vec<Slot>> = BTreeMap::new();
    let mut seen_vids: UnorderedSet<Slot> = UnorderedSet::new();
    for &call_pos in call_positions {
        let Some((rets, args)) = call_rets_and_args(&instrs[call_pos]) else {
            continue;
        };
        for vid in rets.iter().chain(args.iter()) {
            if !seen_vids.insert(*vid) {
                continue;
            }
            if let Some(&slot) = xfer_precolor.get(vid)
                && matches!(slot, Slot::Xfer(_))
            {
                by_slot.entry(slot).or_default().push(*vid);
            }
        }
    }
    for (slot, vids) in by_slot {
        for i in 0..vids.len() {
            for j in (i + 1)..vids.len() {
                let a = vids[i];
                let b = vids[j];
                let (a_def, a_lu) = lifetime_of(&a);
                let (b_def, b_lu) = lifetime_of(&b);
                // `a_lu == b_def` is disjoint — within an instruction,
                // uses read before defs write.
                assert!(
                    a_lu <= b_def || b_lu <= a_def,
                    "Xfer slot reuse with overlapping lifetimes: {:?} \
                     (live [{}, {}]) and {:?} (live [{}, {}]) at {:?}",
                    a,
                    a_def,
                    a_lu,
                    b,
                    b_def,
                    b_lu,
                    slot,
                );
            }
        }
    }
}

/// Call-like instructions (`Call`, `CallGeneric`, `CallClosure`) that
/// clobber Xfer slots and act as call boundaries for liveness analysis.
#[inline]
fn clobbers_xfer(instr: &Instr) -> bool {
    matches!(
        instr,
        Instr::Call(..) | Instr::CallGeneric(..) | Instr::CallClosure(..)
    )
}

/// Returns `(rets, args)` for `Call` / `CallGeneric`. `CallClosure` is
/// intentionally excluded: Xfer precoloring leaves closure calls alone
/// (they still count as call boundaries via `clobbers_xfer`, just not
/// destructured for slot inspection).
#[inline]
fn call_rets_and_args(instr: &Instr) -> Option<(&[Slot], &[Slot])> {
    if let Instr::Call(rets, _, args) | Instr::CallGeneric(rets, _, args) = instr {
        Some((rets, args))
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
    use move_binary_format::file_format::FunctionHandleIndex;

    /// Wide call signatures (past `SmallBitVec`'s inline-storage
    /// limit) must analyze without panicking.
    #[test]
    fn analyze_handles_wide_call_signatures() {
        // 200 args exercises `SmallBitVec`'s heap-allocated path.
        let args: Vec<Slot> = (0..200).map(Slot::Vid).collect();
        let instrs = vec![Instr::Call(vec![], FunctionHandleIndex(0), args)];
        let analysis = BlockAnalysis::analyze(&instrs);
        assert_eq!(analysis.max_xfer_positions, 200);
    }
}
