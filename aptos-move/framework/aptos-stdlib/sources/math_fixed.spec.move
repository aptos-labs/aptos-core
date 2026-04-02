spec aptos_std::math_fixed {

    /// `sqrt` never aborts: math128::sqrt does not abort, and the result fits in u64.
    spec sqrt(x: FixedPoint32): FixedPoint32 {
        pragma opaque;
        aborts_if false;
    }

    /// `exp` aborts when the exponent exceeds the representable range (shift > 31).
    spec exp(x: FixedPoint32): FixedPoint32 {
        pragma opaque;
        aborts_if [abstract] (x.get_raw_value() as u128) / LN2 > 31;
    }

    /// `log2_plus_32` aborts when x is zero (undefined logarithm).
    spec log2_plus_32(x: FixedPoint32): FixedPoint32 {
        pragma opaque;
        aborts_if x.get_raw_value() == 0;
    }

    /// `ln_plus_32ln2` aborts when x is zero (undefined logarithm).
    spec ln_plus_32ln2(x: FixedPoint32): FixedPoint32 {
        pragma opaque;
        aborts_if x.get_raw_value() == 0;
    }

    /// `pow` abort conditions depend on the result exceeding u64, which is
    /// hard to bound in general; mark partial.
    spec pow(x: FixedPoint32, n: u64): FixedPoint32 {
        pragma opaque;
        pragma aborts_if_is_partial;
    }

    /// `mul_div` aborts when z is zero or when x * y / z overflows u64.
    spec mul_div(x: FixedPoint32, y: FixedPoint32, z: FixedPoint32): FixedPoint32 {
        pragma opaque;
        aborts_if z.get_raw_value() == 0;
        aborts_if (x.get_raw_value() as u128) * (y.get_raw_value() as u128) / (z.get_raw_value() as u128) > MAX_U64;
    }
}
