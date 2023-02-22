/// Standard math utilities missing in the Move Language.
module aptos_std::math128 {

    /// Abort value when an invalid argument is provided.
    const EINVALID_ARG: u64 = 1;

    /// Return the largest of two numbers.
    public fun max(a: u128, b: u128): u128 {
        if (a >= b) a else b
    }

    /// Return the smallest of two numbers.
    public fun min(a: u128, b: u128): u128 {
        if (a < b) a else b
    }

    /// Return the average of two.
    public fun average(a: u128, b: u128): u128 {
        if (a < b) {
            a + (b - a) / 2
        } else {
            b + (a - b) / 2
        }
    }

    /// Returns a * b / c going through u128 to prevent intermediate overflow
    public inline fun mul_div(a: u128, b: u128, c: u128): u128 {
        (((a as u256) * (b as u256) / (c as u256)) as u128)
    }

    /// Return x clamped to the interval [lower, upper].
    public fun clamp(x: u128, lower: u128, upper: u128): u128 {
        min(upper, max(lower, x))
    }

    /// Return the value of n raised to power e
    public fun pow(n: u128, e: u128): u128 {
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
    public fun floor_lg2(x: u128): u8 {
        let res = 0;
        assert!(x != 0, EINVALID_ARG);
        // Effectively the position of the most significant set bit
        if (x >= (1 << 64)) {
            x = x >> 64;
            res = res + 64;
        };
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
    public fun sqrt(x: u128): u128 {
        if (x == 0) return 0;
        // Note the plus 1 in the expression. Let n = floor_lg2(x) we have x in [2^n, 2^(n+1)> and thus the answer in
        // the half-open interval [2^(n/2), 2^((n+1)/2)>. For even n we can write this as [2^(n/2), sqrt(2) 2^(n/2)>
        // for odd n [2^((n+1)/2)/sqrt(2), 2^((n+1)/2>. For even n the left end point is integer for odd the right
        // end point is integer. If we choose as our first approximation the integer end point we have as maximum
        // relative error either (sqrt(2) - 1) or (1 - 1/sqrt(2)) both are smaller then 1/2.
        let res = 1 << ((floor_lg2(x) + 1) >> 1);
        // We use standard newton-rhapson iteration to improve the initial approximation.
        // The error term evolves as delta_i+1 = delta_i^2 / 2 (quadratic convergence).
        // It turns out that after 5 iterations the delta is smaller than 2^-64 and thus below the treshold.
        res = (res + x / res) >> 1;
        res = (res + x / res) >> 1;
        res = (res + x / res) >> 1;
        res = (res + x / res) >> 1;
        res = (res + x / res) >> 1;
        min(res, x / res)
    }

    #[test]
    public entry fun test_max() {
        let result = max(3u128, 6u128);
        assert!(result == 6, 0);

        let result = max(15u128, 12u128);
        assert!(result == 15, 1);
    }

    #[test]
    public entry fun test_min() {
        let result = min(3u128, 6u128);
        assert!(result == 3, 0);

        let result = min(15u128, 12u128);
        assert!(result == 12, 1);
    }

    #[test]
    public entry fun test_average() {
        let result = average(3u128, 6u128);
        assert!(result == 4, 0);

        let result = average(15u128, 12u128);
        assert!(result == 13, 0);
    }

    #[test]
    public entry fun test_pow() {
        let result = pow(10u128, 18u128);
        assert!(result == 1000000000000000000, 0);

        let result = pow(10u128, 1u128);
        assert!(result == 10, 0);

        let result = pow(10u128, 0u128);
        assert!(result == 1, 0);
    }

    #[test]
    public entry fun test_mul_div() {
        let tmp: u128 = 1<<127;
        assert!(mul_div(tmp,tmp,tmp) == tmp, 0);
    }

    #[test]
    public entry fun test_floor_lg2() {
        let idx: u8 = 0;
        while (idx < 128) {
            assert!(floor_lg2(1<<idx) == idx, 0);
            idx = idx + 1;
        };
        idx = 1;
        while (idx <= 128) {
            assert!(floor_lg2((((1u256<<idx) - 1) as u128)) == idx - 1, 0);
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

        let result = sqrt(1<<126);
        assert!(result == 1<<63, 0);

        let result = sqrt((((1u256 << 128) - 1) as u128));
        assert!(result == (1u128 << 64) - 1, 0);

        let result = sqrt((1u128 << 127));
        assert!(result == 13043817825332782212, 0);

        let result = sqrt((1u128 << 127) - 1);
        assert!(result == 13043817825332782212, 0);
    }
}
