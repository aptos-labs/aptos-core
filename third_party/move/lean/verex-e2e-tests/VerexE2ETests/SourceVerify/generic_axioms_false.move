// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::generic_axioms_false {
    spec module {
        fun tag<T>(x: T): u64;
        axiom<T> forall a: T, b: T: a == b ==> tag(a) == tag(b);
    }

    fun hash<T: drop>(_x: T): u64 { 0 }
    spec hash {
        pragma opaque;
        ensures [abstract] result == tag(_x);
    }

    fun distinct(a: vector<u8>, b: vector<u8>): bool { hash(a) != hash(b) }
    spec distinct {
        ensures result == (a != b); // error: the axiom is not injective
    }
}
