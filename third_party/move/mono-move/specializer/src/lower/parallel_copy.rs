// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Parallel-copy primitives.
//!
//! - [`Copy`] — uniform `(src, dst, width)` representation of a byte-copy.
//! - [`emit_parallel_copy`] — cycle-aware emit. Iteratively picks any
//!   pending copy whose dst overlaps no other pending copy's src and emits
//!   it; when only cyclic copies remain, breaks one cycle by saving a
//!   chosen member's source to a caller-provided scratch slot. Used by
//!   `Instr::Ret` lowering, where `return_slots` overlap the home region
//!   so a function like `swap(a, b) -> (b, a)` produces a real swap-cycle.
//! - [`reverse_emit_is_safe`] — debug check that a list of copies forms a
//!   forward-only dependency graph, exactly the property that makes
//!   reverse-order emission sound.
//!
//! `lower_call`'s arg-setup path inlines reverse-order emission directly
//! and uses [`reverse_emit_is_safe`] as a debug assertion. Soundness
//! rests on arg positionality + return monotonicity (see
//! `BlockAnalysis::analyze`): pass-through Xfer args land at
//! `arg_offset(j) ≥ ret_offset(k_j)` everywhere, so the dependency graph
//! is forward-only. Home args' sources live in the home region (offsets
//! `< frame_data_size`), disjoint from the arg region. No cycles, so no
//! scratch slot needed for arg setup.

use mono_move_core::{FrameOffset, MicroOp};
use smallbitvec::SmallBitVec;
use smallvec::SmallVec;

/// One byte-copy: `width` bytes from `src` to `dst`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Copy {
    pub src: u32,
    pub dst: u32,
    pub width: u32,
}

