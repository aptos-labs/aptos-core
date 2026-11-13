// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Closures that stay within the callee's frame are accepted, even when the frames of
// functions they call, or memory mentioned only by specifications, lie outside it.

module 0x42::fun_arg_frame_check_valid {
    struct R has key { value: u64 }
    struct S has key { value: u64 }

    struct Holder has key { h: |address| has copy+drop+store }
    spec Holder {
        modifies_of<h>(a: address) R[a];
    }

    fun pure_get(_a: address): u64 {
        1
    }

    fun call_it(g: |address| u64, a: address): u64 {
        g(a)
    }
    spec call_it {
        pragma opaque;
        modifies_of<g>(x: address) R[x];
        aborts_if aborts_of<g>(a);
        ensures ensures_of<g>(a, result);
    }

    fun call_plain(g: |address| u64, a: address): u64 {
        g(a)
    }
    spec call_plain {
        pragma opaque;
        aborts_if aborts_of<g>(a);
        ensures ensures_of<g>(a, result);
    }

    fun apply_pure(fv: |address| u64, a: address): u64 {
        fv(a)
    }
    spec apply_pure {
        pragma opaque;
        pragma verify = false;
    }

    fun get(a: address): u64 acquires R {
        R[a].value
    }
    spec get {
        aborts_if !exists<R>(a);
        ensures result == R[a].value;
        ensures forall x: address: S[x] == old(S[x]);
    }

    fun get2(a: address): u64 {
        get(a)
    }
    spec get2 {
        pragma opaque;
        aborts_if !exists<R>(a);
        ensures result == R[a].value;
    }

    fun apply_reads_r(fv: |address| u64, a: address): u64 {
        fv(a)
    }
    spec apply_reads_r {
        pragma opaque;
        pragma verify = false;
        reads_of<fv> R;
        aborts_if aborts_of<fv>(a);
        ensures ensures_of<fv>(a, result);
    }

    /// The lambda passes a pure function to `call_it`, whose frame allows writes to `R`.
    public fun callee_frame(a: address): u64 {
        apply_pure(|x| call_it(pure_get, x) spec { aborts_if false; }, a)
    }

    /// The lambda invokes a function value; `Holder`'s frame for its field is unrelated.
    public fun field_frame(a: address): u64 {
        apply_pure(|x| call_plain(pure_get, x) spec { aborts_if false; }, a)
    }

    /// `get2` reads only `R`; `S` appears only in `get`'s specification.
    public fun spec_only_memory(a: address): u64 {
        apply_reads_r(get2, a)
    }
    spec spec_only_memory {
        aborts_if !exists<R>(a);
        ensures result == R[a].value;
    }
}
