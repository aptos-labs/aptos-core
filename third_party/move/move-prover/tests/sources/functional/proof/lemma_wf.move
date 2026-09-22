// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// Well-founded recursive lemma application: every application of a lemma
// from the same recursion group must decrease the lemma's measure, declared
// with `decreases` or defaulted to the integer parameters in order.
module 0x42::lemma_wf {

    spec module {
        fun math_pow(base: num, exp: num): num {
            if (exp <= 0) { 1 } else { math_pow(base, exp - 1) * base }
        }

        // Induction on the default measure `(base, exp)`.
        lemma pow_pos(base: num, exp: num) {
            ensures base >= 1 && exp >= 0 ==> math_pow(base, exp) >= 1;
        } proof {
            if (exp > 0) {
                apply pow_pos(base, exp - 1);
            }
        }

        // The same, with the measure declared.
        lemma pow_pos_declared(base: num, exp: num) {
            decreases exp;
            ensures base >= 1 && exp >= 0 ==> math_pow(base, exp) >= 1;
        } proof {
            if (exp > 0) {
                apply pow_pos_declared(base, exp - 1);
            }
        }

        // Without the decrease check each of these would "prove" `false`.
        lemma circular_same(n: num) {
            ensures false;
        } proof {
            apply circular_same(n); // error: measure does not decrease
        }

        lemma circular_ascending(n: num) {
            ensures false;
        } proof {
            apply circular_ascending(n + 1); // error: measure does not decrease
        }

        // Descends, but without a lower bound.
        lemma unbounded(n: num) {
            ensures false;
        } proof {
            apply unbounded(n - 1); // error: measure does not decrease
        }

        // Recursion on the second parameter while the first changes: the
        // default `(n, i)` does not decrease...
        lemma second_param_default(n: num, i: num) {
            ensures i >= 0 ==> math_pow(2, i) >= 1;
        } proof {
            if (i > 0) {
                apply second_param_default(n + 1, i - 1); // error: measure does not decrease
            }
        }

        // ...but a declared `decreases i` does.
        lemma second_param_declared(n: num, i: num) {
            decreases i;
            ensures i >= 0 ==> math_pow(2, i) >= 1;
        } proof {
            if (i > 0) {
                apply second_param_declared(n + 1, i - 1);
            }
        }

        // Lexicographic measure: `i` restarts while `n` decreases.
        lemma lex(n: num, i: num) {
            decreases (n, i);
            ensures n >= 0 && i >= 0 ==> math_pow(2, n) >= 1;
        } proof {
            if (i > 0) {
                apply lex(n, i - 1);
            } else if (n > 0) {
                apply lex(n - 1, n);
            }
        }

        // Mutual recursion on the default measures.
        lemma even_pos(base: num, exp: num) {
            ensures base >= 1 && exp >= 0 ==> math_pow(base, exp) >= 1;
        } proof {
            if (exp > 0) {
                apply odd_pos(base, exp - 1);
            }
        }

        lemma odd_pos(base: num, exp: num) {
            ensures base >= 1 && exp >= 0 ==> math_pow(base, exp) >= 1;
        } proof {
            if (exp > 0) {
                apply even_pos(base, exp - 1);
            }
        }
    }
}
