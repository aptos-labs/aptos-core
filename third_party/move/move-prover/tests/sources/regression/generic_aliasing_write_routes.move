// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A read resource aliases every resource written under a coinciding type, whether written by
// the function, a callee, a native or opaque `modifies`, a `modifies_of` frame, or a closure.

module 0x42::generic_aliasing_write_routes {

    struct R<phantom T> has key { value: bool }
    struct A {}


    /// Must fail: false only at `T = U`, where the read of `R<U>` sees the write to `R<T>`.
    public fun read_sees_write<T, U>(a: address): bool {
        let before = R<U>[a].value;
        R<T>[a].value = !before;
        R<U>[a].value == before
    }
    spec read_sees_write {
        requires exists<R<T>>(a) && exists<R<U>>(a);
        ensures result == true;
    }

    /// Must fail: false only at `T = U = V`, where both reads see the write to `R<V>`. The
    /// case is reached through two equations, each with the written side.
    public fun two_reads_one_write<T, U, V>(a: address): bool {
        R<V>[a].value = false;
        R<T>[a].value || R<U>[a].value
    }
    spec two_reads_one_write {
        requires exists<R<T>>(a) && exists<R<U>>(a) && exists<R<V>>(a);
        requires R<T>[a].value || R<U>[a].value;
        ensures result == true;
    }

    fun write_r<T>(a: address) {
        R<T>[a].value = false;
    }

    /// Must fail: false only at `T = U`, where the callee's write to `R<T>` is a write to the
    /// `R<U>` read here.
    public fun callee_writes<T, U>(a: address): bool {
        let before = R<U>[a].value;
        write_r<T>(a);
        R<U>[a].value == before
    }
    spec callee_writes {
        requires exists<R<T>>(a) && exists<R<U>>(a);
        requires R<U>[a].value;
        ensures result == true;
    }

    native fun remove_r<X>(a: address);
    spec remove_r {
        pragma opaque;
        modifies global<R<X>>(a);
        ensures !exists<R<X>>(a);
    }

    /// Must fail: false only at `T = U`, where the native callee removes the `R<U>` read here.
    /// The write is known only from the callee's `modifies`.
    public fun native_modifies<T, U>(a: address): bool {
        remove_r<T>(a);
        exists<R<U>>(a)
    }
    spec native_modifies {
        requires exists<R<U>>(a);
        ensures result;
    }

    fun declared_only<X>(_a: address) {}
    spec declared_only {
        pragma opaque;
        modifies global<R<X>>(_a);
    }

    /// Must fail: false only at `T = U`. The opaque callee's code writes nothing, but its
    /// `modifies` lets it write `R<T>`, which is then the `R<U>` read here.
    public fun opaque_modifies<T, U>(a: address): bool {
        let before = R<U>[a].value;
        declared_only<T>(a);
        R<U>[a].value == before
    }
    spec opaque_modifies {
        requires exists<R<T>>(a) && exists<R<U>>(a);
        ensures result;
    }

    /// Must fail: false only at `U = A`, where the function value, which may write `R<A>` by
    /// its frame, may write the `R<U>` read here.
    public fun frame_writes<U>(f: |address| has drop, a: address): bool {
        let before = R<U>[a].value;
        f(a);
        R<U>[a].value == before
    }
    spec frame_writes {
        modifies_of<f>(x: address) R<A>[x];
        requires exists<R<A>>(a) && exists<R<U>>(a);
        ensures result == true;
    }

    fun write_r_acquires<X>(a: address) acquires R {
        borrow_global_mut<R<X>>(a).value = false;
    }

    /// Must fail: false only at `T = U`, where the closure's write to `R<T>` is a write to the
    /// `R<U>` read here.
    public fun closure_writes<T, U>(a: address): bool acquires R {
        let g = |x| write_r_acquires<T>(x);
        g(a);
        borrow_global<R<U>>(a).value
    }
    spec closure_writes {
        requires exists<R<T>>(a) && exists<R<U>>(a);
        requires R<U>[a].value;
        ensures result;
    }

    /// Must fail: false only at `U = A`, with `R<A>` named only by the frame.
    public fun frame_only<U>(f: |address| has drop, a: address): bool {
        let before = R<U>[a].value;
        f(a);
        R<U>[a].value == before
    }
    spec frame_only {
        modifies_of<f>(x: address) R<A>[x];
        requires exists<R<U>>(a);
        ensures result == true;
    }
}
