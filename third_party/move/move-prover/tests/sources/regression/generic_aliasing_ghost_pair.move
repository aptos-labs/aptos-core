// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// An invariant instance with two ghost type parameters is checked against a written resource
// whose type fixes both.

module 0x42::generic_aliasing_ghost_pair {
    use std::signer;

    struct P<phantom T, phantom U> has key { v: u64 }
    struct M has key {}
    struct N has key {}

    spec module {
        invariant<T, U> (!exists<N>(@0x42) ==> !exists<P<T, U>>(@0x42))
            && (exists<M>(@0x42) ==> !exists<P<T, U>>(@0x42));
    }

    /// Must fail at the write to `M`.
    public fun concrete(s: &signer) {
        move_to(s, N {});
        move_to(s, P<u64, u8> { v: 0 });
        move_to(s, M {});
    }
    spec concrete {
        requires signer::address_of(s) == @0x42;
        requires !exists<M>(@0x42) && !exists<N>(@0x42);
    }

    /// Must fail at the write to `M`, at `G0 = X, G1 = Y`.
    public fun declared<X, Y>(s: &signer) {
        move_to(s, N {});
        move_to(s, P<X, Y> { v: 0 });
        move_to(s, M {});
    }
    spec declared {
        requires signer::address_of(s) == @0x42;
        requires !exists<M>(@0x42) && !exists<N>(@0x42);
    }
}
