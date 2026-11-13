// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Behavioral predicates whose definitions refer to themselves, directly, through other closure
// targets, or through inline spec functions, form a cycle of inline functions, and are
// uninterpreted. Each such module has a false claim in a function that applies the closure, so
// the claim's verification condition contains the predicates: it fails only if they are
// consistent. Predicates off such a cycle keep their definitions.

// A closure target of its own parameter type, through partial application.
module 0x42::partial_application_self_reference {
    fun leaf(x: u64): u64 { x }

    fun f(cl: |u64| u64 has copy + drop, x: u64): u64 { cl(x) }
    spec f {
        pragma aborts_if_is_partial;
        requires !requires_of<f>(cl, x);
    }

    // FALSE: the result is 1.
    fun use_f(): u64 {
        let c: |u64| u64 has copy + drop = leaf;
        let d: |u64| u64 has copy + drop = |x| f(c, x);
        d(1)
    }
    spec use_f { ensures result == 7; }
}

// A negative self-reference inside a spec function.
module 0x42::spec_function_self_reference {
    spec fun neg(g: |u64| u64, n: u64): bool { !requires_of<g>(n) }

    fun leaf(x: u64): u64 { x }

    fun f(cl: |u64| u64 has copy + drop, x: u64): u64 { cl(x) }
    spec f {
        pragma aborts_if_is_partial;
        requires neg(cl, x);
    }

    // FALSE: the result is 1.
    fun use_f(): u64 {
        let c: |u64| u64 has copy + drop = leaf;
        let d: |u64| u64 has copy + drop = |x| f(c, x);
        d(1)
    }
    spec use_f { ensures result == 7; }
}

// A postcondition that refers to itself and contradicts the axiom for `result_of`.
module 0x42::ensures_self_reference {
    fun f(x: u64): u64 { x }
    spec f {
        pragma opaque;
        aborts_if false;
        // FALSE: unprovable for `f` itself.
        ensures ensures_of<f>(x, result) && result != result_of<f>(x);
    }

    // FALSE: the result is 1.
    fun use_f(): u64 {
        let c: |u64| u64 has drop = |x| f(x);
        c(1)
    }
    spec use_f { ensures result == 7; }
}

// A positive self-reference under an existential. This `requires_of` is not inline, so it is on
// no cycle of inline functions, and keeps its definition.
module 0x42::existential_self_reference {
    fun g(x: u64): u64 { x }
    spec g {
        pragma opaque;
        requires x == 0 || (exists y: u64: y < x && requires_of<g>(y));
        aborts_if false;
        ensures result == x;
    }

    // FALSE: the result is 0.
    fun use_g(): u64 {
        let f: |u64| u64 has drop = |x| g(x);
        f(0)
    }
    spec use_g { ensures result == 7; }

    fun claim(): bool { true }
    spec claim { ensures requires_of<g>(0); }
}

// A cycle through a recursive spec function, which is not inline, is not a cycle of inline
// functions: the predicates keep their definitions.
module 0x42::recursive_spec_function_cycle {
    spec fun all_req(g: |u64| u64, n: u64): bool {
        if (n == 0) { true } else { requires_of<g>(n) && all_req(g, n - 1) }
    }

    fun leaf(x: u64): u64 { x }

    fun f(_cl: |u64| u64 has copy + drop, x: u64): u64 { x }
    spec f {
        pragma opaque;
        requires all_req(_cl, x);
        aborts_if false;
        ensures all_req(_cl, x) ==> result == x;
    }

    fun use_f(): u64 {
        let c: |u64| u64 has copy + drop = leaf;
        let d: |u64| u64 has copy + drop = |x| f(c, x);
        d(1)
    }
    spec use_f { ensures result == 1; }
}

// A reference to a predicate off the cycle keeps its definition.
module 0x42::reference_off_cycle {
    fun h(x: u64): u64 {
        assert!(x != 0, 1);
        x
    }
    spec h {
        pragma opaque;
        aborts_if x == 0;
        ensures result == x;
    }

    fun g(n: u64): u64 {
        if (n == 0) 0 else g(n - 1)
    }
    spec g {
        pragma opaque;
        requires !aborts_of<h>(n) || n == 0;
        aborts_if false;
        ensures result == 0;
    }

    fun claim(): bool { true }
    spec claim { ensures requires_of<g>(3); }

    fun use_g(): u64 {
        let c: |u64| u64 has drop = |x| g(x);
        c(3)
    }
    spec use_g { ensures result == 0; }
}
