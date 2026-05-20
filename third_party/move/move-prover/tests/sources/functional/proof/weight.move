// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// E2E tests for the `[weight = N]` quantifier-instantiation annotation.
//
// Two opt-in sites are exercised:
//   1. `spec fun NAME(...): T [weight = N] { body }`  — attaches `:weight N` to
//      the recursive defining axiom emitted by the Boogie backend.
//   2. `forall x: num {trigger} [weight = N] apply lemma(...);` — attaches
//      `:weight N` to the quantifier synthesized for the lemma instantiation.
//
// Each example below verifies cleanly, demonstrating that the annotation is
// parsed, plumbed through the model, and emitted in Boogie without changing
// proof semantics.

module 0x42::proof_weight {

    // ==================================================================
    // 1. Recursive spec fun with `[weight = N]` on the signature.
    //
    // Without `[weight = N]`, the defining axiom is `:weight 0` (Z3 default),
    // which can matching-loop on symbolic args. With `[weight = 20]`, Z3
    // throttles unrolling once the accumulated cost exceeds the eager
    // threshold. For this small example either works; the annotation just
    // exercises the path.

    spec fun sum(n: num): num [weight = 20] {
        if (n == 0) { 0 } else { n + sum(n - 1) }
    }

    fun sum_zero(): u64 {
        0
    }
    spec sum_zero {
        ensures result == sum(0);
    }

    // ==================================================================
    // 2. Multiple recursive spec funs in the same module, each with a
    //    distinct weight. Demonstrates per-axiom tuning.

    spec fun fact(n: num): num [weight = 30] {
        if (n <= 0) { 1 } else { n * fact(n - 1) }
    }

    fun fact_zero(): u64 {
        1
    }
    spec fact_zero {
        ensures result == fact(0);
    }

    // ==================================================================
    // 3. `forall ... [weight = N] apply ...` proof block.
    //
    // The trigger is a multi-pattern over `sum`. Weight on this quantifier
    // throttles the per-`(x, y)` instantiation of the monotonicity lemma.

    spec module {
        lemma sum_mono(x: num, y: num) {
            requires 0 <= x;
            requires x <= y;
            ensures sum(x) <= sum(y);
        } proof {
            if (x < y) {
                assert sum(y - 1) <= sum(y);
                apply sum_mono(x, y - 1);
            }
        }
    }

    fun sum_up_to(n: u64): u64 {
        if (n == 0) { 0 }
        else { n + sum_up_to(n - 1) }
    }
    spec sum_up_to {
        aborts_if sum(n) > MAX_U64;
        ensures result == sum(n);
    } proof {
        forall x: num, y: num {sum(x), sum(y)} [weight = 5]
            apply sum_mono(x, y);
    }

    // ==================================================================
    // 4. `[weight = 0]` is accepted (redundant — equals the SMT default —
    //    but valid).

    spec fun id_num(n: num): num [weight = 0] {
        if (n == 0) { 0 } else { id_num(n - 1) + 1 }
    }

    fun id_zero(): u64 {
        0
    }
    spec id_zero {
        ensures result == id_num(0);
    }
}
