// Forward-only chains: an effectful function value passes through a transparent function
// whose function-typed parameter declares no frame, on its way to a parameter that does.
// The undeclared parameter has the empty frame, so each value is rejected where it reaches
// it, as formally required. None of the intermediates claims anything, so no false claim
// was being proved here: the rejection is the completeness cost of enforcing undeclared
// frames at every use. They were split out of `loop_memory_havoc` and
// `closure_requires_guarded`, which keep in-frame variants of the same chains (every function
// on the chain declares the frame) so that those files still reach verification.

// From `loop_memory_havoc`: a Counter writer reaches `make_curry`'s and `fwd_outer`'s
// undeclared `f` on its way to a declared-Counter sink.
module 0x42::loop_memory_havoc_forward_chain {
    struct Counter has key { x: u64 }

    // A curried closure of a different type from the captured callback:
    // the closure scan looks up the closed-over function's usage summary,
    // which must include the forwarded `modifies_of<f>` frame via
    // `invoke_frame` transitively.
    #[persistent]
    fun tick_at(a: address) acquires Counter {
        if (exists<Counter>(a) && Counter[a].x < 18446744073709551615) {
            Counter[a].x = Counter[a].x + 1;
        }
    }
    spec tick_at {
        pragma opaque;
        modifies Counter[a];
        aborts_if false;
    }
    fun curry_helper(f: |address| has drop + copy, a: address) { f(a); }
    spec curry_helper {
        pragma aborts_if_is_partial;
        modifies_of<f>(x: address) Counter[x];
    }
    #[persistent]
    fun curry_wrap(f: |address| has drop + copy, a: address, _n: u64) {
        curry_helper(f, a);
    }
    fun make_curry(f: |address| has drop + copy, a: address): |u64| has drop + copy {
        |n| curry_wrap(f, a, n)
    }
    fun fv_curry_fails(a: address, n: u64) {
        let g = make_curry(tick_at, a);
        let i = 0;
        while (i < n) {
            g(i);
            i = i + 1;
        };
    }
    spec fv_curry_fails {
        requires exists<Counter>(a);
        ensures Counter[a].x == old(Counter[a].x);
    }

    // A forwarding helper whose only effect is invoking its declared
    // parameter: the helper's `modifies_of<f>` frame must propagate through
    // its `modified` summary so that a caller's loop invoking the helper
    // still havocs the declared memory. Verified transitively (depth 2).
    fun fwd_inner(f: |&signer| has drop + copy, s: &signer) { f(s); }
    spec fwd_inner {
        modifies_of<f>(x: signer) Counter[std::signer::address_of(x)];
    }
    fun fwd_outer(f: |&signer| has drop + copy, s: &signer) { fwd_inner(f, s); }

    fun fv_forward_depth2_fails(f: |&signer| has drop + copy, s: &signer, n: u64) {
        let i = 0;
        while (i < n) {
            fwd_outer(f, s);
            i = i + 1;
        };
    }
    spec fv_forward_depth2_fails {
        modifies_of<f>(x: signer) Counter[std::signer::address_of(x)];
        requires exists<Counter>(std::signer::address_of(s));
        ensures Counter[std::signer::address_of(s)].x
            == old(Counter[std::signer::address_of(s)].x);
    }
}

// From `closure_requires_guarded`: a reader of R is captured into transparent `hof`'s
// undeclared `f`. `hof`'s claim concerns a single invocation and is not false.
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
