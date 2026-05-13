// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Tests `&mut x.foo` selector arguments for behavioral predicates.
// The Boogie translator's spec rewriter peels the outer `Borrow(Mutable)`
// when the expression appears in a behavioral-predicate argument position,
// so the selector resolves to the field's pre/post value as needed.

module 0x42::bp_mut_ref_selector {
    struct Point has copy, drop {
        x: u64,
        y: u64,
    }

    // Apply `f` to a field of `p` via a `&mut` field borrow.
    fun apply_to_x(f: |&mut u64|, p: &mut Point) { f(&mut p.x) }
    spec apply_to_x {
        // The post-state of `p.x` equals f's post-state of its &mut argument
        // applied to the pre-state of `p.x`.
        ensures p.x == result_of<f>(&mut p.x);
        // p.y is unchanged by the call.
        ensures p.y == old(p).y;
    }

    // Combined with explicit returns.
    fun apply_to_x_returning(f: |&mut u64| u64, p: &mut Point): u64 { f(&mut p.x) }
    spec apply_to_x_returning {
        ensures ensures_of<f>(&mut p.x, result);
    }
}
