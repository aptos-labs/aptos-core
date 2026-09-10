// A state-reading `requires` of an attached lambda spec refers to the
// application's pre-state; consuming it through `requires_of` in a plain
// spec context needs the unique-application-site anchor, exactly like
// `aborts_of`. With two application sites there is no anchor, and the
// predicate is rejected (evaluating the reads at the assertion-site state
// instead would let a false `requires_of` claim verify once memory changed
// in between).
module 0x42::requires_of_state_err {

    struct R has key { v: u64 }

    inline fun apply_twice(f: |u64| u64, a: address, x: u64): u64 {
        let r1 = f(x);
        R[a].v = R[a].v + 1;
        let r2 = f(r1);
        spec {
            assert requires_of<f>(x); // error: no unique application site to anchor the pre-state reads
        };
        r2
    }

    fun caller(a: address, x: u64): u64 {
        apply_twice(|y| y spec { requires R[a].v > 0; ensures result == y; }, a, x)
    }
    spec caller {
        requires exists<R>(a) && R[a].v == 0;
        pragma aborts_if_is_partial;
    }

    // One lexical application site inside a loop can execute repeatedly, so
    // it is not a unique dynamic state anchor.
    inline fun apply_in_loop(f: |u64| u64, x: u64): u64 {
        let i = 0;
        let result = x;
        while (i < 2) {
            result = f(result);
            i = i + 1;
        } spec {
            invariant i <= 2;
        };
        spec {
            assert requires_of<f>(x); // error: a looped application is not a unique anchor
        };
        result
    }

    fun loop_caller(a: address, x: u64): u64 {
        apply_in_loop(
            |y| y spec { requires R[a].v > 0; ensures result == y; },
            x,
        )
    }
    spec loop_caller {
        requires exists<R>(a) && R[a].v == 0;
        pragma aborts_if_is_partial;
    }

    // A regular higher-order function may invoke its argument repeatedly.
    // The one visible application of `f` below is nested in the forwarding
    // lambda, so it cannot provide a unique dynamic state anchor.
    fun repeat_twice(g: |u64| u64 has copy, x: u64): u64 {
        g(g(x))
    }

    inline fun apply_via_forwarder(f: |u64| u64, x: u64): u64 {
        let (g, _) = (|y| f(y), |y| y);
        let result = repeat_twice(g, x);
        spec {
            assert ensures_of<f>(x, result); // error: a forwarded application may repeat dynamically
        };
        result
    }

    fun forwarded_caller(a: address, x: u64): u64 {
        apply_via_forwarder(
            |y| {
                R[a].v = R[a].v + 1;
                y
            },
            x,
        )
    }
    spec forwarded_caller {
        requires exists<R>(a) && R[a].v < MAX_U64 - 1;
        pragma aborts_if_is_partial;
    }
}
