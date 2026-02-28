// exclude_for: cvc5
// Tests bv classification convergence for mutually recursive functions (SCC of size 2).
// In NumberOperationProcessor::analyze, sort_in_reverse_topological_order returns
// mutually recursive functions as Either::Right(scc). The outer fixed-point loop
// must iterate until both functions' parameter types converge.
//
// bv_mask_a has pragma bv on its first parameter; bv must propagate to bv_mask_b's
// first parameter through the call edge bv_mask_a -> bv_mask_b, which only becomes
// visible after at least one SCC iteration.
module 0x42::BvMutualRecursion {

    fun bv_mask_a(x: u64, flag: bool): u64 {
        if (flag) {
            x & 0xF0F0F0F0F0F0F0F0
        } else {
            bv_mask_b(x, true)
        }
    }

    fun bv_mask_b(x: u64, flag: bool): u64 {
        if (flag) {
            x & 0x0F0F0F0F0F0F0F0F
        } else {
            bv_mask_a(x, true)
        }
    }

    spec bv_mask_a {
        pragma bv = b"0";
        pragma opaque;
        aborts_if false;
        ensures result == (x & (0xF0F0F0F0F0F0F0F0 as u64));
    }

    spec bv_mask_b {
        // No explicit bv pragma: the bv classification of the first parameter
        // must be inferred through fixed-point iteration over the SCC.
        pragma opaque;
        aborts_if false;
        ensures result == (x & (0x0F0F0F0F0F0F0F0F as u64));
    }

    // A caller that exercises both functions via the opaque specs.
    fun split_nibbles(x: u64): (u64, u64) {
        (bv_mask_a(x, true), bv_mask_b(x, true))
    }
    spec split_nibbles {
        pragma bv = b"0";
        ensures result_1 == (x & (0xF0F0F0F0F0F0F0F0 as u64));
        ensures result_2 == (x & (0x0F0F0F0F0F0F0F0F as u64));
    }
}
