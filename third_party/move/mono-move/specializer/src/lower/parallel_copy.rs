// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Parallel-copy primitives.
//!
//! - [`Copy`] — uniform `(src, dst, width)` representation of a byte-copy.
//! - [`emit_parallel_copy`] — emits copies without overwriting unread sources.
//!   When no pending copy is safe, it saves a cycle member's source in scratch.
//!   Used by `Instr::Ret` lowering, where return destinations overlap home
//!   slots and can require swaps, as in `swap(a, b) -> (b, a)`.
//! - [`reverse_emit_is_safe`] — debug check that a list of copies forms a
//!   forward-only dependency graph, exactly the property that makes
//!   reverse-order emission sound.
//!
//! `lower_call`'s arg-setup path inlines reverse-order emission directly
//! and uses [`reverse_emit_is_safe`] as a debug assertion. Soundness
//! rests on arg positionality + return monotonicity (see
//! `BlockAnalysis::analyze`): pass-through Transfer args land at
//! `arg_offset(j) ≥ ret_offset(k_j)` everywhere, so the dependency graph
//! is forward-only. Home args' sources live in the home region (offsets
//! `< frame_data_size`), disjoint from the arg region. No cycles, so no
//! scratch needed for arg setup.

use super::{lower_utils::ranges_overlap, LoweringError};
use mono_move_core::{
    checked_align_up_u32, FrameOffset, MicroOp, VMInternalError, VMResult, MAX_ALIGN,
};
use smallbitvec::SmallBitVec;
use smallvec::SmallVec;

/// Stack-resident capacity of the `SmallVec`s sized to the typical
/// small-N path. Emits involving at most this many copies avoid heap
/// allocation; wider signatures spill to the heap transparently.
const STACK_COPY_CAPACITY: usize = 4;

/// Represents a copy of `width` bytes from `src` to `dst`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Copy {
    pub src: FrameOffset,
    pub dst: FrameOffset,
    pub width: u32,
}

