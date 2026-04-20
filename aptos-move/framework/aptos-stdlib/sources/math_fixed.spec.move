spec aptos_std::math_fixed {

    /// `sqrt` never aborts: math128::sqrt(0)==0 so the Newton step is skipped; for y>0 result fits in u64.
    /// No loop in the body, so callers can inline without havocing.
    spec sqrt(x: FixedPoint32): FixedPoint32 {
        aborts_if false;
    }

    /// `mul_div` aborts when z is zero or when x * y / z overflows u64.
    /// The result equals the exact arithmetic quotient.
    spec mul_div(x: FixedPoint32, y: FixedPoint32, z: FixedPoint32): FixedPoint32 {
        aborts_if z.get_raw_value() == 0;
        aborts_if (x.get_raw_value() as u128) * (y.get_raw_value() as u128) / (z.get_raw_value() as u128) > MAX_U64;
        ensures (result.get_raw_value() as u128) ==
                (x.get_raw_value() as u128) * (y.get_raw_value() as u128) / (z.get_raw_value() as u128);
    }
}
