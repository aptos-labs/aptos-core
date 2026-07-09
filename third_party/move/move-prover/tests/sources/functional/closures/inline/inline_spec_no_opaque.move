// Tests inline functions with non-opaque spec blocks: the function body is
// verified standalone against the spec, while call sites still expand the
// body as usual (callers prove through the expansion, not the spec).
module 0x42::inline_spec_no_opaque {

    inline fun inc(x: u64): u64 {
        x + 1
    }
    spec inc {
        aborts_if x + 1 > MAX_U64;
        ensures result == x + 1;
    }

    fun test_inc(x: u64): u64 {
        inc(x)
    }
    spec test_inc {
        aborts_if x + 1 > MAX_U64;
        ensures result == x + 1;
    }

    /// The caller proves a fact about the expanded body which does not follow
    /// from the spec alone, showing the call site is expanded, not opaque.
    inline fun half_range(x: u64): u64 {
        x / 2
    }
    spec half_range {
        aborts_if false;
        ensures result <= x;
    }

    fun test_expansion(x: u64): u64 {
        half_range(x)
    }
    spec test_expansion {
        ensures result == x / 2;
    }

    inline fun bad_inc(x: u64): u64 {
        x + 2
    }
    spec bad_inc {
        aborts_if x + 2 > MAX_U64;
        ensures result == x + 1; // error: post-condition does not hold (body adds 2)
    }

    /// The caller still verifies through the expanded body.
    fun test_bad_inc(x: u64): u64 {
        bad_inc(x)
    }
    spec test_bad_inc {
        aborts_if x + 2 > MAX_U64;
        ensures result == x + 2;
    }

    /// Inline function with `return`, which is compilable for standalone
    /// verification (call sites of inline functions cannot expand `return`,
    /// so it is only used where the function is not called).
    inline fun first_positive(x: u64, y: u64): u64 {
        if (x > 0) {
            return x
        };
        y
    }
    spec first_positive {
        ensures x > 0 ==> result == x;
        ensures x == 0 ==> result == y;
    }
}
