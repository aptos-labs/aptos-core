// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A generic function is verified in every way its accessed resources can alias.

module 0x42::generic_aliasing_all_partitions {

    struct R<phantom T> has key {
        value: bool,
    }

    /// Must fail: false only at `T = U = V`, where all three writes hit one resource.
    public fun three_way<T, U, V>(a: address): bool {
        R<T>[a].value = false;
        R<U>[a].value = false;
        R<V>[a].value = true;
        R<T>[a].value == false || R<U>[a].value == false
    }
    spec three_way {
        requires exists<R<T>>(a) && exists<R<U>>(a) && exists<R<V>>(a);
        ensures result == true;
    }

    /// Must fail: false only at `A = B = C = D`.
    public fun four_way<A, B, C, D>(a: address): bool {
        R<A>[a].value = false;
        R<B>[a].value = false;
        R<C>[a].value = false;
        R<D>[a].value = true;
        R<A>[a].value == false || R<B>[a].value == false || R<C>[a].value == false
    }
    spec four_way {
        requires exists<R<A>>(a) && exists<R<B>>(a) && exists<R<C>>(a) && exists<R<D>>(a);
        ensures result == true;
    }

    /// Must fail: false at `T = U`.
    public fun pair<T, U>(a: address): bool {
        R<T>[a].value = false;
        R<U>[a].value = true;
        R<T>[a].value == false
    }
    spec pair {
        requires exists<R<T>>(a) && exists<R<U>>(a);
        ensures result == true;
    }

    /// Must verify: true in every aliasing case, including `T = U = V`.
    public fun true_in_every_case<T, U, V>(a: address): bool {
        R<T>[a].value = true;
        R<U>[a].value = true;
        R<V>[a].value = true;
        R<T>[a].value && R<U>[a].value && R<V>[a].value
    }
    spec true_in_every_case {
        requires exists<R<T>>(a) && exists<R<U>>(a) && exists<R<V>>(a);
        ensures result == true;
    }

    struct S<phantom T> has key {
        value: bool,
    }

    /// Must fail: false only at `T = U = u64`.
    public fun through_concrete<T, U>(a: address): bool {
        R<T>[a].value = false;
        R<U>[a].value = false;
        R<u64>[a].value = true;
        R<T>[a].value == false || R<U>[a].value == false
    }
    spec through_concrete {
        requires exists<R<T>>(a) && exists<R<U>>(a) && exists<R<u64>>(a);
        ensures result == true;
    }

    /// Must fail: false only at `T = U = V`, through both `R` and `S`.
    public fun across_structs<T, U, V>(a: address): bool {
        R<T>[a].value = false;
        S<T>[a].value = false;
        R<U>[a].value = true;
        S<V>[a].value = true;
        R<T>[a].value == false || S<T>[a].value == false
    }
    spec across_structs {
        requires exists<R<T>>(a) && exists<S<T>>(a) && exists<R<U>>(a) && exists<S<V>>(a);
        ensures result == true;
    }

    /// Must fail: false only when `U = vector<T>` and `T = V`.
    public fun through_nesting<T, U, V>(a: address): bool {
        R<vector<T>>[a].value = false;
        R<U>[a].value = false;
        R<vector<V>>[a].value = true;
        R<vector<T>>[a].value == false || R<U>[a].value == false
    }
    spec through_nesting {
        requires exists<R<vector<T>>>(a) && exists<R<U>>(a) && exists<R<vector<V>>>(a);
        ensures result == true;
    }

    /// Must verify: `R<T>` and `R<vector<T>>` never alias.
    public fun never_alias<T>(a: address): bool {
        R<T>[a].value = true;
        R<vector<T>>[a].value = true;
        R<T>[a].value
    }
    spec never_alias {
        requires exists<R<T>>(a) && exists<R<vector<T>>>(a);
        ensures result == true;
    }

    struct X<phantom A, phantom B> has key {
        value: bool,
    }

    struct Y<phantom A, phantom B> has key {
        value: bool,
    }

    /// Must fail: false only when `T3 = T2 = u64` and `T0 = T1 = vector<u64>` at once.
    public fun two_pairs<T0, T1, T2, T3>(a: address): bool {
        Y<T3, T1>[a].value = false;
        X<vector<T3>, vector<T2>>[a].value = false;
        Y<u64, T0>[a].value = true;
        X<T0, vector<T3>>[a].value = true;
        Y<T3, T1>[a].value == false || X<vector<T3>, vector<T2>>[a].value == false
    }
    spec two_pairs {
        requires exists<Y<T3, T1>>(a) && exists<X<vector<T3>, vector<T2>>>(a)
            && exists<Y<u64, T0>>(a) && exists<X<T0, vector<T3>>>(a);
        ensures result == true;
    }

    /// Must fail: false only at `T0 = T1 = T2 = u64`.
    public fun three_shared<T0, T1, T2>(a: address): bool {
        X<T2, T1>[a].value = false;
        X<T1, T0>[a].value = false;
        X<u64, T0>[a].value = true;
        X<T2, T1>[a].value == false || X<T1, T0>[a].value == false
    }
    spec three_shared {
        requires exists<X<T2, T1>>(a) && exists<X<T1, T0>>(a) && exists<X<u64, T0>>(a);
        ensures result == true;
    }
}
