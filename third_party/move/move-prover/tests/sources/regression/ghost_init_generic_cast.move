// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A cast in the ghost initializer of a generic struct. Instantiating the
// initializer at a pack creates its nodes after the number operation
// analysis has run, so the cast's argument has no entry in its state.
//
// Each spec also states `count(..)` directly. The instance of `count` is
// otherwise reached only through the instantiated initializer, and such
// instances are not yet collected for declaration.

module 0x42::ghost_init_generic_cast {

    struct Bag<T> has drop { items: vector<T> }
    spec Bag {
        ghost size: u128 = (count(items) as u128);
    }

    spec fun count<T>(v: vector<T>): u64 { len(v) }

    // A pack in code.

    fun make_two(): Bag<u8> {
        Bag { items: vector[1, 2] }
    }
    spec make_two {
        ensures count(result.items) == 2;
        ensures result.size == 2;
    }

    /// Must fail: the bag holds two items.
    fun make_two_wrong(): Bag<u8> {
        Bag { items: vector[1, 2] }
    }
    spec make_two_wrong {
        ensures count(result.items) == 2;
        ensures result.size == 3;
    }

    // A pack in a specification takes a second path to the same initializer.

    /// Must fail: the size is the length.
    fun id_wrong(v: vector<u8>): vector<u8> { v }
    spec id_wrong {
        ensures count(v) == len(v);
        ensures Bag<u8> { items: v }.size == len(v) + 1;
    }
}
