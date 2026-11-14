// Counterparts of `fun_param_frame_transparent` that keep within the frame. A function with an
// undeclared function-typed parameter verifies under the empty frame, and every call below is
// accepted.
module 0x42::state_ok {
    struct S has key { v: u64 }

    public fun bump() acquires S {
        let s = &mut S[@0x42];
        s.v = s.v + 1;
    }
    spec bump {
        pragma opaque;
        modifies global<S>(@0x42);
        aborts_if !exists<S>(@0x42);
        aborts_if global<S>(@0x42).v + 1 > MAX_U64;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v) + 1;
    }
}

module 0x42::fun_param_frame_transparent_ok {
    use 0x42::state_ok::{Self, S};

    fun apply(f: |u64| has drop) { f(1) }
    spec apply {
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    fun apply_r(f: |u64| u64 has drop): u64 { f(1) }
    spec apply_r {
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    fun apply_o(f: |u64| has drop) { f(1) }
    spec apply_o {
        pragma opaque;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    fun apply_s(f: |u64| has drop) { f(1) }
    spec apply_s {
        modifies_of<f>(_x: u64) S[@0x42];
    }

    fun apply_w(f: |u64| has drop) { f(1) }
    spec apply_w {
        modifies_of<f> *;
    }

    fun apply_g<T: drop>(f: |T| has drop, x: T) { f(x) }
    spec apply_g {
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    fun pure_direct(): u64 { apply_r(|x| x + 1) }
    spec pure_direct {
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
        ensures result == 2;
    }

    fun pure_generic() { apply_g(|_x: u64| (), 1) }

    fun writer_to_declared() { apply_s(|_x| state_ok::bump()) }

    fun writer_to_wild() { apply_w(|_x| state_ok::bump()) }

    // Forwarding: each forwarded frame is within the callee's.
    fun forward(f: |u64| has drop) { apply(f) }
    spec forward {
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    fun forward_to_opaque(f: |u64| has drop) { apply_o(f) }

    fun forward_framed(f: |u64| has drop) { apply_s(f) }
    spec forward_framed {
        modifies_of<f>(_x: u64) S[@0x42];
    }

    fun forward_framed_to_wild(f: |u64| has drop) { apply_w(f) }
    spec forward_framed_to_wild {
        modifies_of<f>(_x: u64) S[@0x42];
    }

    fun forward_wild_to_wild(f: |u64| has drop) { apply_w(f) }
    spec forward_wild_to_wild {
        modifies_of<f> *;
    }

    // A parameter captured by a lambda and invoked.
    fun nested(f: |u64| has drop + copy) {
        let h = |x| f(x);
        h(3)
    }
    spec nested {
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    // A pure closure built in a loop.
    fun pure_in_loop(n: u64) {
        let i = 0;
        while (i < n) {
            apply_o(|_x| ());
            apply(|_x| ());
            i = i + 1;
        }
    }
    spec pure_in_loop {
        pragma aborts_if_is_partial;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    // A pure closure through a local and through an invoke.
    fun pure_through_local(b: bool) {
        let g: |u64| has drop + copy = |_x| ();
        let g2 = g;
        if (b) { g2 = |_x| () };
        apply(g2)
    }

    fun pure_through_invoke() {
        let h: |(|u64| has drop)| has drop = |g| apply(g);
        h(|_x| ())
    }

    // A pure closure returned by a transparent function.
    fun make_adder(n: u64): |u64| u64 has drop {
        |x| x + n
    }

    fun pure_from_call(x: u64): u64 {
        apply_r(make_adder(5))
            + x
    }
    spec pure_from_call {
        pragma aborts_if_is_partial;
        ensures result == x + 6;
    }

    // A pure closure whose target calls a transparent function with a function value.
    fun apply_b(f: |u64| u64, x: u64): u64 { f(x) }
    spec apply_b {
        requires requires_of<f>(x);
        aborts_if aborts_of<f>(x);
        ensures ensures_of<f>(x, result);
    }

    fun nested_pure(x: u64): u64 {
        apply_b(|y| apply_b(|z| z + 1, y), x) + 1
    }

    // A closure capturing every function-typed parameter of its target.
    fun step(f: |u64| u64 has copy + drop, x: u64): u64 { f(x) }

    fun chain(x: u64): u64 {
        let c0: |u64| u64 has copy + drop = |y| y + 1;
        let c1: |u64| u64 has copy + drop = |y| step(c0, y);
        step(c1, x)
    }
    spec chain {
        pragma aborts_if_is_partial;
        ensures result == x + 1;
    }
}
