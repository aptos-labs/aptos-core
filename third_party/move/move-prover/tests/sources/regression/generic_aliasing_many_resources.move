// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Aliasing cases grow with the cases, not the resources: one type parameter over twelve
// resources gives four cases.

module 0x42::generic_aliasing_many_resources {

    struct A {}
    struct B {}
    struct C {}

    struct R<phantom T> has key { value: bool }
    struct S<phantom T> has key { value: bool }
    struct Q<phantom T> has key { value: bool }

    /// Must fail: false only at `T = B`, where the write to `R<T>` is the write to `R<B>`.
    public fun only_at_b<T>(a: address): bool {
        R<B>[a].value = true;
        R<T>[a].value = false;
        let _ = exists<R<A>>(a) || exists<R<C>>(a) || exists<S<T>>(a) || exists<S<A>>(a)
            || exists<S<B>>(a) || exists<S<C>>(a) || exists<Q<T>>(a) || exists<Q<A>>(a)
            || exists<Q<B>>(a) || exists<Q<C>>(a);
        R<B>[a].value
    }
    spec only_at_b {
        requires exists<R<B>>(a) && exists<R<T>>(a);
        aborts_if false;
        ensures result == true;
    }

    /// Must verify: true in every aliasing case.
    public fun in_every_case<T>(a: address): bool {
        exists<R<T>>(a) || exists<R<A>>(a) || exists<R<B>>(a) || exists<R<C>>(a)
            || exists<S<T>>(a) || exists<S<A>>(a) || exists<S<B>>(a) || exists<S<C>>(a)
            || exists<Q<T>>(a) || exists<Q<A>>(a) || exists<Q<B>>(a) || exists<Q<C>>(a)
    }
    spec in_every_case {
        aborts_if false;
    }
}
