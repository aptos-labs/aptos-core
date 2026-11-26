// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4
// exclude_for: lean
// The Lean backend does not establish `in_let` (an equality of vectors with different integer
// element types under an uninterpreted value) and fails to synthesize membership for the
// quantification over a vector literal in `in_quantifier`; neither concerns receiver calls.

// A receiver-style call whose receiver type is inferred only after the call is translated, in a
// condition: the result is related to its context as before.
module 0x42::receiver_call_inferred {
    struct S has copy, drop { v: vector<u64> }

    fun values(self: &S): vector<u64> { self.v }

    spec fun arb<T>(): T;

    fun in_let(s: S, w: vector<u8>): u64 { len_of(&s) + std::vector::length(&w) }
    spec in_let {
        pragma aborts_if_is_partial;
        ensures { let x = arb(); x.values() == w && x == s } ==> len(w) == len(s.v);
    }

    fun in_quantifier(s: S): u64 { len_of(&s) }
    spec in_quantifier {
        ensures forall x in vector[s]: len(x.values()) == result;
    }

    fun in_quantifier_incorrect(s: S): u64 { len_of(&s) }
    spec in_quantifier_incorrect {
        ensures forall x in vector[s]: len(x.values()) == result + 1;
    }

    fun len_of(s: &S): u64 { std::vector::length(&s.v) }
}
