// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Lemmas and proof blocks: every proof statement, a recursive lemma with a
/// declared measure, and a generic lemma whose type argument an application
/// determines.
module 0x42::lemmas {
    spec fun sum(n: num): num {
        if (n <= 0) { 0 } else { n + sum(n - 1) }
    }

    spec module {
        lemma monotonicity(x: num, y: num) {
            requires 0 <= x;
            requires x <= y;
            ensures sum(x) <= sum(y);
        } proof {
            if (x < y) {
                assert sum(y - 1) <= sum(y);
                apply monotonicity(x, y - 1);
            }
        }

        lemma bounded(x: num) {
            requires x >= 0;
            ensures sum(x) >= 0;
            decreases x;
        } proof {
            if (x > 0) {
                apply bounded(x - 1);
            } else {
                assume [trusted] true;
            }
        }

        lemma length_nonneg<T>(v: vector<T>) {
            ensures len(v) >= 0;
        }
    }

    fun sum_up_to(n: u64): u64 {
        if (n == 0) { 0 } else { n + sum_up_to(n - 1) }
    }
    spec sum_up_to {
        aborts_if sum(n) > MAX_U64;
        ensures result == sum(n);
    } proof {
        forall x: num, y: num {sum(x), sum(y)} [weight = 2] apply monotonicity(x, y);
    }

    fun first<T: copy>(v: &vector<T>): u64 {
        std::vector::length(v)
    }
    spec first {
        ensures result == len(v);
    } proof {
        let n = len(v);
        apply length_nonneg(v);
        calc(0 <= n == len(v));
        split n > 0;
        post assert result == n;
    }
}
