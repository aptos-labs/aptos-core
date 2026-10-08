// As `lambda_declared_frame_havoc`, but the closure is bound to a local before it is
// passed. The call site still has to havoc what the closure writes, and the compile-time
// compliance check must not reject the call: a local holding a closure is otherwise
// over-approximated by the caller's whole specification footprint.
module 0x42::lambda_let_bound_frame_havoc {
    struct Counter has key { n: u64 }
    struct Box has key { v: u64 }

    /// Generic over its callers: declares no frame for `f` and constrains no global
    /// memory of its own.
    public fun apply(b: &mut Box, f: |&mut u64|) {
        f(&mut b.v)
    }
    spec apply {
        pragma opaque;
        pragma verify = false;
    }

    fun bump(a: address) acquires Counter {
        Counter[a].n = Counter[a].n + 1;
    }
    spec bump {
        pragma opaque;
        aborts_if !exists<Counter>(a);
        aborts_if global<Counter>(a).n + 1 > MAX_U64;
        modifies global<Counter>(a);
        ensures global<Counter>(a).n == old(global<Counter>(a).n) + 1;
    }

    /// Must fail: the callback writes `Counter` through the opaque call.
    public fun claims_unchanged(b: &mut Box, a: address) acquires Counter {
        let f = |v| {
            *v = *v + 1;
            bump(a);
        };
        apply(b, f)
    }
    spec claims_unchanged {
        pragma aborts_if_is_partial = true;
        ensures global<Counter>(a).n == old(global<Counter>(a).n);
    }
}