/// Pushes onto `out` the `MicroOp::Move`/`Move8` ops that perform `copies` with
/// parallel-copy semantics: every destination receives its source's original bytes.
///
/// Uses Kahn's topological sort, with an edge `Y → X` when `X.dst` overlaps
/// `Y.src`. A copy with no blockers can be emitted safely. When none is safe,
/// [`find_cycle_member`] selects a source to save in `MAX_ALIGN`-aligned scratch;
/// that copy reads the saved bytes once its destination is safe to write.
/// Mixed widths can leave a saved copy blocked by another cycle, so its scratch
/// bytes must remain intact until it emits. Scratch is reused only when no saved
/// copy remains pending. Each copy is saved at most once, giving at most
/// `2 * copies.len()` moves.
///
/// Trivial copies (`width == 0` or `src == dst`) are filtered up front.
///
/// Destinations must not overlap each other. `scratch` must be at or above the
/// end of every source and destination range, with space beyond it available
/// for scratch use. Returns the bytes used from `scratch`, including alignment
/// padding, which the caller must reserve.
///
/// TODO(security): we should consider adding metering to this function to
/// protect against adversarial inputs.
pub(crate) fn emit_parallel_copy(
    out: &mut Vec<MicroOp>,
    copies: &[Copy],
    scratch: FrameOffset,
) -> VMResult<u32> {
    // Filter trivial copies. Inline-stack-allocated for the common
    // small-N path; spills to heap only for adversarial wide signatures.
    let mut copies: SmallVec<[Copy; STACK_COPY_CAPACITY]> = copies
        .iter()
        .copied()
        .filter(|c| c.width > 0 && c.src != c.dst)
        .collect();
    let n = copies.len();
    if n == 0 {
        return Ok(0);
    }
    if n == 1 {
        emit_one(out, copies[0]);
        return Ok(0);
    }

    // Invariants:
    // 1. All source and destination ranges end at or before `scratch`.
    // 2. Destinations are disjoint; overlapping writes would make the result
    //    depend on copy order.
    #[cfg(debug_assertions)]
    {
        for c in copies.iter() {
            debug_assert!(
                c.src.0 + c.width <= scratch.0 && c.dst.0 + c.width <= scratch.0,
                "copy [{}, {}) -> [{}, {}) reaches past scratch {}",
                c.src.0,
                c.src.0 + c.width,
                c.dst.0,
                c.dst.0 + c.width,
                scratch.0,
            );
        }
        for i in 0..copies.len() {
            for j in (i + 1)..copies.len() {
                debug_assert!(
                    !ranges_overlap(
                        copies[i].dst.0,
                        copies[i].width,
                        copies[j].dst.0,
                        copies[j].width,
                    ),
                    "copies {} and {} write overlapping bytes: \
                     [{}, {}) and [{}, {})",
                    i,
                    j,
                    copies[i].dst.0,
                    copies[i].dst.0 + copies[i].width,
                    copies[j].dst.0,
                    copies[j].dst.0 + copies[j].width,
                );
            }
        }
    }

    // For a pending copy `i`, `blockers[i]` identifies sources that must be
    // read or saved before `i` writes its destination. Emitting or saving a
    // copy clears its bit from every other pending copy's blocker set.
    // A copy is safe when `alive[i] && blockers[i].all_false()`.
    let mut blockers: SmallVec<[SmallBitVec; STACK_COPY_CAPACITY]> = (0..n)
        .map(|i| {
            let mut bv = SmallBitVec::from_elem(n, false);
            for j in 0..n {
                if i != j
                    && ranges_overlap(
                        copies[i].dst.0,
                        copies[i].width,
                        copies[j].src.0,
                        copies[j].width,
                    )
                {
                    bv.set(j, true);
                }
            }
            bv
        })
        .collect();
    let mut alive = SmallBitVec::from_elem(n, true);
    let mut remaining = n;
    // Scratch allocations start at or after `scratch_top`; `scratch_end` tracks
    // the largest end offset used. A source at or above `scratch` reads saved bytes.
    let mut scratch_top = scratch.0;
    let mut scratch_end = scratch.0;

    // Terminates within 2n iterations: each iteration emits a pending copy or
    // saves one. `find_cycle_member` returns only a copy named in some blocker
    // set, and saving clears that copy from every blocker set for good, so no
    // copy is saved twice.
    while remaining > 0 {
        let safe =
            (0..n).find(|&i| alive.get(i).expect("alive sized to n") && blockers[i].all_false());

        if let Some(i) = safe {
            emit_one(out, copies[i]);
            mark_emitted(i, n, &mut alive, &mut blockers);
            remaining -= 1;
        } else {
            let chosen = find_cycle_member(&alive, &blockers)
                .ok_or_else(|| VMInternalError::new(LoweringError::ParallelCopyNoProgress))?;
            // Reuse the region from its start once no pending copy reads it.
            if !(0..n).any(|idx| {
                alive.get(idx).expect("alive sized to n") && copies[idx].src.0 >= scratch.0
            }) {
                scratch_top = scratch.0;
            }
            let width = copies[chosen].width;
            let (slot, slot_end) = checked_align_up_u32(scratch_top, MAX_ALIGN as u32)
                .and_then(|slot| Some((slot, slot.checked_add(width)?)))
                .ok_or_else(|| VMInternalError::new(LoweringError::ParallelCopyScratchOverflow))?;
            emit_one(out, Copy {
                src: copies[chosen].src,
                dst: FrameOffset(slot),
                width,
            });
            // Scratch overlaps no destination, so `chosen` no longer blocks
            // other copies. It remains pending until its own destination is safe.
            copies[chosen].src = FrameOffset(slot);
            clear_blocker(chosen, n, &alive, &mut blockers);
            scratch_top = slot_end;
            scratch_end = scratch_end.max(slot_end);
        }
    }
    Ok(scratch_end - scratch.0)
}

/// Follows blockers from the first pending copy until a copy repeats.
/// Requires at least one pending copy and no safe copy. Since each pending
/// copy has a pending blocker, the walk must reach a cycle. Returns `None`
/// if no copy is pending or the walk reaches a copy with no blocker.
fn find_cycle_member(alive: &SmallBitVec, blockers: &[SmallBitVec]) -> Option<usize> {
    let count = alive.len();
    let mut visited = SmallBitVec::from_elem(count, false);
    let mut current = (0..count).find(|&idx| alive.get(idx).expect("alive sized to count"))?;
    while !visited.get(current).expect("visited sized to count") {
        visited.set(current, true);
        current =
            (0..count).find(|&idx| blockers[current].get(idx).expect("blockers sized to count"))?;
    }
    Some(current)
}

