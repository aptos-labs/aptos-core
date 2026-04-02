spec aptos_std::math_fixed8 {

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

    /// exp_raw aborts iff the computed shift exceeds 2 (result would not fit in u8).
    /// The call pow_raw(ROOTTWO=17, remainder) satisfies ROOTTWO <= 32 and
    /// remainder <= 10, so pow_raw never aborts.
    spec exp_raw(x: u8): u8 {
        pragma opaque;
        aborts_if x / LN2 > 3;
    }

    /// exp aborts iff the input exceeds the representable range.
    spec exp(x: u8): u8 {
        pragma opaque;
        aborts_if x / LN2 > 3;
    }
}
