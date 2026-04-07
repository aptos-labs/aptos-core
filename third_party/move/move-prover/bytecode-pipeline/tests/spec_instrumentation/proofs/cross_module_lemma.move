// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Test cross-module lemma resolution: module B applies a lemma defined in module A.
module 0x42::LemmaProvider {

    spec lemma add_comm(a: u64, b: u64) {
        ensures a + b == b + a;
    } proof {
        assume [trusted] true;
    }
}

module 0x42::LemmaConsumer {
    use 0x42::LemmaProvider;

    fun commutative_add(x: u64, y: u64): u64 {
        x + y
    }
    spec commutative_add {
        requires x + y <= MAX_U64;
        ensures result == y + x;
    } proof {
        apply LemmaProvider::add_comm(x, y);
    }
}
