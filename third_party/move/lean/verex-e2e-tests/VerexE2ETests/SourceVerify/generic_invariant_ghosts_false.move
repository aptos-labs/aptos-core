// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A write that leaves a generic invariant's type parameter undetermined owes
// the invariant at a ghost type parameter, as the Move Prover adds one: the
// function is proved for every type there.
module 0x42::generic_invariant_ghosts_false {
    use std::signer;

    struct Box<T: store> has key { value: T }
    struct Marker has key {}

    spec module {
        invariant<T> forall a: address where exists<Box<T>>(a): exists<Marker>(a);
    }

    public fun mark(account: &signer) {
        move_to(account, Marker {});
    }

    public fun unmark(a: address) acquires Marker {
        let Marker {} = move_from<Marker>(a); // error: a `Box<T>` may be at `a`
    }

    public fun publish<T: store>(account: &signer, value: T) {
        assert!(exists<Marker>(signer::address_of(account)), 1);
        move_to(account, Box { value });
    }
}
