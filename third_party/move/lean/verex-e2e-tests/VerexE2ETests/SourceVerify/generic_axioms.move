// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A generic axiom is assumed at each instantiation a verification applies its
// specification functions at: in the function's own specification, in its
// callees' contracts, and in the specification functions these expand.
module 0x42::generic_axioms {
    spec module {
        fun spec_id<T>(x: T): T;
        axiom<T> forall x: T: spec_id(x) == x;

        fun tag<T>(x: T): u64;
        axiom<T> forall a: T, b: T: a == b <==> tag(a) == tag(b);
    }

    // At the function's own type parameter.
    fun id_t<T>(x: T): T { x }
    spec id_t {
        ensures result == spec_id(x);
    }

    // At a concrete type, whose values are bounded.
    fun id_u64(x: u64): u64 { x }
    spec id_u64 {
        ensures result == spec_id(x);
    }

    // Through a callee's contract, at the type the call gives.
    fun hash<T: drop>(_x: T): u64 { 0 }
    spec hash {
        pragma opaque;
        ensures [abstract] result == tag(_x);
    }

    fun distinct(a: vector<u8>, b: vector<u8>): bool { hash(a) != hash(b) }
    spec distinct {
        ensures result == (a != b);
    }
}
