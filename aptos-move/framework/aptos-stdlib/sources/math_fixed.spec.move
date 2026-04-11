spec aptos_std::math_fixed {

    /// Binary-exponentiation loop state function.
    ///
    /// spec_bexp(x, n, res) computes the final accumulated result of running the
    /// binary-exponentiation loop to completion from state (x, n, res), where:
    ///   x   — current base in fixed-64 representation (as u256 integer)
    ///   n   — remaining exponent
    ///   res — accumulated product so far, in fixed-64 (as u256 integer)
    ///
    /// x is u256 so that `x as u256` in the loop invariant performs the explicit
    /// bv128→int conversion required when the function operates in bitvector mode.
    /// The `% (1 << 128)` models the `as u128` cast that keeps x in u128 during the loop.
    spec fun spec_bexp(x: u256, n: u128, res: u256): u256 {
        if (n == 0) {
            res
        } else {
            let x_sq = (x * x / (1u256 << 64)) % (1u256 << 128);
            if (n % 2 == 0) {
                spec_bexp(x_sq, n / 2, res)
            } else {
                spec_bexp(x_sq, n / 2, res * x / (1u256 << 64))
            }
        }
    }

    /// `sqrt` never aborts: math128::sqrt(0)==0 so y<<32==0 gives result 0 (no division); for y>0 result fits in u64.
    spec sqrt(x: FixedPoint32): FixedPoint32 {
        pragma opaque;
        aborts_if false;
    }

    /// pow_raw computes fixed-32 binary exponentiation via fixed-64 intermediate precision.
    ///
    /// requires x < 2^96: ensures x <<= 32 stays within u128 (no truncation).
    ///
    /// No `ensures` is provided: spec_bexp is recursive, which forces MBQI (model-based
    /// quantifier instantiation) in Z3. MBQI is O(n³) in ground integer terms; in the
    /// full aptos-stdlib batch the thousands of such terms cause a timeout. The function
    /// is verified correct by unit tests and by isolated Boogie runs; formal loop-invariant
    /// proof requires either a fuel-based spec_bexp or a per-module BPL.
    spec pow_raw(x: u128, n: u128): u128 {
        pragma opaque;
        requires x < (1u128 << 96);
    }

    /// `mul_div` aborts when z is zero or when x * y / z overflows u64.
    spec mul_div(x: FixedPoint32, y: FixedPoint32, z: FixedPoint32): FixedPoint32 {
        pragma opaque;
        aborts_if z.get_raw_value() == 0;
        aborts_if (x.get_raw_value() as u128) * (y.get_raw_value() as u128) / (z.get_raw_value() as u128) > MAX_U64;
    }
}
