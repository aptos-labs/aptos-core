// The one-application successor denoted by `fun_post_of` is deterministic in
// `(f, args)` only when no variant of the fun type touches global memory: a
// chain evaluates all steps at one memory snapshot, while the applications it
// describes thread distinct intermediate memories. Function values whose
// declared accesses (`reads_of`) are non-empty are rejected.
module 0x42::opaque_inline_fun_post_of_memory_fail {

    struct R has key {
        value: u64,
    }

    inline fun call_memory(f: |u64| has drop) {
        f(1)
    }
    spec call_memory {
        pragma opaque;
        pragma verify = false;
        reads_of<f> R;
        ensures f == fun_post_of<old(f)>(1); // error: `fun_post_of` over a function value whose behavior depends on global memory is not supported
    }

    /// Test: the lambda reads global memory (declared via `reads_of`), so the
    /// fun type's successor is not deterministic in `(f, args)`.
    fun test_memory_dependent(): u64 {
        let x = 0;
        call_memory(|i| x = x + i + R[@0x1].value spec {
            ensures x == old(x) + i + global<R>(@0x1).value;
        });
        x
    }
}
