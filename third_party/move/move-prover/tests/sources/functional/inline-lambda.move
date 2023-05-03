module 0x42::Test {
    inline fun apply(v: u64, predicate: |u64| bool): bool {
        spec {
            assert v != 42;
            assert predicate(v);
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
}
