// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// Negative test demonstrating the failure mode when `[weight = N]` is set
// too high. The defining axiom for a recursive `spec fun` carries `:weight N`;
// once that weight reaches Z3's `qi.eager_threshold` (the prover's default is
// 100), the very first eager instantiation exhausts the cost budget and the
// quantifier is effectively disabled. A proof that needs even one unfolding
// of the spec function then fails.
//
// This is the user-error footgun documented under the planned
// "warn at weight >= 100" diagnostic — the test pins the current behaviour
// (post-condition failure) so we'll notice if/when that warning lands.

module 0x42::proof_weight_too_large {

    // weight = 1000 is far above the eager_threshold (100). Z3 defers this
    // axiom immediately, so the equation `id_num(0) == 0` never propagates.
    spec fun id_num(n: num): num [weight = 1000] {
        if (n == 0) { 0 } else { id_num(n - 1) + 1 }
    }

    fun id_zero(): u64 {
        0
    }
    spec id_zero {
        // Even this trivial post-condition can't discharge: the prover
        // never gets to unfold `id_num(0)`.
        ensures result == id_num(0);
    }
}