/// Mark copy `i` as emitted: clear it from `alive` and from every
/// other still-alive blocker set.
#[inline]
fn mark_emitted(i: usize, n: usize, alive: &mut SmallBitVec, blockers: &mut [SmallBitVec]) {
    alive.set(i, false);
    clear_blocker(i, n, alive, blockers);
}

/// Clear bit `i` from `blockers[k]` for every alive `k != i`.
#[inline]
fn clear_blocker(i: usize, n: usize, alive: &SmallBitVec, blockers: &mut [SmallBitVec]) {
    for (k, blocker) in blockers.iter_mut().enumerate().take(n) {
        if k != i && alive.get(k).expect("alive sized to n") {
            blocker.set(i, false);
        }
    }
}

#[inline]
fn emit_one(out: &mut Vec<MicroOp>, c: Copy) {
    if c.width == 8 {
        out.push(MicroOp::Move8 {
            dst: c.dst,
            src: c.src,
        });
    } else {
        out.push(MicroOp::Move {
            dst: c.dst,
            src: c.src,
            size: c.width,
        });
    }
}

/// For every pair `(j_a, j_b)` with `j_a < j_b`, `copies[j_a].src`
/// must not overlap `copies[j_b].dst`. Equivalently, "no low-j src
/// is clobbered by a high-j dst" — the property reverse-order emit
/// needs.
pub(crate) fn reverse_emit_is_safe(copies: &[Copy]) -> bool {
    for j_a in 0..copies.len() {
        for j_b in (j_a + 1)..copies.len() {
            if ranges_overlap(
                copies[j_a].src.0,
                copies[j_a].width,
                copies[j_b].dst.0,
                copies[j_b].width,
            ) {
                return false;
            }
        }
    }
    true
}

