// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Confirms a closure-BP single-state `S.. |~` lowering substitutes the bound
// state variable at the `&mut` input slot. RequiresOf/AbortsOf only carry a
// pre-state label, so substitution must reach the input through `pre_sub`
// (not the EnsuresOf trailing-slot path).

module 0x42::closure_bp_post_sub_pre_only {

    struct Acc has copy, drop { value: u64 }

    fun apply_twice(f: |&mut Acc| has drop + copy, x: &mut Acc) {
        f(x);
        f(x)
    }
    spec apply_twice {
        ensures exists S in *:
            (S.. |~ requires_of<f>(x)) &&
            (S.. |~ !aborts_of<f>(x));
    }
}
