// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// `decide` reads a goal up to negation, as it reads a hypothesis.
module 0x42::proof_negated_goals {
    spec fun held(x: u64): bool;

    fun not_not(x: u64): u64 {
        x
    }
    spec not_not {
        requires held(x);
        ensures !(!held(x));
    }

    fun not_or(x: u64, y: u64): u64 {
        x
    }
    spec not_or {
        requires !held(x);
        requires !held(y);
        ensures !(held(x) || held(y));
    }

    fun not_exists(x: u64): u64 {
        x
    }
    spec not_exists {
        requires forall k: u64 where k < x: !held(k);
        ensures !(exists k: u64 where k < x: held(k));
    }

    fun not_and(x: u64, y: u64): u64 {
        x
    }
    spec not_and {
        requires held(x) ==> !held(y);
        ensures !(held(x) && held(y));
    }
}