/// For every pair `(j_a, j_b)` with `j_a < j_b`, `copies[j_a].dst`
/// must not overlap `copies[j_b].src`. Equivalently, "no high-j src
/// is clobbered by a low-j dst" — the property forward-order emit
/// needs.
pub(crate) fn forward_emit_is_safe(copies: &[Copy]) -> bool {
    for j_a in 0..copies.len() {
        for j_b in (j_a + 1)..copies.len() {
            if ranges_overlap(
                copies[j_a].dst.0,
                copies[j_a].width,
                copies[j_b].src.0,
                copies[j_b].width,
            ) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use mono_move_core::align_up_u32;
    use proptest::prelude::*;

    impl Copy {
        /// Test-only constructor that wraps raw `u32` offsets in
        /// `FrameOffset`. Keeps `Copy { ... }` literals out of every test.
        fn from_raw(src: u32, dst: u32, width: u32) -> Self {
            Self {
                src: FrameOffset(src),
                dst: FrameOffset(dst),
                width,
            }
        }
    }

    /// Scratch base for the example tests: aligned and above all test offsets.
    const SCRATCH: u32 = 256;

    fn run(copies: &[Copy]) -> Vec<MicroOp> {
        run_and_measure(copies, SCRATCH).0
    }

    /// Simulates the emitted ops with a unique tag for each frame byte. Checks
    /// that destinations receive their sources' original bytes, other bytes
    /// below `scratch` remain unchanged, and scratch accesses stay within the
    /// reported size. Returns the ops and scratch size.
    fn run_and_measure(copies: &[Copy], scratch: u32) -> (Vec<MicroOp>, u32) {
        let mut ops = Vec::new();
        let scratch_size = emit_parallel_copy(&mut ops, copies, FrameOffset(scratch))
            .expect("emit_parallel_copy failed");
        let scratch_end = scratch + scratch_size;
        let mut frame: Vec<u32> = (0..scratch_end).collect();
        for op in &ops {
            let (src, dst, width) = decode(op);
            assert!(src + width <= scratch_end && dst + width <= scratch_end);
            let bytes = frame[src as usize..(src + width) as usize].to_vec();
            frame[dst as usize..(dst + width) as usize].copy_from_slice(&bytes);
        }
        let mut expected: Vec<u32> = (0..scratch).collect();
        for copy in copies {
            for byte in 0..copy.width {
                expected[(copy.dst.0 + byte) as usize] = copy.src.0 + byte;
            }
        }
        assert_eq!(
            &frame[..scratch as usize],
            &expected[..],
            "copies: {:?}, ops (src, dst, width): {:?}",
            copies,
            ops.iter().map(decode).collect::<Vec<_>>(),
        );
        (ops, scratch_size)
    }

    /// Decode a `Move`/`Move8` into `(src_offset, dst_offset, width)` for
    /// assertions. Panics on any other op.
    fn decode(op: &MicroOp) -> (u32, u32, u32) {
        match op {
            MicroOp::Move8 { dst, src } => (src.0, dst.0, 8),
            MicroOp::Move { dst, src, size } => (src.0, dst.0, *size),
            other => panic!("unexpected micro-op {}", other),
        }
    }

    // ----- reverse_emit_is_safe -------------------------------------------

    #[test]
    fn reverse_emit_empty_is_safe() {
        assert!(reverse_emit_is_safe(&[]));
    }

    #[test]
    fn reverse_emit_forward_chain_is_safe() {
        // copies[1].src overlaps copies[0].dst — j_a=0's src does NOT
        // overlap j_b=1's dst, so reverse-order emit is safe.
        let copies = [Copy::from_raw(0, 8, 8), Copy::from_raw(8, 16, 8)];
        assert!(reverse_emit_is_safe(&copies));
    }

    #[test]
    fn reverse_emit_cycle_is_unsafe() {
        let copies = [Copy::from_raw(0, 8, 8), Copy::from_raw(8, 0, 8)];
        assert!(!reverse_emit_is_safe(&copies));
    }

    #[test]
    fn reverse_emit_backward_chain_is_unsafe() {
        let copies = [Copy::from_raw(8, 0, 8), Copy::from_raw(16, 8, 8)];
        assert!(!reverse_emit_is_safe(&copies));
    }

    #[test]
    fn reverse_emit_varied_widths_disjoint_is_safe() {
        let copies = [Copy::from_raw(0, 32, 16), Copy::from_raw(16, 48, 8)];
        assert!(reverse_emit_is_safe(&copies));
    }

    // ----- forward_emit_is_safe -------------------------------------------

    #[test]
    fn forward_emit_empty_is_safe() {
        assert!(forward_emit_is_safe(&[]));
    }

    #[test]
    fn forward_emit_backward_chain_is_safe() {
        // copies[0].dst (= 0) doesn't overlap copies[1].src (= 16). And
        // copies[1].dst (= 8) doesn't matter for forward-order. So
        // forward emit is safe.
        let copies = [Copy::from_raw(8, 0, 8), Copy::from_raw(16, 8, 8)];
        assert!(forward_emit_is_safe(&copies));
    }

    #[test]
    fn forward_emit_cycle_is_unsafe() {
        // 0 ↔ 8 swap: copies[0].dst (= 8) overlaps copies[1].src (= 8).
        let copies = [Copy::from_raw(0, 8, 8), Copy::from_raw(8, 0, 8)];
        assert!(!forward_emit_is_safe(&copies));
    }

    #[test]
    fn forward_emit_forward_chain_is_unsafe() {
        // copies[0].dst (= 8) overlaps copies[1].src (= 8). Emitting in
        // forward order would clobber copies[1]'s source before it's read.
        // This is the exact Pack/Unpack overlap hazard.
        let copies = [Copy::from_raw(0, 8, 8), Copy::from_raw(8, 16, 8)];
        assert!(!forward_emit_is_safe(&copies));
    }

    #[test]
    fn forward_emit_pack_reorder_is_unsafe() {
        // Pack into [32..48) from previous call's rets at [32..48), with
        // fields reordered: field a (offset 0) := src@40, field b
        // (offset 8) := src@32. copies[0].dst = 32 overlaps copies[1].src
        // = 32. This is the corrupted-Pair scenario.
        let copies = [Copy::from_raw(40, 32, 8), Copy::from_raw(32, 40, 8)];
        assert!(!forward_emit_is_safe(&copies));
    }

    #[test]
    fn forward_emit_identity_pack_is_safe() {
        // Natural-order Pack: each field's src happens to equal its
        // dst (the call ABI laid them out at the same offsets), so
        // there are no overlapping rectangles between copies at all.
        let copies = [Copy::from_raw(32, 32, 8), Copy::from_raw(40, 40, 8)];
        assert!(forward_emit_is_safe(&copies));
    }

    // ----- emit_parallel_copy ---------------------------------------------

    #[test]
    fn empty_emits_nothing() {
        assert!(run(&[]).is_empty());
    }

    #[test]
    fn identity_is_elided() {
        let ops = run(&[Copy::from_raw(8, 8, 8)]);
        assert!(ops.is_empty());
    }

    #[test]
    fn zero_width_is_filtered() {
        let ops = run(&[Copy::from_raw(0, 8, 0)]);
        assert!(ops.is_empty());
    }

    #[test]
    fn single_copy_emits_one_move() {
        let ops = run(&[Copy::from_raw(0, 8, 8)]);
        assert_eq!(ops.len(), 1);
        assert_eq!(decode(&ops[0]), (0, 8, 8));
    }

    #[test]
    fn disjoint_copies_emit_in_topo_order() {
        // Independent copies can run in either order without scratch.
        let (ops, scratch_size) = run_and_measure(
            &[Copy::from_raw(0, 16, 8), Copy::from_raw(8, 24, 8)],
            SCRATCH,
        );
        assert_eq!(ops.len(), 2);
        assert_eq!(scratch_size, 0);
    }

    #[test]
    fn forward_chain_emits_dependent_first() {
        // C0: 0 -> 8, C1: 8 -> 16. C0.dst overlaps C1.src, so C1 must
        // emit before C0. Result: [C1, C0].
        let ops = run(&[Copy::from_raw(0, 8, 8), Copy::from_raw(8, 16, 8)]);
        assert_eq!(ops.len(), 2);
        assert_eq!(decode(&ops[0]), (8, 16, 8));
        assert_eq!(decode(&ops[1]), (0, 8, 8));
    }

    #[test]
    fn two_cycle_via_scratch() {
        // Swap: 0 ↔ 8. Saving the first source gives this sequence:
        // scratch ← [0]; [0] ← [8]; [8] ← scratch.
        let ops = run(&[Copy::from_raw(0, 8, 8), Copy::from_raw(8, 0, 8)]);
        assert_eq!(ops.len(), 3);
        let (s0, d0, w0) = decode(&ops[0]);
        // First op saves one cycle member's source to scratch.
        assert_eq!(d0, SCRATCH);
        assert_eq!(w0, 8);
        let saved_src = s0;
        // Last op must restore from scratch.
        let (s2, _, w2) = decode(&ops[2]);
        assert_eq!(s2, SCRATCH);
        assert_eq!(w2, 8);
        // Middle op is the OTHER cycle member, which writes to the
        // saved-source location.
        let (_, d1, w1) = decode(&ops[1]);
        assert_eq!(d1, saved_src);
        assert_eq!(w1, 8);
    }

    #[test]
    fn three_cycle_via_scratch() {
        // 0 -> 8, 8 -> 16, 16 -> 0. Expect 4 ops: 1 save + 3 cycle moves.
        let ops = run(&[
            Copy::from_raw(0, 8, 8),
            Copy::from_raw(8, 16, 8),
            Copy::from_raw(16, 0, 8),
        ]);
        assert_eq!(ops.len(), 4);
        let (_, d0, _) = decode(&ops[0]);
        assert_eq!(d0, SCRATCH, "first op must be the scratch save");
        let (s_last, _, _) = decode(&ops[3]);
        assert_eq!(s_last, SCRATCH, "last op must be the scratch restore");
    }

    #[test]
    fn two_independent_cycles_share_scratch() {
        // Swaps {0 ↔ 8} and {16 ↔ 24} each need three moves and reuse the
        // same eight scratch bytes.
        let (ops, scratch_size) = run_and_measure(
            &[
                Copy::from_raw(0, 8, 8),
                Copy::from_raw(8, 0, 8),
                Copy::from_raw(16, 24, 8),
                Copy::from_raw(24, 16, 8),
            ],
            SCRATCH,
        );
        assert_eq!(ops.len(), 6);
        assert_eq!(scratch_size, 8);
    }

    #[test]
    fn fat_pointer_cycle_uses_full_width_scratch() {
        // 16-byte fat-ref swap. Scratch must hold 16 bytes; algorithm
        // emits Move (not Move8) ops at width 16.
        let ops = run(&[Copy::from_raw(0, 16, 16), Copy::from_raw(16, 0, 16)]);
        assert_eq!(ops.len(), 3);
        for op in &ops {
            let (_, _, w) = decode(op);
            assert_eq!(w, 16);
        }
    }

    #[test]
    fn cycle_plus_acyclic_dependent_emits_dependent_after_cycle() {
        // The swap {0 ↔ 8} needs one save; the independent copy (32 -> 40)
        // needs none, giving four moves total.
        let (ops, scratch_size) = run_and_measure(
            &[
                Copy::from_raw(0, 8, 8),
                Copy::from_raw(8, 0, 8),
                Copy::from_raw(32, 40, 8),
            ],
            SCRATCH,
        );
        assert_eq!(ops.len(), 4);
        assert_eq!(scratch_size, 8);
    }

    #[test]
    fn cycle_break_saves_a_cycle_member() {
        // A = (0 -> 16, 8) and B = (16 -> 0, 4) form a cycle. X = (40 -> 4, 4)
        // waits on A and blocks Y = (48 -> 40, 4), but belongs to no cycle.
        // Saving A breaks the cycle; saving X would add an unnecessary move.
        let (ops, scratch_size) = run_and_measure(
            &[
                Copy::from_raw(40, 4, 4),
                Copy::from_raw(48, 40, 4),
                Copy::from_raw(0, 16, 8),
                Copy::from_raw(16, 0, 4),
            ],
            SCRATCH,
        );
        assert_eq!(ops.len(), 5);
        assert_eq!(decode(&ops[0]), (0, SCRATCH, 8));
        assert_eq!(scratch_size, 8);
    }

    #[test]
    fn pending_save_keeps_its_scratch_bytes() {
        // Saving C = (16 -> 0, 8) unblocks P = (0 -> 16, 4), but C still
        // waits on Q = (4 -> 40, 8), which is in a cycle with R = (40 -> 8, 4).
        // Q's save must not overwrite C's saved bytes.
        let (ops, scratch_size) = run_and_measure(
            &[
                Copy::from_raw(16, 0, 8),
                Copy::from_raw(0, 16, 4),
                Copy::from_raw(4, 40, 8),
                Copy::from_raw(40, 8, 4),
            ],
            SCRATCH,
        );
        assert_eq!(ops.len(), 6);
        assert_eq!(scratch_size, 16);
    }

    /// Generates copies with disjoint destinations. Sources can overlap
    /// destinations or each other at arbitrary offsets within the same region.
    fn arb_copies() -> impl Strategy<Value = Vec<Copy>> {
        let width = prop::sample::select(vec![1u32, 2, 3, 4, 8, 16]);
        prop::collection::vec((width, 0u32..3), 2..8).prop_flat_map(|slots| {
            let mut dsts = Vec::with_capacity(slots.len());
            let mut end = 0;
            for (width, gap) in slots {
                dsts.push((end + gap, width));
                end += gap + width;
            }
            let srcs: Vec<_> = dsts.iter().map(|&(_, width)| 0..=end - width).collect();
            (Just(dsts), srcs)
                .prop_map(|(dsts, srcs)| {
                    dsts.into_iter()
                        .zip(srcs)
                        .map(|((dst, width), src)| Copy::from_raw(src, dst, width))
                        .collect::<Vec<_>>()
                })
                .prop_shuffle()
        })
    }

    proptest! {
        #[test]
        fn mixed_width_copies_are_parallel(
            copies in arb_copies(),
            misalignment in 0u32..MAX_ALIGN as u32,
        ) {
            // Lowering may pass an unaligned scratch base, requiring padding.
            let scratch = SCRATCH + misalignment;
            let (ops, scratch_size) = run_and_measure(&copies, scratch);
            prop_assert!(ops.len() <= 2 * copies.len());
            let padding = align_up_u32(scratch, MAX_ALIGN as u32) - scratch;
            let max_scratch_size: u32 = padding
                + copies
                    .iter()
                    .map(|copy| align_up_u32(copy.width, MAX_ALIGN as u32))
                    .sum::<u32>();
            prop_assert!(scratch_size <= max_scratch_size);
        }
    }
}
