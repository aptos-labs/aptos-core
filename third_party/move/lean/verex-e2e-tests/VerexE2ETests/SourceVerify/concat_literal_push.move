// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A clause's concatenation with a literal vector and a body's `push_back` read
// as one value, also as the argument of an opaque function.
module 0x42::concat_literal_push {
    use std::vector;

    spec fun digest(bytes: vector<u8>): u64;

    fun hash(bytes: vector<u8>): u64 { (vector::length(&bytes) as u64) }
    spec hash {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == digest(bytes);
    }

    fun tagged(bytes: vector<u8>): u64 {
        vector::push_back(&mut bytes, 7);
        hash(bytes)
    }
    spec tagged {
        pragma aborts_if_is_partial;
        ensures result == digest(concat(bytes, vec(7u8)));
    }

    fun wrong_tag(bytes: vector<u8>): u64 {
        vector::push_back(&mut bytes, 7);
        hash(bytes)
    }
    spec wrong_tag {
        pragma aborts_if_is_partial;
        ensures result == digest(concat(bytes, vec(8u8))); // error: another tag
    }
}
