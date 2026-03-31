// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Pipeline tests for lemma declarations and apply/forall-apply.
module 0x42::TestLemma {

    // ==========================================================================
    // Non-generic trusted lemma with requires, simple apply.

    spec lemma mul_bound(x: u64, y: u64) {
        requires x <= 100;
        requires y <= 100;
        ensures x * y <= 10000;
    } proof {
        assume [trusted] true;
    }

    fun small_product(x: u64, y: u64): u64 {
        x * y
    }
    spec small_product {
        requires x <= 100;
        requires y <= 100;
        ensures result <= 10000;
    } proof {
        apply mul_bound(x, y);
    }

    // ==========================================================================
    // Non-generic lemma verified by VC, simple apply.

    spec lemma add_le_sum(a: u64, b: u64) {
        requires a <= b;
        ensures a + b <= 2 * b;
    }

    fun double_max(a: u64, b: u64): u64 {
        a + b
    }
    spec double_max {
        requires a <= b;
        requires 2 * b <= MAX_U64;
        ensures result <= 2 * b;
    } proof {
        apply add_le_sum(a, b);
    }

    // ==========================================================================
    // Generic lemma with requires, simple apply.

    spec lemma len_pos_means_nonempty<T>(v: vector<T>) {
        requires len(v) > 0;
        ensures len(v) >= 1;
    } proof {
        assume [trusted] true;
    }

    fun first_exists<T>(v: &vector<T>): bool {
        spec {
            assume len(v) > 0;
        };
        true
    }
    spec first_exists {
        requires len(v) > 0;
        ensures result == true;
    } proof {
        apply len_pos_means_nonempty(v);
    }

    // ==========================================================================
    // Non-generic lemma, forall...apply with triggers.

    spec fun sum(n: num): num {
        if (n == 0) { 0 } else { n + sum(n - 1) }
    }

    spec lemma sum_nonneg(n: num) {
        requires n >= 0;
        ensures sum(n) >= 0;
    } proof {
        assume [trusted] true;
    }

    fun sum_to(n: u64): u64 {
        if (n == 0) { 0 }
        else { n + sum_to(n - 1) }
    }
    spec sum_to {
        aborts_if sum(n) > MAX_U64;
        ensures result == sum(n);
    } proof {
        forall x: num {sum(x)} apply sum_nonneg(x);
    }

    // ==========================================================================
    // Generic lemma, forall...apply.

    spec fun all_positive<T>(v: vector<T>, len_v: num): bool {
        len_v >= 0
    }

    spec lemma positive_len_bound<T>(v: vector<T>) {
        requires len(v) > 0;
        ensures all_positive(v, len(v));
    } proof {
        assume [trusted] true;
    }

    fun has_elements<T>(v: &vector<T>): bool {
        spec {
            assume len(v) > 0;
        };
        true
    }
    spec has_elements {
        requires len(v) > 0;
        ensures result == true;
    } proof {
        forall w: vector<T> {all_positive(w, len(w))} apply positive_len_bound(w);
    }

    // ==========================================================================
    // Forward reference: function proof applies a lemma defined later.

    fun add_bounded(x: u64, y: u64): u64 {
        x + y
    }
    spec add_bounded {
        requires x <= 50;
        requires y <= 50;
        ensures result <= 100;
    } proof {
        apply add_bound_lemma(x, y);
    }

    spec lemma add_bound_lemma(x: u64, y: u64) {
        requires x <= 50;
        requires y <= 50;
        ensures x + y <= 100;
    } proof {
        assume [trusted] true;
    }

    // ==========================================================================
    // Mutual recursion: A -> B -> C -> A.

    spec fun fib(n: num): num {
        if (n <= 1) { n } else { fib(n - 1) + fib(n - 2) }
    }

    spec lemma fib_nonneg_a(n: num) {
        requires n >= 0;
        ensures fib(n) >= 0;
    } proof {
        if (n > 1) {
            apply fib_nonneg_b(n - 1);
        }
    }

    spec lemma fib_nonneg_b(n: num) {
        requires n >= 0;
        ensures fib(n) >= 0;
    } proof {
        if (n > 1) {
            apply fib_nonneg_c(n - 1);
        }
    }

    spec lemma fib_nonneg_c(n: num) {
        requires n >= 0;
        ensures fib(n) >= 0;
    } proof {
        if (n > 1) {
            apply fib_nonneg_a(n - 1);
        }
    }
}
