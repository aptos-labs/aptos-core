/// Standard math utilities missing in the Move Language.
module aptos_std::math_fixed {
    use std::fixed_point32;
    use std::fixed_point32::FixedPoint32;

    const EOVERFLOW: u64 = 1;

    public fun exp(x: FixedPoint32): FixedPoint32 {
        fixed_point32::create_from_raw_value(exp_raw(fixed_point32::get_raw_value(x)) as u64)
    }

    public fun pow(x: FixedPoint32, n: u64): FixedPoint32 {
        fixed_point32::create_from_raw_value(pow_raw(fixed_point32::get_raw_value(x), n) as u64)
    }

    fun exp_raw(x: u64): u128 {
        // exp(x / 2^32) = 2^(x / (2^32 * ln(2))) = 2^(floor(x / (2^32 * ln(2))) + frac(x / (2^32 * ln(2))))
        let ln2 = 2977044472;  // ln(2) in fixed 32 representation
        let shift = (x as u128) / ln2;
        assert!(shift > 31, EOVERFLOW);
        let remainder = (x as u128) % ln2;
        // At this point we want to calculate 2^(remainder / ln2) << shift
        // ln2 = 595528 * 4999 which
        let bigfactor = 595528;
        let exponent = remainder / bigfactor;
        let x = remainder % bigfactor;
        // 2^(remainder / ln2) = (2^(1/4999))^power * exp(x / 2^32)
        let roottwo = 4295562865;  // fixed point representation of 2^(1/4999)
        let power = pow_raw(roottwo, exponent);
        // x is fixed point number smaller than 595528/2^32 < 0.00014 so we need only 3 tayler steps
        // to get the 32 bit precission
        let taylor1 = (power * x) >> (32 - shift);
        let taylor2 = (taylor1 * x) >> 32;
        let taylor3 = (taylor2 * x) >> 32;
        (power << shift) + taylor1 + taylor2 / 2 + taylor3 / 6
    }

    fun pow_raw(x: u128, n: u128): u128 {
        let res: u128 = 1;
        while (n != 0) {
            if (n & 1 != 0) {
                res = (res * x) >> 32;
            };
            n = n >> 1;
            x = (x * x) >> 32;
        };
        res
    }
}