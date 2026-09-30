// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Write-backs in a generic callee translate when the callee is instantiated with a later type
// parameter of its caller.

module 0x42::write_back_local_type_inst {
    use std::signer;

    struct W<phantom A, phantom B> has key {}
    struct S<phantom A, phantom B> {}
    struct Q<phantom T> has key {}
    struct R2<phantom T> has key { value: bool }
    struct R<phantom T> has key {}

    spec module {
        invariant<X, Y> exists<Q<Y>>(@0x42) ==> (exists<W<X, Y>>(@0x42) || true);
    }

    fun g<X, Y, Z>(a: address) acquires R2 {
        borrow_global_mut<R2<X>>(a).value = false;
    }

    /// Must verify.
    public fun f<U, T, B, A, C>(s: &signer): bool acquires R2 {
        let b = exists<W<U, T>>(@0x42);
        move_to(s, Q<S<A, C>> {});
        g<B, B, B>(@0x42);
        b || true
    }
    spec f {
        requires signer::address_of(s) == @0x42;
        requires !exists<Q<S<A, C>>>(@0x42);
        requires exists<R2<B>>(@0x42);
        ensures result;
    }

    fun write_r2<X>(a: address) acquires R2 {
        borrow_global_mut<R2<X>>(a).value = false;
    }

    /// Must verify: the callee is verified at `[B]`, index 1 in a one-element instantiation.
    public fun h<A, B>(a: address): bool acquires R2 {
        let r = exists<R<A>>(a);
        write_r2<B>(a);
        r || true
    }
    spec h {
        requires exists<R2<B>>(a);
        ensures result;
    }
}
