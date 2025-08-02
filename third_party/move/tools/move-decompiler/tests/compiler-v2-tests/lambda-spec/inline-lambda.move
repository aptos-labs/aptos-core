module 0x42::Test {
    inline fun apply(v: u64, predicate: |u64| bool): bool {
        spec {
            assert v != 42;
            assert predicate(v);
        };
        predicate(v)
    }

    public fun test_apply(a1: u64, a2: u64) {
        let r1 = apply(0, |v| v >= 0 spec { ensures result == (v >= 0); });
        spec {
            assert r1;
        };

        let r2 = apply(0, |v| v != a1 + a2);
        spec {
            assert !r2;
        };
    }

    inline fun inline_1(x: u64, f: |u64|bool, g:|u64|bool, e:|u64|bool) : bool {
        let y = f(x);
        let z = g(x);
        let w = e(x);
        spec {
            assert y == (x > 2);
            assert y == f(x);
            assert z == g(x);
            assert w == e(x); // e is translated into an uninterpreted spec fun
        };
        y
    }

    fun call_inline(y: u64) {
        let z = 3 + y;
        inline_1(y, |x| x > 2, |x| x > z, |x| { while(z > 0) {let _x = x + 1;}; x > 5 });
    }
}
