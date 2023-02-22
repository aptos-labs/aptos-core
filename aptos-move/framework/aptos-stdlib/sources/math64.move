/// Standard math utilities missing in the Move Language.
module aptos_std::math64 {

    /// Abort value when an invalid argument is provided.
    const EINVALID_ARG: u64 = 1;

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

    /// Return x clamped to the interval [lower, upper].
    public fun clamp(x: u64, lower: u64, upper: u64): u64 {
        min(upper, max(lower, x))
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

    /// Returns floor(lg2(x))
    public fun floor_lg2(x: u64): u8 {
        let res = 0;
        assert!(x != 0, EINVALID_ARG);
        // Effectively the position of the most significant set bit
        if (x >= (1 << 32)) {
            x = x >> 32;
            res = res + 32;
        };
        if (x >= (1 << 16)) {
            x = x >> 16;
            res = res + 16;
        };
        if (x >= (1 << 8)) {
            x = x >> 8;
            res = res + 8;
        };
        if (x >= (1 << 4)) {
            x = x >> 4;
            res = res + 4;
        };
        if (x >= (1 << 2)) {
            x = x >> 2;
            res = res + 2;
        };
        if (x >= (1 << 1)) {
            res = res + 1;
        };
        res
    }

    /// Returns square root of x, precisely floor(sqrt(x))
    public fun sqrt(x: u64): u64 {
        if (x == 0) return 0;
        // Note the plus 1 in the expression. Let n = floor_lg2(x) we have x in [2^n, 2^(n+1)> and thus the answer in
        // the half-open interval [2^(n/2), 2^((n+1)/2)>. For even n we can write this as [2^(n/2), sqrt(2) 2^(n/2)>
        // for odd n [2^((n+1)/2)/sqrt(2), 2^((n+1)/2>. For even n the left end point is integer for odd the right
        // end point is integer. If we choose as our first approximation the integer end point we have as maximum
        // relative error either (sqrt(2) - 1) or (1 - 1/sqrt(2)) both are smaller then 1/2.
        let res = 1 << ((floor_lg2(x) + 1) >> 1);
        // We use standard newton-rhapson iteration to improve the initial approximation.
        // The error term evolves as delta_i+1 = delta_i^2 / 2 (quadratic convergence).
        // It turns out that after 4 iterations the delta is smaller than 2^-32 and thus below the treshold.
        res = (res + x / res) >> 1;
        res = (res + x / res) >> 1;
        res = (res + x / res) >> 1;
        res = (res + x / res) >> 1;
        min(res, x / res)
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

    #[test]
    public entry fun test_floor_lg2() {
        let idx: u8 = 0;
        while (idx < 64) {
            assert!(floor_lg2(1<<idx) == idx, 0);
            idx = idx + 1;
        };
        idx = 1;
        while (idx <= 64) {
            assert!(floor_lg2((((1u128<<idx) - 1) as u64)) == idx - 1, 0);
            idx = idx + 1;
        };
    }

    #[test]
    public entry fun test_sqrt() {
        let result = sqrt(0);
        assert!(result == 0, 0);

        let result = sqrt(1);
        assert!(result == 1, 0);

        let result = sqrt(256);
        assert!(result == 16, 0);

        let result = sqrt(1<<62);
        assert!(result == 1<<31, 0);

        let result = sqrt((((1u128 << 64) - 1) as u64));
        assert!(result == (1u64 << 32) - 1, 0);

        let result = sqrt((1u64 << 63));
        assert!(result == 3037000499, 0);

        let result = sqrt((1u64 << 63) - 1);
        assert!(result == 3037000499, 0);
    }
}
