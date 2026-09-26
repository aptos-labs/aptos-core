// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A generic function must be verified in every way its accessed resources can alias.
//
// A generic function is verified at the open instantiation, where distinct type parameters
// are treated as distinct types, plus one extra instantiation per aliasing case. Those cases
// were derived only for pairs of accessed resources: with `R<T>`, `R<U>` and `R<V>`, the
// cases `T = U`, `T = V` and `U = V` were verified but `T = U = V` never was, so a
// postcondition false only when all three alias was proved. The cases are now derived to a
// fixpoint, covering every merge of the resources that can coincide.

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

    /// Must fail: false only at `A = B = C = D`, which takes more than one merge step to reach.
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

    /// Must fail: the pairwise case, which was already derived.
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

    /// Must verify, and must not hit the cap on aliasing cases: six aliasable resources have
    /// 203 distinct aliasing cases. The same case must be counted once whichever parameter
    /// represents each merged group, or the count exceeds the cap and this is rejected.
    public fun six_under_cap<T1, T2, T3, T4, T5, T6>(a: address): bool {
        R<T1>[a].value = true;
        R<T2>[a].value = true;
        R<T3>[a].value = true;
        R<T4>[a].value = true;
        R<T5>[a].value = true;
        R<T6>[a].value = true;
        R<T1>[a].value
    }
    spec six_under_cap {
        requires exists<R<T1>>(a) && exists<R<T2>>(a) && exists<R<T3>>(a)
            && exists<R<T4>>(a) && exists<R<T5>>(a) && exists<R<T6>>(a);
        ensures result == true;
    }
}
