// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Function values whose identity is not known -- parameters and struct fields -- may be equal.

module 0x42::fun_value_equality {

    struct Holder has drop {
        cb: |u64|u64 has copy+store+drop,
    }

    /// Must fail: a caller may pass the same function value twice.
    public fun two_params(p: |u64|u64 has copy+drop, q: |u64|u64 has copy+drop): u64 {
        p(1) + q(1)
    }
    spec two_params {
        pragma opaque;
        ensures p != q;
    }

    /// Must fail: `s.cb` may hold the value passed as `p`.
    public fun param_and_field(s: &Holder, p: |u64|u64 has copy+store+drop): bool {
        let _ = p;
        let _ = s;
        true
    }
    spec param_and_field {
        ensures s.cb != p;
    }

    /// Must fail: two holders may store the same function value.
    public fun two_fields(a: &Holder, b: &Holder): bool {
        let _ = a;
        let _ = b;
        true
    }
    spec two_fields {
        ensures a.cb != b.cb;
    }

    /// Must verify: a value equals itself.
    public fun same_param(p: |u64|u64 has copy+drop): u64 {
        p(1)
    }
    spec same_param {
        ensures p == p;
    }

    /// Must verify: a field equals itself.
    public fun same_field(a: &Holder): bool {
        let _ = a;
        true
    }
    spec same_field {
        ensures a.cb == a.cb;
    }

    struct Box has copy, drop {
        f: |u64|u64 has copy+drop,
    }

    #[persistent]
    fun inc(x: u64): u64 {
        x + 1
    }

    fun call(f: |u64|u64 has copy+drop, x: u64): u64 {
        f(x)
    }

    /// Must fail: structs holding the same function value are equal.
    public fun boxed_params(p: |u64|u64 has copy+drop, q: |u64|u64 has copy+drop): bool {
        Box { f: p } == Box { f: q }
    }
    spec boxed_params {
        ensures result == false;
    }

    fun wrap(f: |u64|u64 has copy+drop): |u64|u64 has copy+drop {
        |x| call(f, x)
    }

    /// Must fail: one closure capturing the same function value twice is equal to itself.
    public fun captured_params(p: |u64|u64 has copy+drop, q: |u64|u64 has copy+drop): bool {
        wrap(p) == wrap(q)
    }
    spec captured_params {
        ensures result == false;
    }

    /// Must fail: tuples holding the same function value are equal.
    public fun tupled_params(p: |u64|u64 has copy+drop, q: |u64|u64 has copy+drop): bool {
        let _ = p;
        let _ = q;
        true
    }
    spec tupled_params {
        ensures (p, 1) != (q, 1);
    }

    /// Must fail: a struct field may hold a named function value.
    public fun field_and_closure(s: &Holder): bool {
        let k: |u64|u64 has copy+store+drop = inc;
        s.cb == k
    }
    spec field_and_closure {
        ensures result == false;
    }

    fun id(p: |u64|u64 has copy+drop): |u64|u64 has copy+drop {
        p
    }
    spec id {
        pragma opaque;
        ensures result == p;
    }

    /// Must verify: equal function values make equal structs.
    public fun boxed_equal(p: |u64|u64 has copy+drop): bool {
        Box { f: id(p) } == Box { f: p }
    }
    spec boxed_equal {
        ensures result;
    }

    /// Must verify: equality of function values is symmetric and transitive.
    public fun equivalence(p: |u64|u64 has copy+drop, q: |u64|u64 has copy+drop, r: |u64|u64 has copy+drop): u64 {
        p(1) + q(1) + r(1)
    }
    spec equivalence {
        ensures (p == q) == (q == p);
        ensures p == q && q == r ==> p == r;
    }
}
