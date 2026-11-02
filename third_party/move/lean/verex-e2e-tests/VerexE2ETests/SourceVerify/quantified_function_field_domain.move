// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::quantified_function_field_domain {
    struct G<phantom T> has key, drop, copy, store {
        f: |u64|u64 has copy+store+drop,
    }
    spec G {
        invariant forall o: G<u64>, x: u64, r: u64:
            ensures_of<o.f>(x, r) ==> r >= x;
    }

    // Must not prove a property of G<bool>'s field from a G<u64> domain.
    // Until field-validity domains are modeled, reject the quantifier explicitly.
    fun other_instantiation(g: &G<bool>): u64 { (g.f)(5) }
    spec other_instantiation { ensures result >= 5; }
}
