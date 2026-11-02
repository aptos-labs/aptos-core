// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A recursive call past the inlining depth is reported, not dropped. A recursive function that
// is not opaque fails verification; an opaque one proves its spec by induction through its own
// spec at the recursive call.

module 0x42::recursion_inline_bound {
    // Not opaque: its own verification reaches the recursion past the inlining depth.
    fun count(n: u64): u64 {
        if (n == 0) 0 else count(n - 1) + 1
    }

    // Not opaque and not verified on its own: only its caller reports the recursion.
    fun count_unverified(n: u64): u64 {
        if (n == 0) 0 else count_unverified(n - 1) + 1
    }
    spec count_unverified {
        pragma verify = false;
    }

    // FALSE: count_unverified(5) == 5.
    fun root(): u64 {
        count_unverified(5)
    }
    spec root {
        ensures result == 0;
    }

    // Opaque: the recursive call uses the spec.
    fun count_opaque(n: u64): u64 {
        if (n == 0) 0 else count_opaque(n - 1) + 1
    }
    spec count_opaque {
        pragma opaque;
        aborts_if false;
        ensures result == n;
    }

    fun root_opaque(): u64 {
        count_opaque(5)
    }
    spec root_opaque {
        ensures result == 5;
    }
}

// Nested calls through function values are not recursion: each level reaches a different
// closure, so they are inlined fully.
module 0x42::nested_function_values {
    fun leaf(x: u64): u64 { x + 1 }
    spec leaf {
        aborts_if x + 1 > MAX_U64;
        ensures result == x + 1;
    }

    fun step(f: |u64| u64 has copy + drop, x: u64): u64 { f(x) }

    fun mid(x: u64): u64 { step(leaf, x) }

    // Applications of one function type nested through two targets.
    fun nested(): u64 {
        let c: |u64| u64 has copy + drop = mid;
        c(1)
    }
    spec nested { ensures result == 2; }

    // The same higher-order function at two levels.
    fun top(x: u64): u64 { step(mid, x) }

    fun twice(): u64 { top(1) }
    spec twice { ensures result == 2; }

    // FALSE: top(1) == 2.
    fun twice_fails(): u64 { top(1) }
    spec twice_fails { ensures result == 3; }
}

// A function value passed in a struct field counts toward the inlining depth like a function
// parameter.
module 0x42::struct_field_function_values {
    struct S has copy, drop { f: |u64| u64 has copy + drop }

    fun leaf(x: u64): u64 { x + 1 }
    spec leaf {
        aborts_if x + 1 > MAX_U64;
        ensures result == x + 1;
    }

    fun step(s: S, x: u64): u64 { (s.f)(x) }

    fun mid(x: u64): u64 { step(S { f: leaf }, x) }

    fun top(x: u64): u64 { step(S { f: mid }, x) }

    fun twice(): u64 { top(1) }
    spec twice { ensures result == 2; }

    // FALSE: top(1) == 2.
    fun twice_fails(): u64 { top(1) }
    spec twice_fails { ensures result == 3; }
}

// A closure of `apply` nested in itself through its captured argument, deeper than the inlining
// depth: recursion through function values, reported at `apply`.
module 0x42::closure_chain {
    fun apply(f: |u64| u64 has copy + drop, x: u64): u64 { f(x) }

    // FALSE: the result is x.
    fun root(x: u64): u64 {
        let c0: |u64| u64 has copy + drop = |y| y;
        let c1: |u64| u64 has copy + drop = |y| apply(c0, y);
        let c2: |u64| u64 has copy + drop = |y| apply(c1, y);
        let c3: |u64| u64 has copy + drop = |y| apply(c2, y);
        apply(c3, x)
    }
    spec root { ensures result == x + 1; }
}
