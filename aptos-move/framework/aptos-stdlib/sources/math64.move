/// Standard math utilities missing in the Move Language.
module aptos_std::math64 {

    /// Return the largest of two numbers.
    public fun max(a: u64, b: u64): u64 {
        if (a >= b) a else b
    }

    /// Return the smallest of two numbers.
    public fun min(a: u64, b: u64): u64 {
        if (a < b) a else b
    }

    /// Return the average of two.
    public fun average(a: u64, b: u64): u64 {
        if (a < b) {
            a + (b - a) / 2
        } else {
            b + (a - b) / 2
        }
    }

    spec average {
        pragma opaque;
        aborts_if false;
        ensures result == (a + b) / 2;
    }

    /// Return the value of n raised to power e
    public fun pow(n: u64, e: u64): u64 {
        if (e == 0) {
            1
        } else {
            let p = 1;
            while (e > 1) {
                if (e % 2 == 1) {
                    p = p * n;
                };
                e = e / 2;
                n = n * n;
            };
            p * n
        }
    }

    #[test]
    public entry fun test_max_64() {
        let result = max(3u64, 6u64);
        assert!(result == 6, 0);

        let result = max(15u64, 12u64);
        assert!(result == 15, 1);
    }

    #[test]
    public entry fun test_min() {
        let result = min(3u64, 6u64);
        assert!(result == 3, 0);

        let result = min(15u64, 12u64);
        assert!(result == 12, 1);
    }

    #[test]
    public entry fun test_average() {
        let result = average(3u64, 6u64);
        assert!(result == 4, 0);

        let result = average(15u64, 12u64);
        assert!(result == 13, 0);
    }

    #[test]
    public entry fun test_average_does_not_overflow() {
        let result = average(18446744073709551615, 18446744073709551615);
        assert!(result == 18446744073709551615, 0);
    }

    #[test]
    public entry fun test_pow() {
        let result = pow(10u64, 18u64);
        assert!(result == 1000000000000000000, 0);

        let result = pow(10u64, 1u64);
        assert!(result == 10, 0);

        let result = pow(10u64, 0u64);
        assert!(result == 1, 0);
    }
}
