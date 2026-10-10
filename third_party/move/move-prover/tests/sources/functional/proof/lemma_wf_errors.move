// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// Model-level errors of the well-foundedness check for recursive lemmas.
module 0x42::lemma_wf_errors {

    spec module {
        fun id(b: bool): bool { b }

        // Recursive, but no integer parameter to default the measure to.
        lemma no_measure(b: bool) { // error: needs a `decreases` clause
            ensures id(b) == b;
        } proof {
            apply no_measure(b);
        }

        lemma two_clauses(n: num) {
            decreases n;
            decreases n; // error: at most one `decreases` clause
            ensures n == n;
        } proof {
            if (n > 0) {
                apply two_clauses(n - 1);
            }
        }

        // Group members disagree on the measure's arity.
        lemma pair_a(n: num, i: num) {
            decreases (n, i);
            ensures true;
        } proof {
            if (n > 0) {
                apply pair_b(n - 1, i);
            }
        }

        lemma pair_b(n: num, i: num) { // error: measure of 1 component, group uses 2
            decreases n;
            ensures true;
        } proof {
            if (n > 0) {
                apply pair_a(n - 1, i);
            }
        }

        // A quantified application of a group member has no decreasing instance.
        lemma quantified_self(n: num) {
            ensures true;
        } proof {
            forall k: num {id(k > 0)} apply quantified_self(k); // error: same recursion group
        }

        lemma bad_component(n: num) {
            decreases id(true); // error: components must be integers
            ensures true;
        } proof {
            if (n > 0) {
                apply bad_component(n - 1);
            }
        }

        lemma empty_measure(n: num) {
            decreases (); // error: requires at least one component
            ensures n == n;
        } proof {
            if (n > 0) {
                apply empty_measure(n - 1);
            }
        }
    }

    fun g(x: u64): u64 {
        x
    }
    spec g {
        decreases x; // error: not supported outside lemmas
    }
}
