// An undeclared function-typed parameter has the empty frame, whether its function is opaque
// or not: the function is verified assuming the parameter accesses no memory. Every place a
// function value reaches such a parameter must therefore keep within it. Each client below
// passes a writer of `S` to a parameter whose frame does not admit it, and must be rejected.
// Every client also claims `S` is unchanged, which is false at runtime.
//
// Accepted counterparts are in `fun_param_frame_transparent_ok`.
module 0x42::state {
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

module 0x42::fun_param_frame_transparent {
    use 0x42::state::{Self, S};

    // Transparent, undeclared frame: verified assuming `f` writes nothing.
    fun apply(f: |u64| has drop) { f(1) }
    spec apply {
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    fun apply_o(f: |u64| has drop) { f(1) }
    spec apply_o {
        pragma opaque;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    fun apply_o2(f: |u64| has drop + copy, x: u64) { f(x) }
    spec apply_o2 {
        pragma opaque;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    fun apply_g<T: drop>(f: |T| has drop, x: T) { f(x) }
    spec apply_g {
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    fun apply_s(f: |u64| has drop) { f(1) }
    spec apply_s {
        modifies_of<f>(_x: u64) S[@0x42];
    }

    fun inner_s(g: |u64| has drop + copy, x: u64) { g(x) }
    spec inner_s {
        modifies_of<g>(_x: u64) S[@0x42];
    }

    // A writer passed directly to a transparent function.
    fun direct() { apply(|_x| state::bump()) }
    spec direct {
        pragma aborts_if_is_partial;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    // A generic transparent function.
    fun direct_generic() { apply_g(|_x: u64| state::bump(), 1) }
    spec direct_generic {
        pragma aborts_if_is_partial;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    // `forward` is accepted: its own undeclared frame is within `apply_o`'s. The writer is
    // rejected at the call to `forward`.
    fun forward(g: |u64| has drop) { apply_o(g) }

    fun through_forward() { forward(|_x| state::bump()) }
    spec through_forward {
        pragma aborts_if_is_partial;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    // A loop calling a function that passes a writer to a transparent function.
    fun passes_writer() { apply(|_x| state::bump()) }

    fun loop_over_passer(n: u64) {
        let i = 0;
        while (i < n) {
            passes_writer();
            i = i + 1;
        }
    }
    spec loop_over_passer {
        pragma aborts_if_is_partial;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    // A writer built inside a loop: the loop head gives its temporary a second definition.
    fun built_in_loop(n: u64) {
        let i = 0;
        while (i < n) {
            apply_o(|_x| state::bump());
            i = i + 1;
        }
    }
    spec built_in_loop {
        pragma aborts_if_is_partial;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    // A writer captured by a lambda that passes it on: the capture binds the lambda's
    // undeclared parameter.
    fun captured() {
        let g: |u64| has drop + copy = |_z| state::bump();
        let h = |y| apply_o2(g, y);
        h(1)
    }
    spec captured {
        pragma aborts_if_is_partial;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    // A writer passed as the argument of an invoked function value.
    fun through_invoke() {
        let h: |(|u64| has drop)| has drop = |g| apply(g);
        h(|_x| state::bump())
    }
    spec through_invoke {
        pragma aborts_if_is_partial;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    // A writer copied through a local, reassigned on one path.
    fun through_local(b: bool) {
        let g: |u64| has drop + copy = |_x| state::bump();
        let g2 = g;
        if (b) { g2 = |_x| () };
        apply(g2)
    }
    spec through_local {
        pragma aborts_if_is_partial;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    // A writer returned by a function: its effects are not known here.
    fun make_writer(): |u64| has drop { |_x| state::bump() }

    fun from_call() { apply(make_writer()) }
    spec from_call {
        pragma aborts_if_is_partial;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    // A function value returned as the callee's own parameter, not a closure it builds.
    fun pass_through(f: |u64| has drop): |u64| has drop { f }

    fun from_pass_through() { apply(pass_through(|_x| ())) }

    // A closure returned by an opaque function: its body is not the code that runs.
    fun make_opaque(): |u64| has drop { |_x| () }
    spec make_opaque {
        pragma opaque;
    }

    fun from_opaque_call() { apply(make_opaque()) }

    // A closure whose target invokes a local function value, which nothing bounds.
    fun invokes_local() {
        let g: |u64| has drop = |_x| state::bump();
        g(1)
    }
    spec invokes_local {
        pragma opaque;
        modifies global<S>(@0x42);
        aborts_if !exists<S>(@0x42);
        aborts_if global<S>(@0x42).v + 1 > MAX_U64;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v) + 1;
    }

    fun over_local_invoke() { apply_o(|_x| invokes_local()) }
    spec over_local_invoke {
        pragma aborts_if_is_partial;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    // A closure leaving a function-typed parameter of its target to the invoker.
    fun step2(f: |u64| has drop, x: u64) { f(x) }

    fun uncaptured_fun_param() {
        let h: |(|u64| has drop)| has drop = |f| step2(f, 1);
        apply_hof(h)
    }

    fun apply_hof(h: |(|u64| has drop)| has drop) { h(|_x| ()) }

    // A closure whose target takes a function value: its effects are not known.
    fun over_hof() {
        let g: |u64| has drop + copy = |_x| state::bump();
        apply(|x| inner_s(g, x))
    }
    spec over_hof {
        pragma aborts_if_is_partial;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    // A closure whose target writes `S` only through a function with a declared frame.
    fun via_declared() { apply_s(|_x| state::bump()) }
    spec via_declared {
        pragma opaque;
        modifies global<S>(@0x42);
        aborts_if !exists<S>(@0x42);
        aborts_if global<S>(@0x42).v + 1 > MAX_U64;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v) + 1;
    }

    fun via_declared_to_opaque() { apply_o(|_x| via_declared()) }
    spec via_declared_to_opaque {
        pragma aborts_if_is_partial;
        ensures global<S>(@0x42).v == old(global<S>(@0x42).v);
    }

    // Parameters forwarded with a frame wider than the callee's.
    fun forward_framed(f: |u64| has drop) { apply(f) }
    spec forward_framed {
        modifies_of<f>(_x: u64) S[@0x42];
    }

    fun forward_wild(f: |u64| has drop) { apply(f) }
    spec forward_wild {
        modifies_of<f> *;
    }
}
