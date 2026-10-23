// A closure built inside a loop cannot have its frame deferred, and the reason is the
// loop-exit path rather than the call itself. The call-site havoc derived from a footprint
// covers the path through the loop body -- the same claim asserted right after the call is
// refuted -- but a claim made after the loop is checked against the loop-exit path, whose
// memory havoc comes from the callee's usage summary. That summary is empty exactly when the
// callee declares no frame, so the write made through the parameter is invisible there.
//
// A loop head havocs the temporary holding the closure, giving it a second definition and so
// no footprint, which is what routes this to the rejection below.
module 0x42::lambda_loop_frame_not_deferred {
    struct Counter has key { n: u64 }
    struct Box has key { v: u64 }

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

    /// Must be rejected: the post-loop claim is false, and nothing on the exit path accounts
    /// for what the closure wrote.
    public fun claims_unchanged_after_loop(
        b: &mut Box, a: address, n: u64
    ) acquires Counter {
        let i = 0;
        while (i < n) {
            apply(b, |v| {
                *v = *v + 1;
                bump(a);
            });
            i = i + 1;
        };
    }
    spec claims_unchanged_after_loop {
        pragma aborts_if_is_partial = true;
        ensures global<Counter>(a).n == old(global<Counter>(a).n);
    }
}
