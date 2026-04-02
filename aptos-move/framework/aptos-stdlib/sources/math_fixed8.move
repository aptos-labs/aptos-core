/// Small fixed-point math for formal verification purposes.
/// Uses 4-bit fractional precision: raw value r represents r / 16.
///
/// This module mirrors math_fixed but with types small enough that
/// pow_raw can be fully verified by loop unrolling (pragma unroll = 4).
/// The exponent n is bounded by LN2 - 1 = 10 (fits in 4 bits), so
/// the binary-exponentiation loop runs at most 4 iterations.
module aptos_std::math_fixed8 {

    /// Abort code on overflow of exp.
    const EOVERFLOW_EXP: u64 = 1;

    /// Natural log 2 in 4-bit fixed point: floor(ln(2) * 16) = 11.
    const LN2: u8 = 11;

    /// 2^(1/11) in 4-bit fixed point: floor(2^(1/11) * 16) = 17.
    /// Used as the "root-two" seed for exp_raw.
    const ROOTTWO: u64 = 17;

    /// Compute x^n in 4-bit fixed point.
    /// x is a raw fixed-point value (actual value = x / 16); result likewise.
    /// Requires n <= 10 and x <= 32 (see spec).
    fun pow_raw(x: u64, n: u8): u64 {
        let res: u64 = 16; // 1.0 in fixed-4
        while (n != 0) {
            if (n & 1 != 0) {
                res = (res * x) >> 4;
            };
            n >>= 1;
            x = (x * x) >> 4;
        };
        res
    }

    /// Core of exp: computes an approximation of 2^(x/LN2) in fixed-4.
    /// Aborts if x / LN2 > 3 (result would not fit in u8).
    ///
    /// The bound 3 follows the formula: result_bits - fractional_bits - 1 = 8 - 4 - 1 = 3.
    /// power <= spec_pow_raw(17, 10) = 28, so power << 3 = 224 <= 255 (fits in u8),
    /// but power << 4 = 448 overflows u8.
    ///
    /// All intermediate values use u8 so shift type is u8 throughout,
    /// avoiding any u64->u8 downcast that would obscure the bound on shift.
    fun exp_raw(x: u8): u8 {
        let shift: u8 = x / LN2;           // in [0, 23] for x <= 255
        assert!(shift <= 3, std::error::invalid_state(EOVERFLOW_EXP));
        let remainder: u8 = x % LN2;       // in [0, 10]
        // 17 = floor(2^(1/11) * 16), the fixed-4 representation of 2^(1/11).
        // 17 <= 32 satisfies pow_raw's requires; remainder <= 10 satisfies n <= 10.
        let power = pow_raw(17, remainder);
        ((power << shift) as u8)
    }

    /// Compute an approximation of e^x for a 4-bit fixed-point input x.
    /// Aborts when x exceeds the representable range (shift > 3).
    public fun exp(x: u8): u8 {
        exp_raw(x)
    }
}
