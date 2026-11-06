// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// An axiom concluding an equation of its binders matches any term of their
// type, so it is no rewrite rule; it stays a fact, instantiated where a goal
// states such an equation.
module 0x42::axiom_binder_equation {
    spec module {
        axiom forall a: vector<u8>, b: vector<u8>:
            spec_digest(a) == spec_digest(b) ==> a == b;
    }

    spec fun spec_digest(bytes: vector<u8>): u64;

    fun same_digest(a: vector<u8>, b: vector<u8>): (vector<u8>, vector<u8>) {
        (a, b)
    }
    spec same_digest {
        requires spec_digest(a) == spec_digest(b);
        ensures result_1 == result_2;
    }

    fun length(bytes: vector<u8>): u64 {
        std::vector::length(&bytes)
    }
    spec length {
        ensures result == len(bytes);
    }

    fun not_equal(a: vector<u8>, b: vector<u8>): u64 {
        std::vector::length(&a) + std::vector::length(&b)
    }
    spec not_equal {
        requires len(a) + len(b) < 100;
        ensures a == b ==> result == 2 * len(a);
    }
}
