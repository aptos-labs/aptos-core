spec aptos_std::math_fixed64 {

    /// `sqrt` aborts when the input is zero (math128::sqrt(0)==0 causes division by zero in the Newton step).
    spec sqrt(x: FixedPoint64): FixedPoint64 {
        pragma opaque;
        aborts_if x.get_raw_value() == 0;
    }

    /// `mul_div` aborts when z is zero or when x * y / z overflows u128.
    spec mul_div(x: FixedPoint64, y: FixedPoint64, z: FixedPoint64): FixedPoint64 {
        pragma opaque;
        aborts_if z.get_raw_value() == 0;
        aborts_if (x.get_raw_value() as u256) * (y.get_raw_value() as u256) / (z.get_raw_value() as u256) > MAX_U128;
    }
}
