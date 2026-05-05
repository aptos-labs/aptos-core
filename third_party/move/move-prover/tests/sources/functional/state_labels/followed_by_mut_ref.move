// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Tests sequential composition of closures over `&mut T` parameters via
// `exists S in *` state labels.  This exercises the Boogie codegen fix for
// issue #19623: when `exists S in *` is combined with `..S |~ ensures_of<f>`
// for a `|&mut T|` closure (no global memory), the encoder must emit
// `S_val: T` as the bound variable rather than `exists  ::` (empty).
//
// The `followed_by` pattern:
//   ensures exists S in *:
//       (..S |~ ensures_of<f>(old(x), x)) &&
//       (S.. |~ ensures_of<g>(old(x), x));
// witnesses the intermediate value S of x such that f transforms
// old(x) → S and g transforms S → x (the final value).

module 0x42::followed_by_mut_ref {

    struct Acc has copy, drop { value: u64 }

    /// Apply `f` then `g` to `x`, witnessing the intermediate value via
    /// an existential state label.
    fun followed_by(
        f: |&mut Acc| has drop,
        g: |&mut Acc| has drop,
        x: &mut Acc,
    ) {
        f(x);
        g(x)
    }
    spec followed_by {
        ensures exists S in *:
            (..S |~ ensures_of<f>(old(x), x)) &&
            (S.. |~ ensures_of<g>(old(x), x));
    }
}
