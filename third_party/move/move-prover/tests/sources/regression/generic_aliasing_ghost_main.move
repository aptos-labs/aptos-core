// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Aliasing cases for ghost type parameters include matches with one side's type parameters
// fixed: each `f` violates its invariant only when two ghosts are `u64` and `u8` at once.

module 0x42::generic_aliasing_ghost_main_match {
    use std::signer;
    struct R<phantom T> has key {}
    struct S<phantom T> has key {}
    struct Q<phantom T> has key {}
    struct W<phantom A, phantom B, phantom C> has key {}
    struct M has key {}
    struct N has key {}

    spec module {
        invariant<T, U, X>
            (!exists<N>(@0x42) ==> !exists<R<T>>(@0x42) && !exists<S<U>>(@0x42) && !exists<Q<X>>(@0x42))
            && (exists<M>(@0x42) ==> !(exists<R<T>>(@0x42) && exists<S<U>>(@0x42)) || exists<Q<X>>(@0x42))
            && (exists<W<T, U, X>>(@0x7) ==> exists<W<T, U, X>>(@0x7))
            && (exists<W<u64, u8, T>>(@0x7) ==> exists<W<u64, u8, T>>(@0x7));
    }

    public fun set_n(s: &signer) { move_to(s, N {}); }
    spec set_n {
        requires signer::address_of(s) == @0x42;
        ensures exists<N>(@0x42);
    }
    public fun set_r(s: &signer) { move_to(s, R<u64> {}); }
    spec set_r {
        requires signer::address_of(s) == @0x42;
        requires exists<N>(@0x42) && !exists<M>(@0x42);
        ensures exists<R<u64>>(@0x42);
    }
    public fun set_s(s: &signer) { move_to(s, S<u8> {}); }
    spec set_s {
        requires signer::address_of(s) == @0x42;
        requires exists<N>(@0x42) && !exists<M>(@0x42);
        ensures exists<S<u8>>(@0x42);
    }
    public fun set_q(s: &signer) { move_to(s, Q<u64> {}); }
    spec set_q {
        requires signer::address_of(s) == @0x42;
        requires exists<N>(@0x42) && !exists<M>(@0x42);
        ensures exists<Q<u64>>(@0x42);
    }

    /// Must fail at the write to M at T = u64, U = u8, X generic.
    public fun f(s: &signer) {
        set_n(s);
        set_r(s);
        set_s(s);
        set_q(s);
        move_to(s, M {});
    }
    spec f {
        requires signer::address_of(s) == @0x42;
        requires !exists<M>(@0x42) && !exists<N>(@0x42);
    }
}

module 0x43::generic_aliasing_ghost_main_spec_variance {
    use std::signer;
    struct R<phantom T> has key {}
    struct S<phantom T> has key {}
    struct W<phantom A, phantom B, phantom C> has key {}
    struct M has key {}
    struct N has key {}

    spec module {
        invariant<T, U>
            (!exists<N>(@0x43) ==> !exists<R<T>>(@0x43) && !exists<S<U>>(@0x43))
            && (exists<M>(@0x43) ==> !(exists<R<T>>(@0x43) && exists<S<U>>(@0x43)))
            && (exists<W<u8, T, U>>(@0x7) ==> exists<W<u64, u64, u8>>(@0x7) || true);
    }

    public fun set_n(s: &signer) { move_to(s, N {}); }
    spec set_n {
        requires signer::address_of(s) == @0x43;
        ensures exists<N>(@0x43);
    }
    public fun set_r(s: &signer) { move_to(s, R<u64> {}); }
    spec set_r {
        requires signer::address_of(s) == @0x43;
        requires exists<N>(@0x43) && !exists<M>(@0x43);
        ensures exists<R<u64>>(@0x43);
    }
    public fun set_s(s: &signer) { move_to(s, S<u8> {}); }
    spec set_s {
        requires signer::address_of(s) == @0x43;
        requires exists<N>(@0x43) && !exists<M>(@0x43);
        ensures exists<S<u8>>(@0x43);
    }

    /// Must fail at the write to M at T = u64, U = u8.
    public fun f(s: &signer) {
        set_n(s);
        set_r(s);
        set_s(s);
        move_to(s, M {});
    }
    spec f {
        requires signer::address_of(s) == @0x43;
        requires !exists<M>(@0x43) && !exists<N>(@0x43);
    }
}
