module 0x42::Test {
    inline fun apply(v: u64, predicate: |u64| bool): bool {
        spec {
            assert v >= 0;
        };
        predicate(v)
    }

    public fun test_apply(x: u64) {
        let r1 = apply(x, |v: u64| v >= 0);
        spec {
            assert r1;
        };
        assert!(r1, 1);

        let r2 = apply(x, |v: u64| v != 0);
        spec {
            assert r2;
        };
        assert!(r2, 2);
    }
}
