/// Standard math utilities missing in the Move Language.
module aptos_std::math_fixed {
    use std::fixed_point32;
    use std::fixed_point32::FixedPoint32;
    use aptos_std::math128;

    /// Abort code on overflow
    const EOVERFLOW: u64 = 1;

    /// Square root of fixed point number
    public fun sqrt(x: FixedPoint32): FixedPoint32 {
        let y = (fixed_point32::get_raw_value(x) as u128);
        fixed_point32::create_from_raw_value((math128::sqrt(y << 32) as u64))
    }

    /// Exponent function with a precission of 6 digits.
    public fun exp(x: FixedPoint32): FixedPoint32 {
        fixed_point32::create_from_raw_value((exp_raw((fixed_point32::get_raw_value(x) as u128)) as u64))
    }

    /// Integer power of a fixed point number
    public fun pow(x: FixedPoint32, n: u64): FixedPoint32 {
        fixed_point32::create_from_raw_value((pow_raw((fixed_point32::get_raw_value(x) as u128), (n as u128)) as u64))
    }

    /// Specialized function for x * y / z that omits intermediate shifting
    public fun mul_div(x: FixedPoint32, y: FixedPoint32, z: FixedPoint32): FixedPoint32 {
        let a = (fixed_point32::get_raw_value(x) as u128);
        let b = (fixed_point32::get_raw_value(y) as u128);
        let c = (fixed_point32::get_raw_value(z) as u128);
        fixed_point32::create_from_raw_value (((a * b / c) as u64))
    }

    fun exp_raw(x: u128): u128 {
        // exp(x / 2^32) = 2^(x / (2^32 * ln(2))) = 2^(floor(x / (2^32 * ln(2))) + frac(x / (2^32 * ln(2))))
        let ln2 = 2977044472;  // ln(2) in fixed 32 representation
        let shift_long = x / ln2;
        assert!(shift_long <= 31, EOVERFLOW);
        let shift = (shift_long as u8);
        let remainder = x % ln2;
        // At this point we want to calculate 2^(remainder / ln2) << shift
        // ln2 = 595528 * 4999 which means
        let bigfactor = 595528;
        let exponent = remainder / bigfactor;
        let x = remainder % bigfactor;
        // 2^(remainder / ln2) = (2^(1/4999))^exponent * exp(x / 2^32)
        let roottwo = 4295562865;  // fixed point representation of 2^(1/4999)
        // This has an error of 5000 / 4 10^9 roughly 6 digits of precission
        let power = pow_raw(roottwo, exponent);
        // x is fixed point number smaller than 595528/2^32 < 0.00014 so we need only 2 tayler steps
        // to get the 6 digits of precission
        let taylor1 = (power * x) >> (32 - shift);
        let taylor2 = (taylor1 * x) >> 32;
        (power << shift) + taylor1 + taylor2 / 2
    }

    fun pow_raw(x: u128, n: u128): u128 {
        let res: u128 = 1 << 32;
        while (n != 0) {
            if (n & 1 != 0) {
                res = (res * x) >> 32;
            };
            n = n >> 1;
            x = (x * x) >> 32;
        };
        res
    }

    #[test]
    public entry fun test_sqrt() {
        // We use the case of exp
        let fixed_base = 1 << 32;
        let result = sqrt(fixed_point32::create_from_u64(1));
        assert!(fixed_point32::get_raw_value(result) == fixed_base, 0);

        let result = sqrt(fixed_point32::create_from_u64(2));
        assert_approx_the_same((fixed_point32::get_raw_value(result) as u128), 6074001000);
    }

    #[test]
    public entry fun test_exp() {
        let fixed_base = 1 << 32;
        let result = exp_raw(0);
        assert!(result == fixed_base, 0);

        let result = exp_raw(fixed_base);
        let e = 11674931554;  // e in 32 bit fixed point
        assert_approx_the_same(result, e);
    }

    #[test]
    public entry fun test_pow() {
        // We use the case of exp
        let result = pow_raw(4295562865, 4999);
        assert_approx_the_same(result,  1 << 33);
    }

    #[testonly]
    fun assert_approx_the_same(x: u128, y: u128) {
        if (x < y) {
            let tmp = x;
            x = y;
            y = tmp;
        };
        assert!((x - y) * 100000 / x == 0, 0);
    }
}
