module 0x42::math8 {

    /// Abort value when an invalid argument is provided.
    const EINVALID_ARG_FLOOR_LOG2: u64 = 1;
    const EDIVISION_BY_ZERO: u64 = 1;

    /// Return the largest of two numbers.
    public fun max(a: u8, b: u8): u8 {
        if (a >= b) a else b
    }
    spec max(a: u8, b: u8): u8 {
        aborts_if false;
        ensures a >= b ==> result == a;
        ensures a < b ==> result == b;
    }

    /// Return the smallest of two numbers.
    public fun min(a: u8, b: u8): u8 {
        if (a < b) a else b
    }
    spec min(a: u8, b: u8): u8 {
        aborts_if false;
        ensures a < b ==> result == a;
        ensures a >= b ==> result == b;
    }

    /// Return the average of two.
    public fun average(a: u8, b: u8): u8 {
        if (a < b) {
            a + (b - a) / 2
        } else {
            b + (a - b) / 2
        }
    }
    spec average(a: u8, b: u8): u8 {
        pragma opaque;
        aborts_if false;
        ensures result == (a + b) / 2;
    }

    /// Returns a * b / c going through u128 to prevent intermediate overflow
    public inline fun mul_div(a: u8, b: u8, c: u8): u8 {
        (((a as u16) * (b as u16) / (c as u16)) as u8)
    }

    /// Return x clamped to the interval [lower, upper].
    public fun clamp(x: u8, lower: u8, upper: u8): u8 {
        min(upper, max(lower, x))
    }
    spec clamp(x: u8, lower: u8, upper: u8): u8 {
        requires (lower <= upper);
        aborts_if false;
        ensures (lower <=x && x <= upper) ==> result == x;
        ensures (x < lower) ==> result == lower;
        ensures (upper < x) ==> result == upper;
    }

    /// Return the value of n raised to power e
    public fun pow(n: u8, e: u8): u8 {
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
    spec pow(n: u8, e: u8): u8 {
        pragma opaque;
        pragma unroll = 3;
        aborts_if spec_pow(n, e) > MAX_U8;
        ensures result == spec_pow(n, e);
    }

    /// Returns floor(lg2(x))
    public fun floor_log2(x: u8): u8 {
        let res = 0;
        assert!(x != 0, std::error::invalid_argument(EINVALID_ARG_FLOOR_LOG2));
        // Effectively the position of the most significant set bit
        let n = 4;
        while (n > 0) {
            if (x >= (1 << n)) {
                x = x >> n;
                res = res + n;
            };
            n = n >> 1;
        };
        res
    }
    spec floor_log2(x: u8): u8 {
        pragma unroll=2;
        pragma opaque;
        aborts_if x == 0;
        ensures spec_pow(2, result) <= x;
        ensures x < spec_pow(2, result+1);
    }

    /// Returns square root of x, precisely floor(sqrt(x))
    public fun sqrt(x: u8): u8 {
        if (x == 0) return 0;
        // Note the plus 1 in the expression. Let n = floor_lg2(x) we have x in [2^n, 2^(n+1)> and thus the answer in
        // the half-open interval [2^(n/2), 2^((n+1)/2)>. For even n we can write this as [2^(n/2), sqrt(2) 2^(n/2)>
        // for odd n [2^((n+1)/2)/sqrt(2), 2^((n+1)/2>. For even n the left end point is integer for odd the right
        // end point is integer. If we choose as our first approximation the integer end point we have as maximum
        // relative error either (sqrt(2) - 1) or (1 - 1/sqrt(2)) both are smaller then 1/2.
        let res = 1 << ((floor_log2(x) + 1) >> 1);
        // We use standard newton-rhapson iteration to improve the initial approximation.
        // The error term evolves as delta_i+1 = delta_i^2 / 2 (quadratic convergence).
        // It turns out that after 4 iterations the delta is smaller than 2^-32 and thus below the treshold.
        res = (res + x / res) >> 1;
        res = (res + x / res) >> 1;
        res = (res + x / res) >> 1;
        res = (res + x / res) >> 1;
        min(res, x / res)
    }
    spec sqrt(x: u8): u8 {
        aborts_if false;
        pragma verify_duration_estimate = 120;
        pragma unroll=3;
        ensures x > 0 ==> result * result <= x;
        ensures x > 0 ==> x < (result+1) * (result+1);
    }

    public inline fun ceil_div(x: u8, y: u8): u8 {
        // ceil_div(x, y) = floor((x + y - 1) / y) = floor((x - 1) / y) + 1
        // (x + y - 1) could spuriously overflow. so we use the later version
        if (x == 0) {
            assert!(y != 0, EDIVISION_BY_ZERO);
            0
        }
        else (x - 1) / y + 1
    }

    spec fun spec_pow_rec(n: u8, e: u8): u8 {
        if (e == 0) {
            1
        }
        else {
            n * spec_pow_rec(n, e-1)
        }
    }

    spec fun spec_pow(n: u8, e: u8): u8 {
        if (e == 0) {
            1
        }
        else {
            if (e == 1) {
                n
            }
            else {
                if (e == 2) {
                    n*n
                }
                else {
                    if (e == 3) {
                        n*n*n
                    }
                    else {
                        if (e == 4) {
                            n*n*n*n
                        }
                        else {
                            if (e == 5) {
                                n*n*n*n*n
                            }
                            else {
                                if (e == 6) {
                                    n*n*n*n*n*n
                                }
                                else {
                                    if (e == 7) {
                                        n*n*n*n*n*n*n
                                    }
                                    else {
                                        n*n*n*n*n*n*n*n
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
