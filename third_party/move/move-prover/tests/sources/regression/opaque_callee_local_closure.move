// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// An opaque function may modify memory through a closure it creates and runs, but not through
// one it only returns or drops.

module 0x42::opaque_callee_local_closure {
    use std::vector;

    struct R has key { value: u64 }

    fun bump(a: address) acquires R {
        R[a].value = R[a].value + 1;
    }

    fun local_closure(a: address) {
        let g: |address| has copy + drop = |x| bump(x);
        if (exists<R>(a)) g(a)
    }
    spec local_closure {
        pragma opaque;
    }

    /// Must fail: `local_closure` may change `R[a]`.
    public fun caller(a: address) {
        local_closure(a)
    }
    spec caller {
        requires exists<R>(a);
        ensures R[a].value == old(R[a].value);
    }

    fun call(f: |address| has copy + drop, a: address) {
        if (exists<R>(a)) f(a)
    }

    fun passed_closure(a: address) {
        call(|x| bump(x), a)
    }
    spec passed_closure {
        pragma opaque;
    }

    /// Must fail: the closure passed to `call` may change `R[a]`.
    public fun passed_caller(a: address) {
        passed_closure(a)
    }
    spec passed_caller {
        requires exists<R>(a);
        ensures R[a].value == old(R[a].value);
    }

    fun captured_closure(a: address) {
        let g: |address| has copy + drop = |x| bump(x);
        let h: |address| has copy + drop = |x| g(x) spec { ensures true; };
        if (exists<R>(a)) h(a)
    }
    spec captured_closure {
        pragma opaque;
    }

    /// Must fail: `h` runs the captured `g`, which may change `R[a]`.
    public fun captured_caller(a: address) {
        captured_closure(a)
    }
    spec captured_caller {
        requires exists<R>(a);
        ensures R[a].value == old(R[a].value);
    }

    fun capture_and_drop(v: u64): u64 {
        let g: |address| has copy + drop = |x| bump(x);
        let _h: |address| has copy + drop = |x| g(x) spec { ensures true; };
        v
    }
    spec capture_and_drop {
        pragma opaque;
        ensures result == v;
    }

    /// Must verify: neither closure runs.
    public fun captured_dropped(_a: address): u64 {
        capture_and_drop(1)
    }
    spec captured_dropped {
        requires exists<R>(_a);
        ensures R[_a].value == old(R[_a].value);
    }

    fun forward(f: |address| has copy + drop, a: address) {
        call(f, a)
    }

    fun forwarded_closure(a: address) {
        forward(|x| bump(x), a)
    }
    spec forwarded_closure {
        pragma opaque;
    }

    /// Must fail: `forward` passes the closure to `call`, which runs it.
    public fun forwarded_caller(a: address) {
        forwarded_closure(a)
    }
    spec forwarded_caller {
        requires exists<R>(a);
        ensures R[a].value == old(R[a].value);
    }

    fun keep(_f: |address| has copy + drop) {}

    fun passed_not_run(v: u64): u64 {
        let g: |address| has copy + drop = |x| bump(x);
        keep(|x| g(x) spec { ensures true; });
        v
    }
    spec passed_not_run {
        pragma opaque;
        ensures result == v;
    }

    /// Must verify: `keep` does not run the closure.
    public fun passed_not_run_caller(_a: address): u64 {
        passed_not_run(1)
    }
    spec passed_not_run_caller {
        requires exists<R>(_a);
        ensures R[_a].value == old(R[_a].value);
    }

    fun stored(v: u64): u64 {
        let fs: vector<|address| has copy + drop> = vector[];
        vector::push_back(&mut fs, |x| bump(x));
        let _ = fs;
        v
    }
    spec stored {
        pragma opaque;
        ensures result == v;
    }

    /// Must verify: the closure is only stored in a vector.
    public fun stored_caller(_a: address): u64 {
        stored(1)
    }
    spec stored_caller {
        requires exists<R>(_a);
        ensures R[_a].value == old(R[_a].value);
    }

    fun make(): |address| has copy + drop {
        |x| bump(x)
    }
    spec make {
        pragma opaque;
    }

    /// Must verify: the closure is only returned.
    public fun returned(_a: address) {
        let _f = make();
    }
    spec returned {
        requires exists<R>(_a);
        ensures R[_a].value == old(R[_a].value);
    }

    fun make_and_drop(v: u64): u64 {
        let _g: |address| has copy + drop = |x| bump(x);
        v
    }
    spec make_and_drop {
        pragma opaque;
        ensures result == v;
    }

    /// Must verify: the closure is dropped.
    public fun dropped(_a: address): u64 {
        make_and_drop(1)
    }
    spec dropped {
        requires exists<R>(_a);
        ensures R[_a].value == old(R[_a].value);
    }
}
