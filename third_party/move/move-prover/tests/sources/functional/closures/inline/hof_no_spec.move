// Inline functions with function-typed parameters carry no specs; calls are
// expanded and lambda arguments are beta-reduced. Lambdas may therefore freely
// reference and mutate locals of the enclosing scope, including through mutable
// references — which function values cannot capture. Callers verify through the
// expansion.
module 0x42::hof_no_spec {

    inline fun apply2(f: |u64|, x: u64, y: u64) {
        f(x);
        f(y);
    }

    /// The lambda mutates an enclosing local through a mutable reference.
    fun sum_two(x: u64, y: u64): u64 {
        let s = 0;
        let r = &mut s;
        apply2(|e| *r = *r + e, x, y);
        s
    }
    spec sum_two {
        aborts_if x + y > MAX_U64;
        ensures result == x + y;
    }

    /// The lambda assigns an enclosing local directly.
    fun max_two(x: u64, y: u64): u64 {
        let m = 0;
        apply2(|e| if (e > m) { m = e }, x, y);
        m
    }
    spec max_two {
        aborts_if false;
        ensures result == (if (x > y) x else y);
    }
}
