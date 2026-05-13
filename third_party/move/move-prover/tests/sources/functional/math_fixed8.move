/// Small fixed-point math for formal verification purposes.
/// Uses 4-bit fractional precision: raw value r represents r / 16.
///
/// This module mirrors math_fixed but with types small enough that
/// pow_raw can be fully verified by loop unrolling (pragma unroll = 4).
/// The exponent n is bounded by LN2 - 1 = 10 (fits in 4 bits), so
/// the binary-exponentiation loop runs at most 4 iterations.
module 0x42::math_fixed8 {

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

    /// pow_raw correctly implements 4-bit fixed-point binary exponentiation.
    ///
    /// pragma unroll = 4: sufficient because n ≤ 10 < 2^4, so the loop runs
    /// at most 4 iterations.
    ///
    /// requires x <= 32: prevents u64 overflow in intermediate multiplications.
    ///   Starting from x_0 <= 32, after 4 squarings: x_4 <= 1048576;
    ///   all products res*x_i and x_i*x_i remain well below 2^63.
    ///
    /// The prelude roundtrip axiom `bv2int(int2bv(n)) == n` lets Z3 evaluate
    /// `n & 1` as pure integer arithmetic (`n mod 2`) without crossing theory
    /// boundaries, making the combined int+bitvector VC tractable.
    spec pow_raw(x: u64, n: u8): u64 {
        pragma opaque;
        pragma unroll = 4;
        pragma verify_duration_estimate = 60;
        requires n <= 10;
        requires x <= 32;
        aborts_if false;
        ensures result == spec_pow_raw(x, n);
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

    /// exp_raw aborts iff the computed shift exceeds 2 (result would not fit in u8).
    /// The call pow_raw(ROOTTWO=17, remainder) satisfies ROOTTWO <= 32 and
    /// remainder <= 10, so pow_raw never aborts.
    spec exp_raw(x: u8): u8 {
        pragma opaque;
        pragma verify_duration_estimate = 60;
        aborts_if x / LN2 > 3;
    }

    /// Compute an approximation of e^x for a 4-bit fixed-point input x.
    /// Aborts when x exceeds the representable range (shift > 3).
    public fun exp(x: u8): u8 {
        exp_raw(x)
    }

    /// exp aborts iff the input exceeds the representable range.
    spec exp(x: u8): u8 {
        pragma opaque;
        pragma verify_duration_estimate = 60;
        aborts_if x / LN2 > 3;
    }

    /// Unrolled reference implementation of pow_raw for n in [0, 10].
    ///
    /// Traces the exact result of the binary-exponentiation algorithm:
    ///   res = 16; while n != 0 { if n&1: res=(res*x)>>4; n>>=1; x=(x*x)>>4 }
    ///
    /// Uses a nested let structure so intermediate values (x2, x4, x8) are
    /// shared across branches, and all >> operators have explicit parentheses
    /// to avoid Move's precedence rule (* binds tighter than >>).
    ///
    /// Spec arithmetic is mathematical (unbounded integers), so no overflow.
    spec fun spec_pow_raw(x: u64, n: u8): u64 {
        if (n == 0) {
            // loop body never executes: result = 16
            16
        } else if (n == 1) {
            // iter 1: res = (16*x)>>4 = x
            x
        } else {
            let x2 = (x * x) >> 4;          // = x^2 in fixed-4
            if (n == 2) {
                // iter 1: skip; x1=x2. iter 2: res = (16*x2)>>4 = x2
                x2
            } else if (n == 3) {
                // iter 1: res=x; x1=x2. iter 2: res=(x*x2)>>4
                (x * x2) >> 4
            } else {
                let x4 = (x2 * x2) >> 4;    // = x^4 in fixed-4
                if (n == 4) {
                    // 3 skips then res = x4
                    x4
                } else if (n == 5) {
                    // res=x; skip; x4; res=(x*x4)>>4
                    (x * x4) >> 4
                } else if (n == 6) {
                    // skip; res=x2; x4; res=(x2*x4)>>4
                    (x2 * x4) >> 4
                } else if (n == 7) {
                    let x3 = (x * x2) >> 4; // = x^3 in fixed-4
                    // res=x; res=(x*x2)>>4=x3; x4; res=(x3*x4)>>4
                    (x3 * x4) >> 4
                } else {
                    let x8 = (x4 * x4) >> 4; // = x^8 in fixed-4
                    if (n == 8) {
                        // 3 skips then res = x8
                        x8
                    } else if (n == 9) {
                        // res=x; 2 skips; x8; res=(x*x8)>>4
                        (x * x8) >> 4
                    } else if (n == 10) {
                        // skip; res=x2; skip; x8; res=(x2*x8)>>4
                        (x2 * x8) >> 4
                    } else {
                        // n > 10: outside supported range
                        0
                    }
                }
            }
        }
    }

}
