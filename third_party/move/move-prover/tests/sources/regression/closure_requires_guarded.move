// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A closure target's spec holds only under its `requires`. Invoking the closure with an
// argument that violates it gives an unknown outcome: no result, abort or memory facts.
// Each `*_fails` claim is false and must fail; each `*_ok` claim must verify.

module 0x42::closure_requires_guarded {
    struct R has key { v: u64 }

    fun g(x: u64): u64 { x + 1 }
    spec g {
        requires x < 10;
        aborts_if false;
        ensures result == x + 1;
    }

    // Aborts outside its precondition, which its `aborts_if` does not mention.
    fun g_opaque(x: u64): u64 {
        assert!(x < 10, 1);
        x + 1
    }
    spec g_opaque {
        pragma opaque;
        requires x < 10;
        aborts_if false;
        ensures result == x + 1;
    }

    // Writes `R` outside its precondition, which its `modifies` does not mention.
    fun bump(a: address, x: u64) acquires R {
        if (x >= 10) R[a].v = 0;
    }
    spec bump {
        pragma opaque;
        requires x < 10;
        aborts_if false;
        modifies R[a];
        ensures R[a] == old(R[a]);
    }

    fun add(r: &mut u64, x: u64) { *r = *r + x }
    spec add {
        requires x < 10;
        aborts_if r + x > MAX_U64;
        ensures r == old(r) + x;
    }

    fun apply(f: |u64| u64 has drop, x: u64): u64 { f(x) }

    // Its `ensures` restates the precondition.
    fun in_range(x: u64): bool { x < 10 }
    spec in_range {
        pragma opaque;
        requires x < 10;
        aborts_if false;
        ensures x < 10;
    }

    // Narrows to `u8`, aborting outside its precondition.
    fun narrow(x: u64): u8 {
        assert!(x < 10, 1);
        (x as u8)
    }
    spec narrow {
        pragma opaque;
        requires x < 10;
        aborts_if false;
        ensures result == x;
    }

    // Its precondition reads memory that it writes.
    fun reset(a: address): bool acquires R {
        let was_zero = R[a].v == 0;
        R[a].v = 0;
        was_zero
    }
    spec reset {
        pragma opaque;
        requires exists<R>(a);
        requires R[a].v == 0;
        aborts_if false;
        modifies R[a];
        ensures result;
    }

    // Brings `|| bool` into behavioral predicates, so its result function carries axioms.
    fun call_bool(f: || bool has drop): bool { f() }
    spec call_bool {
        pragma aborts_if_is_partial;
        ensures result == result_of<f>();
    }

    // The post-state satisfies `reset`'s precondition, the pre-state does not.
    fun opaque_post_state_fails(a: address): bool acquires R {
        let f: || bool has drop = || reset(a);
        f()
    }
    spec opaque_post_state_fails {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        requires R[a].v == 1;
        ensures R[a].v == 0 ==> result;
    }

    fun opaque_post_state_ok(a: address): bool acquires R {
        let f: || bool has drop = || reset(a);
        f()
    }
    spec opaque_post_state_ok {
        requires exists<R>(a);
        requires R[a].v == 0;
        aborts_if false;
        ensures result;
    }

    fun opaque_ensures_fails(): bool {
        let f: |u64| bool has drop = |x| in_range(x);
        f(100)
    }
    spec opaque_ensures_fails { ensures result == false; }

    // Outside the precondition the result is still a valid `u8`.
    fun opaque_result_range_ok(): u64 {
        let f: |u64| u8 has drop = |x| narrow(x);
        (f(100) as u64)
    }
    spec opaque_result_range_ok {
        pragma aborts_if_is_partial;
        ensures result < 256;
    }

    // Outside the precondition the result is still the target's result.
    fun opaque_result_of_ok(): u8 {
        let f: |u64| u8 has drop = |x| narrow(x);
        f(100)
    }
    spec opaque_result_of_ok {
        pragma aborts_if_is_partial;
        ensures result == result_of<narrow>(100);
    }

    fun mut_ref_fails(): u64 {
        let v = 0;
        let f: |&mut u64, u64| has drop = |r, x| add(r, x);
        f(&mut v, 100);
        v
    }
    spec mut_ref_fails { ensures result == 0; }

    fun mut_ref_ok(): u64 {
        let v = 0;
        let f: |&mut u64, u64| has drop = |r, x| add(r, x);
        f(&mut v, 5);
        v
    }
    spec mut_ref_ok { ensures result == 5; }

    fun transparent_fails(): u64 {
        let f: |u64| u64 has drop = |x| g(x);
        f(100)
    }
    spec transparent_fails { ensures result == 0; }

    fun transparent_ok(): u64 {
        let f: |u64| u64 has drop = |x| g(x);
        f(5)
    }
    spec transparent_ok { ensures result == 6; }

    fun through_hof_fails(): u64 {
        apply(|x| g(x), 100)
    }
    spec through_hof_fails { ensures result == 0; }

    fun through_hof_ok(): u64 {
        apply(|x| g(x), 5)
    }
    spec through_hof_ok { ensures result == 6; }

    fun opaque_abort_fails(): u64 {
        let f: |u64| u64 has drop = |x| g_opaque(x);
        f(100)
    }
    spec opaque_abort_fails { aborts_if false; }

    fun opaque_abort_ok(): u64 {
        let f: |u64| u64 has drop = |x| g_opaque(x);
        f(5)
    }
    spec opaque_abort_ok {
        aborts_if false;
        ensures result == 6;
    }

    fun opaque_frame_fails(a: address) acquires R {
        let f: |u64| has drop = |x| bump(a, x);
        f(100)
    }
    spec opaque_frame_fails {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        ensures R[a] == old(R[a]);
    }

    fun opaque_frame_ok(a: address) acquires R {
        let f: |u64| has drop = |x| bump(a, x);
        f(5)
    }
    spec opaque_frame_ok {
        requires exists<R>(a);
        aborts_if false;
        ensures R[a] == old(R[a]);
    }
}

// A transparent higher-order target with a `requires`, wrapped in a closure of another type,
// verifies: transparent targets are dispatched to their body, not to behavioral predicates.
module 0x42::closure_requires_transparent_hof {
    struct R has key { v: u64 }

    fun read(a: address, x: u64): u64 acquires R {
        R[a].v + x
    }
    spec read {
        aborts_if !exists<R>(a) || R[a].v + x > MAX_U64;
        ensures result == R[a].v + x;
    }

    fun hof(f: |u64| u64 has copy + drop, x: u64, _z: bool): u64 {
        f(x)
    }
    spec hof {
        requires x < 100;
        aborts_if aborts_of<f>(x);
        ensures result == result_of<f>(x);
    }

    fun apply2(g: |u64, bool| u64 has drop, x: u64): u64 {
        g(x, true)
    }

    fun test(a: address): u64 acquires R {
        let f: |u64| u64 has copy + drop = |x| read(a, x);
        apply2(|y, z| hof(f, y, z), 1)
    }
    spec test {
        requires exists<R>(a);
        requires R[a].v < 10;
        ensures result == R[a].v + 1;
    }
}
