// `result_of` over a lambda whose attached spec has no functional
// `ensures result == E` condition falls back to the value derived from the
// body: the attached spec describes conditions, not the value, and the body
// stays authoritative. (Before this fallback, such a lambda reported the
// misleading "add a spec block" error although a spec block was attached.)
module 0x42::result_of_attached_fallback {

    inline fun apply(f: |u64| u64, x: u64): u64 {
        let r = f(x);
        spec {
            assert r == result_of<f>(x);
        };
        r
    }

    // The attached ensures is an inequality, not a functional value shape;
    // `result_of` resolves to the body-derived value `y + 1`.
    fun attached_nonfunctional(x: u64): u64 {
        apply(|y| y + 1 spec { ensures result >= y; }, x)
    }
    spec attached_nonfunctional {
        requires x < MAX_U64;
        ensures result == x + 1;
    }

    inline fun apply_expect(f: |u64| u64, x: u64, expected: u64): u64 {
        let r = f(x);
        spec {
            // NEGATIVE canary: proves the fallback splices the real derived
            // value (for the caller below, `result_of<f>(x)` is `x + 1`,
            // not `expected`).
            assert result_of<f>(x) == expected; // error: for the `wrong_expectation` caller, x + 1 != x + 2
        };
        r
    }

    fun wrong_expectation(x: u64): u64 {
        apply_expect(|y| y + 1 spec { ensures result >= y; }, x, x + 2)
    }
    spec wrong_expectation {
        requires x < MAX_U64 - 2;
    }
}
