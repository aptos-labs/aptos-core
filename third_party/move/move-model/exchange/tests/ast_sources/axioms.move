// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Module axioms are exported with the global invariants, generic ones with
/// their type parameters.
module 0x42::axioms {
    struct Counter has key {
        value: u64,
    }

    spec module {
        fun injection(v: vector<u8>): u64;
        axiom forall v1: vector<u8>, v2: vector<u8>: v1 == v2 <==> injection(v1) == injection(v2);

        fun generic_injection<T>(x: T): u64;
        axiom<T> forall x: T, y: T: x == y <==> generic_injection(x) == generic_injection(y);

        invariant forall a: address where exists<Counter>(a): global<Counter>(a).value > 0;
    }
}
