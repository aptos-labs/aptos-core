// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --vector-theory=SmtArrayExt

// Structs holding function values of unknown identity may be equal under an extensional vector theory.

module 0x42::fun_value_equality_ext {

    struct Box has copy, drop {
        f: |u64|u64 has copy+drop,
    }

    /// Must fail: structs holding the same function value are equal.
    public fun boxed_params(p: |u64|u64 has copy+drop, q: |u64|u64 has copy+drop): bool {
        Box { f: p } == Box { f: q }
    }
    spec boxed_params {
        ensures result == false;
    }

    fun id(p: |u64|u64 has copy+drop): |u64|u64 has copy+drop {
        p
    }
    spec id {
        pragma opaque;
        ensures result == p;
    }

    /// Must verify: equal function values make equal structs.
    public fun boxed_equal(p: |u64|u64 has copy+drop): bool {
        Box { f: id(p) } == Box { f: p }
    }
    spec boxed_equal {
        ensures result;
    }
}
