// Function values cannot be applied in specifications; behavioral predicates
// must be used instead.
module 0x42::Test {

    inline fun apply(v: u64, predicate: |u64| bool): bool {
        spec {
            assert predicate(v); // error: function value applied in a specification
        };
        predicate(v)
    }

    public fun test_apply(a1: u64, a2: u64) {
        let r1 = apply(0, |v| v >= 0);
        spec {
            assert r1;
        };

        let r2 = apply(0, |v| v != a1 + a2);
        spec {
            assert !r2;
        };
    }

    inline fun apply2(x: u64, f: |u64| u64): u64 {
        let y = f(x);
        spec {
            assert y == f(x); // error: function value applied in a specification
        };
        y
    }

    fun test_apply2(a: u64): u64 {
        apply2(a, |x| x + 1)
    }

    inline fun for_range(n: u64, f: |u64| bool): bool {
        let all = true;
        let i = 0;
        while (i < n) {
            all = all && f(i);
            i = i + 1;
        } spec {
            invariant forall j in 0..i: f(j); // error: function value applied in a specification
        };
        all
    }

    fun test_for_range(n: u64): bool {
        for_range(n, |x| x >= 0)
    }
}
