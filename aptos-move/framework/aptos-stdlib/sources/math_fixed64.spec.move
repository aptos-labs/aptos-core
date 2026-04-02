spec aptos_std::math_fixed64 {

    /// `sqrt` aborts when the input is zero (division by zero in the Newton step).
    spec sqrt(x: FixedPoint64): FixedPoint64 {
        pragma opaque;
        aborts_if x.get_raw_value() == 0;
    }

    /// `exp` aborts when the exponent exceeds the representable range (shift > 63).
    spec exp(x: FixedPoint64): FixedPoint64 {
        pragma opaque;
        aborts_if [abstract] (x.get_raw_value() as u256) / LN2 > 63;
    }

    /// `log2_plus_64` aborts when x is zero (undefined logarithm).
    spec log2_plus_64(x: FixedPoint64): FixedPoint64 {
        pragma opaque;
        aborts_if x.get_raw_value() == 0;
    }

    /// `ln_plus_32ln2` aborts when x is zero (undefined logarithm).
    spec ln_plus_32ln2(x: FixedPoint64): FixedPoint64 {
        pragma opaque;
        aborts_if x.get_raw_value() == 0;
    }

    /// `pow` abort conditions depend on the result exceeding u128, which is
    /// hard to bound in general; mark partial and acknowledge no known simple condition.
    spec pow(x: FixedPoint64, n: u64): FixedPoint64 {
        pragma opaque;
        pragma aborts_if_is_partial;
    }

    /// `mul_div` aborts when z is zero or when x * y / z overflows u128.
    spec mul_div(x: FixedPoint64, y: FixedPoint64, z: FixedPoint64): FixedPoint64 {
        pragma opaque;
        aborts_if z.get_raw_value() == 0;
        aborts_if (x.get_raw_value() as u256) * (y.get_raw_value() as u256) / (z.get_raw_value() as u256) > MAX_U128;
    }
}