/// Emit `MicroOp::Move`/`Move8` ops effecting all `copies` in parallel: each
/// emitted sequence behaves as if every copy's read of its source happened
/// simultaneously, before any write that could clobber that source.
///
/// Algorithm: Kahn's online topological sort with cycle-break-via-scratch.
/// Edge `Y → X` when `X.dst` overlaps `Y.src`; a "safe" copy has empty
/// `blockers[i]` and emits, clearing its bit from every other blocker set.
/// On a cycle, save one chosen copy's src to `scratch` and rewrite its src
/// to `scratch`; that copy emits later, reading the saved bytes. Each cycle
/// resolves before the next reuses `scratch`, so one slot handles
/// arbitrarily many independent cycles. O(N²) with `SmallBitVec` blockers
/// and a stack-resident `SmallVec` for the small-N path.
///
/// Trivial copies (`width == 0` or `src == dst`) are filtered up front.
///
/// `scratch` must be a frame offset of a slot wide enough to hold the largest
/// copy width that could appear in a cycle; the caller is responsible for
/// reserving it (`LoweringContext::scratch_offset`). When `copies` is acyclic,
/// `scratch` is unused.
pub(crate) fn emit_parallel_copy(out: &mut Vec<MicroOp>, copies: &[Copy], scratch: u32) {
    // Filter trivial copies. Inline-stack-allocated for the common
    // small-N path; spills to heap only for adversarial wide signatures.
    let mut copies: SmallVec<[Copy; 4]> = copies
        .iter()
        .copied()
        .filter(|c| c.width > 0 && c.src != c.dst)
        .collect();
    let n = copies.len();
    if n == 0 {
        return;
    }
    if n == 1 {
        emit_one(out, copies[0]);
        return;
    }

    // `blockers[i]` is the bitset of indices `j` such that `copies[j].src`
    // overlaps `copies[i].dst` — the copies that must emit before `i`.
    // When we emit (or unblock via cycle break) copy `e`, we clear bit
    // `e` from every other `blockers[k]`. `alive` tracks which copies
    // remain pending; safety check is `alive[i] && blockers[i].all_false()`.
    let mut blockers: SmallVec<[SmallBitVec; 4]> = (0..n)
        .map(|i| {
            let mut bv = SmallBitVec::from_elem(n, false);
            for j in 0..n {
                if i != j
                    && ranges_overlap(
                        copies[i].dst,
                        copies[i].width,
                        copies[j].src,
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

    while remaining > 0 {
        let safe =
            (0..n).find(|&i| alive.get(i).expect("alive sized to n") && blockers[i].all_false());

        if let Some(i) = safe {
            emit_one(out, copies[i]);
            mark_emitted(i, n, &mut alive, &mut blockers);
            remaining -= 1;
        } else {
            // Cycle: route the first alive copy's source through
            // `scratch`. After the rewrite, the chosen copy's src no
            // longer overlaps any other copy's dst (scratch is
            // reserved), so it stops blocking anyone — same bookkeeping
            // as a normal emit, but the chosen copy stays pending
            // until its own dst becomes safe.
            let chosen = (0..n)
                .find(|&i| alive.get(i).expect("alive sized to n"))
                .expect("remaining > 0 implies at least one alive copy");
            emit_one(out, Copy {
                src: copies[chosen].src,
                dst: scratch,
                width: copies[chosen].width,
            });
            copies[chosen].src = scratch;
            clear_blocker(chosen, n, &alive, &mut blockers);
        }
    }
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
    for k in 0..n {
        if k != i && alive.get(k).expect("alive sized to n") {
            blockers[k].set(i, false);
        }
    }
}

#[inline]
fn emit_one(out: &mut Vec<MicroOp>, c: Copy) {
    if c.width == 8 {
        out.push(MicroOp::Move8 {
            dst: FrameOffset(c.dst),
            src: FrameOffset(c.src),
        });
    } else {
        out.push(MicroOp::Move {
            dst: FrameOffset(c.dst),
            src: FrameOffset(c.src),
            size: c.width,
        });
    }
}

#[inline]
fn ranges_overlap(a_off: u32, a_w: u32, b_off: u32, b_w: u32) -> bool {
    a_off < b_off + b_w && b_off < a_off + a_w
}

/// Debug-only check: for every pair `(j_a, j_b)` with `j_a < j_b`,
/// `copies[j_a].src` must not overlap `copies[j_b].dst`. Equivalently,
/// "no low-j src is clobbered by a high-j dst" — exactly the property
/// reverse-order emit needs.
#[cfg(any(test, debug_assertions))]
pub(crate) fn reverse_emit_is_safe(copies: &[Copy]) -> bool {
    for j_a in 0..copies.len() {
        for j_b in (j_a + 1)..copies.len() {
            if ranges_overlap(
                copies[j_a].src,
                copies[j_a].width,
                copies[j_b].dst,
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

    fn run(copies: &[Copy], scratch: u32) -> Vec<MicroOp> {
        let mut out = Vec::new();
        emit_parallel_copy(&mut out, copies, scratch);
        out
    }

    /// Decode a `Move`/`Move8` into `(src_offset, dst_offset, width)` for
    /// assertions. Panics on any other op.
    fn decode(op: &MicroOp) -> (u32, u32, u32) {
        match op {
            MicroOp::Move8 { dst, src } => (src.0, dst.0, 8),
            MicroOp::Move { dst, src, size } => (src.0, dst.0, *size),
            other => panic!("unexpected micro-op {:?}", other),
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
        let copies = [
            Copy {
                src: 0,
                dst: 8,
                width: 8,
            },
            Copy {
                src: 8,
                dst: 16,
                width: 8,
            },
        ];
        assert!(reverse_emit_is_safe(&copies));
    }

    #[test]
    fn reverse_emit_cycle_is_unsafe() {
        let copies = [
            Copy {
                src: 0,
                dst: 8,
                width: 8,
            },
            Copy {
                src: 8,
                dst: 0,
                width: 8,
            },
        ];
        assert!(!reverse_emit_is_safe(&copies));
    }

    #[test]
    fn reverse_emit_backward_chain_is_unsafe() {
        let copies = [
            Copy {
                src: 8,
                dst: 0,
                width: 8,
            },
            Copy {
                src: 16,
                dst: 8,
                width: 8,
            },
        ];
        assert!(!reverse_emit_is_safe(&copies));
    }

    #[test]
    fn reverse_emit_varied_widths_disjoint_is_safe() {
        let copies = [
            Copy {
                src: 0,
                dst: 32,
                width: 16,
            },
            Copy {
                src: 16,
                dst: 48,
                width: 8,
            },
        ];
        assert!(reverse_emit_is_safe(&copies));
    }

    // ----- emit_parallel_copy ---------------------------------------------

    #[test]
    fn empty_emits_nothing() {
        assert!(run(&[], 100).is_empty());
    }

    #[test]
    fn identity_is_elided() {
        let ops = run(
            &[Copy {
                src: 8,
                dst: 8,
                width: 8,
            }],
            100,
        );
        assert!(ops.is_empty());
    }

    #[test]
    fn zero_width_is_filtered() {
        let ops = run(
            &[Copy {
                src: 0,
                dst: 8,
                width: 0,
            }],
            100,
        );
        assert!(ops.is_empty());
    }

    #[test]
    fn single_copy_emits_one_move() {
        let ops = run(
            &[Copy {
                src: 0,
                dst: 8,
                width: 8,
            }],
            100,
        );
        assert_eq!(ops.len(), 1);
        assert_eq!(decode(&ops[0]), (0, 8, 8));
    }

    #[test]
    fn disjoint_copies_emit_in_topo_order() {
        // Two independent copies; either order is correct, but neither
        // should require the scratch slot.
        let ops = run(
            &[
                Copy {
                    src: 0,
                    dst: 16,
                    width: 8,
                },
                Copy {
                    src: 8,
                    dst: 24,
                    width: 8,
                },
            ],
            100,
        );
        assert_eq!(ops.len(), 2);
        for op in &ops {
            let (_, dst, _) = decode(op);
            assert!(dst != 100, "scratch should be unused for disjoint copies");
        }
    }

    #[test]
    fn forward_chain_emits_dependent_first() {
        // C0: 0 -> 8, C1: 8 -> 16. C0.dst overlaps C1.src, so C1 must
        // emit before C0. Result: [C1, C0].
        let ops = run(
            &[
                Copy {
                    src: 0,
                    dst: 8,
                    width: 8,
                },
                Copy {
                    src: 8,
                    dst: 16,
                    width: 8,
                },
            ],
            100,
        );
        assert_eq!(ops.len(), 2);
        assert_eq!(decode(&ops[0]), (8, 16, 8));
        assert_eq!(decode(&ops[1]), (0, 8, 8));
    }

    #[test]
    fn two_cycle_via_scratch() {
        // Swap: 0 ↔ 8. Expected: scratch ← [0]; [8] ← [0]; [0] ← scratch.
        // Or equivalently: scratch ← [8]; [0] ← [8]; [8] ← scratch.
        // (Either choice — the algorithm picks `pending[0]`.)
        let ops = run(
            &[
                Copy {
                    src: 0,
                    dst: 8,
                    width: 8,
                },
                Copy {
                    src: 8,
                    dst: 0,
                    width: 8,
                },
            ],
            100,
        );
        assert_eq!(ops.len(), 3);
        let (s0, d0, w0) = decode(&ops[0]);
        // First op saves one cycle member's source to scratch.
        assert_eq!(d0, 100);
        assert_eq!(w0, 8);
        let saved_src = s0;
        // Last op must restore from scratch.
        let (s2, _, w2) = decode(&ops[2]);
        assert_eq!(s2, 100);
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
        let ops = run(
            &[
                Copy {
                    src: 0,
                    dst: 8,
                    width: 8,
                },
                Copy {
                    src: 8,
                    dst: 16,
                    width: 8,
                },
                Copy {
                    src: 16,
                    dst: 0,
                    width: 8,
                },
            ],
            100,
        );
        assert_eq!(ops.len(), 4);
        let (_, d0, _) = decode(&ops[0]);
        assert_eq!(d0, 100, "first op must be the scratch save");
        let (s_last, _, _) = decode(&ops[3]);
        assert_eq!(s_last, 100, "last op must be the scratch restore");
    }

    #[test]
    fn two_independent_cycles_share_scratch() {
        // {0 ↔ 8} and {16 ↔ 24}. Each is a 2-cycle; total 6 ops with
        // scratch reused.
        let ops = run(
            &[
                Copy {
                    src: 0,
                    dst: 8,
                    width: 8,
                },
                Copy {
                    src: 8,
                    dst: 0,
                    width: 8,
                },
                Copy {
                    src: 16,
                    dst: 24,
                    width: 8,
                },
                Copy {
                    src: 24,
                    dst: 16,
                    width: 8,
                },
            ],
            100,
        );
        assert_eq!(ops.len(), 6);
        // Scratch is written exactly twice (once per cycle break) and
        // read exactly twice (once per cycle restore).
        let scratch_writes = ops
            .iter()
            .filter(|op| {
                let (_, d, _) = decode(op);
                d == 100
            })
            .count();
        let scratch_reads = ops
            .iter()
            .filter(|op| {
                let (s, _, _) = decode(op);
                s == 100
            })
            .count();
        assert_eq!(scratch_writes, 2);
        assert_eq!(scratch_reads, 2);
    }

    #[test]
    fn fat_pointer_cycle_uses_full_width_scratch() {
        // 16-byte fat-ref swap. Scratch must hold 16 bytes; algorithm
        // emits Move (not Move8) ops at width 16.
        let ops = run(
            &[
                Copy {
                    src: 0,
                    dst: 16,
                    width: 16,
                },
                Copy {
                    src: 16,
                    dst: 0,
                    width: 16,
                },
            ],
            100,
        );
        assert_eq!(ops.len(), 3);
        for op in &ops {
            let (_, _, w) = decode(op);
            assert_eq!(w, 16);
        }
    }

    #[test]
    fn cycle_plus_acyclic_dependent_emits_dependent_after_cycle() {
        // Cycle {0 ↔ 8}, plus C2 = (32 -> 40) — independent. C2 should
        // emit at some point, scratch unused for C2.
        let ops = run(
            &[
                Copy {
                    src: 0,
                    dst: 8,
                    width: 8,
                },
                Copy {
                    src: 8,
                    dst: 0,
                    width: 8,
                },
                Copy {
                    src: 32,
                    dst: 40,
                    width: 8,
                },
            ],
            100,
        );
        assert_eq!(ops.len(), 4); // 1 save + 2 cycle moves + 1 standalone
                                  // Exactly one Move into scratch and one Move out.
        let scratch_writes = ops
            .iter()
            .filter(|op| {
                let (_, d, _) = decode(op);
                d == 100
            })
            .count();
        assert_eq!(scratch_writes, 1);
    }
}
