// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Parallel-copy building blocks.
//!
//! Reusable pieces for emitting byte-copies into the call ABI:
//!
//! - [`Copy`] — uniform `(src, dst, width)` representation of a byte-copy.
//! - [`reverse_emit_is_safe`] — debug check that a list of copies forms a
//!   forward-only dependency graph (no low-j src overlapping a high-j dst),
//!   exactly the property that makes reverse-order emission
//!   sound.
//!
//! The current `lower_call` arg-setup path inlines reverse-order emission
//! directly and uses [`reverse_emit_is_safe`] as a debug assertion.
//! Soundness rests on arg positionality + return monotonicity (see
//! `BlockAnalysis::analyze`): pass-through Xfer args land at
//! `arg_offset(j) ≥ ret_offset(k_j)` everywhere; the dependency graph is
//! forward-only, so reverse iteration resolves all edges with no scratch
//! slot and no cycle handling. Home args' sources live in the home region
//! (offsets < `frame_data_size`), disjoint from the arg region — they
//! don't enter the dependency graph at all.
//!
//! [`emit_parallel_copy`] is a reserved slot for cycle-aware emission
//! (e.g., `Ret` swap-cycles per the Codex review). It is currently
//! unimplemented; when a caller needs cycle handling, the implementation
//! lands here on top of the [`Copy`] representation and overlap helper.

use mono_move_core::MicroOp;

/// One byte-copy: `width` bytes from `src` to `dst`. Used by the
/// `lower_call` reverse-order-emit debug check and reserved for the future
/// cycle-aware emission. Fields are dead in release builds with no
/// debug assertions enabled.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct Copy {
    pub src: u32,
    pub dst: u32,
    pub width: u32,
}

/// Cycle-aware parallel-copy emission. Currently unimplemented — the
/// arg-setup reverse-order emit is inlined at `lower_call`. Real cycle
/// handling (scratch-slot routing) is needed for `Ret` swap-cycles per
/// the Codex review; that implementation lands here when written.
#[allow(dead_code)]
pub(crate) fn emit_parallel_copy(_out: &mut Vec<MicroOp>, _copies: &[Copy]) {
    unimplemented!(
        "cycle-aware parallel copy not yet implemented; arg-setup reverse-order emit \
         is inlined at `lower_call`. This slot is reserved for `Ret` swap-cycle handling."
    );
}

/// Debug-only check: for every pair `(j_a, j_b)` with `j_a < j_b`,
/// `copies[j_a].src` must not overlap `copies[j_b].dst`. Equivalently,
/// "no low-j src is clobbered by a high-j dst" — exactly the property
/// reverse-order emit needs.
#[cfg(any(test, debug_assertions))]
pub(crate) fn reverse_emit_is_safe(copies: &[Copy]) -> bool {
    fn ranges_overlap(a_off: u32, a_w: u32, b_off: u32, b_w: u32) -> bool {
        a_off < b_off + b_w && b_off < a_off + a_w
    }
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

    #[test]
    fn empty_is_safe() {
        assert!(reverse_emit_is_safe(&[]));
    }

    #[test]
    fn forward_chain_is_safe() {
        // copies[0]: src=0, dst=8 (j=0).
        // copies[1]: src=8, dst=16 (j=1).
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
    fn cycle_is_unsafe() {
        // Swap is the canonical cycle. Under arg positionality + return
        // monotonicity this shape is unreachable from `lower_call`'s arg
        // setup; if it ever shows up there, the invariants have been
        // broken upstream.
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
    fn backward_chain_is_unsafe() {
        // copies[0].src=8, copies[1].dst=8 → overlap j_a=0's src with
        // j_b=1's dst. Decreasing-j would clobber j=0's source before
        // reading it.
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
    fn varied_widths_disjoint_is_safe() {
        // 16-byte fat-pointer alongside an 8-byte scalar at disjoint
        // offsets.
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
}
