module 0x42::Test {
    inline fun apply(v: u64, predicate: |u64| bool): bool {
        spec {
            assert v >= 42;
        };
        predicate(v)
    }

    public fun test_apply_correct() {
        let r1 = apply(42, |v| v >= 1);
        spec {
            assert r1;
        };

        let r2 = apply(43, |v| v <= 2);
        spec {
            assert !r2;
        };
    }

    public fun test_apply_error() {
        let r1 = apply(42, |v| v >= 1);
        spec {
            assert r1;
        };

        let r2 = apply(3, |v| v <= 2);
        spec {
            assert !r2;
        };
    }
}
