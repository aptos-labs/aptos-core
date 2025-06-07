spec aptos_std::i64 {
    spec module {
        pragma aborts_if_is_strict;
    }

    /// Interprets the I64 `bits` field as a signed integer.
    spec fun to_num(i: I64): num {
        // Compare to 2^63: if gte, value is negative
        if (i.bits >= BITS_MIN_I64) {
            // Interpret bits as two's complement negative number
            (i.bits as num) - TWO_POW_64
        } else {
            (i.bits as num)
        }
    }

    spec from {
        aborts_if v > BITS_MAX_I64 with EOVERFLOW;
    }

    spec neg_from {
        aborts_if v > BITS_MIN_I64 with EOVERFLOW;

        // from(v) + neg_from(v) == 0
        ensures is_zero(add(result, from(v)));
    }

    spec neg {
        // Abort if neg_from would overflow
        aborts_if !is_neg(v) && v.bits > BITS_MIN_I64 with EOVERFLOW;

        // Abort if abs(v) would overflow (MIN_I164 cannot be negated)
        aborts_if is_neg(v) && v.bits == BITS_MIN_I64 with EOVERFLOW;

        // Mathematical behavior
        ensures eq(result, mul(v, neg_from(1)));

        // Involution: neg(neg(v)) == v (if both directions do not abort)
        ensures eq(neg(result), v);
    }

    spec wrapping_add {
        ensures result.bits == (num1.bits + num2.bits) % TWO_POW_64;
    }

    spec add {
        pragma opaque;

        // Abort conditions
        // Overflow when: two positives yield negative, or two negatives yield positive
        aborts_if !is_neg(num1) && !is_neg(num2) && is_neg(wrapping_add(num1, num2)) with EOVERFLOW;
        aborts_if is_neg(num1) && is_neg(num2) && !is_neg(wrapping_add(num1, num2)) with EOVERFLOW;

        // Inverse property
        // add(a, -a) = 0
        ensures eq(abs(num1), abs(num2)) && sign(num1) != sign(num2) ==> is_zero(result);

        // Identity properties
        ensures is_zero(num2) ==> eq(result, num1);
        ensures is_zero(num1) ==> eq(result, num2);

        // Soundness: result equals num1 + num2 in num domain
        ensures to_num(result) == to_num(num1) + to_num(num2);

        ensures result == wrapping_add(num1, num2);
    }

    spec wrapping_sub {
        ensures result.bits == (num1.bits + twos_complement(num2.bits)) % TWO_POW_64;
    }

    spec sub {
        pragma opaque;

        // Function aborts if subtraction would overflow
        aborts_if !is_neg(num1) && !is_neg(from(twos_complement(num2.bits))) && is_neg(wrapping_add(num1, from(twos_complement(num2.bits)))) with EOVERFLOW;
        aborts_if is_neg(num1) &&  is_neg(from(twos_complement(num2.bits))) && !is_neg(wrapping_add(num1, from(twos_complement(num2.bits)))) with EOVERFLOW;

        // Subtracting zero returns the original number
        ensures is_zero(num1) ==> result.bits == twos_complement(num2.bits);
        ensures is_zero(num2) ==> eq(result, num1);

        // Subtracting a number from itself gives zero
        ensures eq(num1, num2) ==> is_zero(result);

        // Subtraction behaves like adding the negative in num space
        ensures to_num(result) == to_num(num1) + to_num(from(twos_complement(num2.bits)));

        ensures result == wrapping_sub(num1,  num2);
    }

    spec mul {
        // Abort conditions
        // If result should be negative (opposite signs), must not exceed abs(MIN_I64)
        aborts_if sign(num1) != sign(num2) &&
            (abs_u64(num1) as u128) * (abs_u64(num2) as u128) > (BITS_MIN_I64 as u128)
            with EOVERFLOW;

        // If result should be positive (same signs), must not exceed MAX_I64
        aborts_if sign(num1) == sign(num2) &&
            (abs_u64(num1) as u128) * (abs_u64(num2) as u128) > (BITS_MAX_I64 as u128)
            with EOVERFLOW;

        // result is positive, sign(num1) == sign(num2)
        ensures !is_neg(result) && !is_zero(result) ==> sign(num1) == sign(num2);

        // result is negative, sign(num1) != sign(num2)
        ensures is_neg(result) && !is_zero(result) ==> sign(num1) != sign(num2);

        // result is 0, num1 is zero or num2 is zero
        ensures is_zero(result) ==> is_zero(num1) || is_zero(num2);

        // Behavior guarantees
        ensures eq(result, mul(num2, num1));
        ensures to_num(result) == to_num(num1) * to_num(num2);
    }

    spec div {
        // Abort conditions
        aborts_if is_zero(num2) with EDIVISION_BY_ZERO;

        // MIN_I64 / -1 = MAX_I64 + 1, which is too big to fit in an I64
        aborts_if sign(num1) == sign(num2) && abs_u64(num1) / abs_u64(num2) > BITS_MAX_I64 with EOVERFLOW;
        aborts_if sign(num1) != sign(num2) && abs_u64(num1) / abs_u64(num2) > BITS_MIN_I64 with EOVERFLOW;

        // Behavior guarantees
        // Division result always rounds toward zero.
        // The result multiplied back gives the truncated part of num1
        ensures !is_zero(num2) ==>
            to_num(num1) == to_num(result) * to_num(num2) + to_num(mod(num1, num2));

        // Zero divided by anything is zero
        ensures is_zero(num1) ==> is_zero(result);

        // Sign correctness
        // result is positive, sign(num1) == sign(num2)
        ensures !is_neg(result) && !is_zero(result) ==> sign(num1) == sign(num2);
        // result is negative, sign(num1) != sign(num2)
        ensures is_neg(result) && !is_zero(result) ==> sign(num1) != sign(num2);

        // Always round down
        // if num1 is positive, mul(num2, result) <= num1
        ensures !is_neg(num1) ==> lte(mul(num2, result), num1);
        // if num1 is negative, mul(num2, result) >= num1
        ensures is_neg(num1) ==> gte(mul(num2, result), num1);
    }

    spec mod {
        // Abort conditions - enumerate abort cases
        aborts_with EDIVISION_BY_ZERO, EOVERFLOW;

        // Fundamental identity of mod: a mod b = a - b * (a / b)
        ensures result == wrapping_sub(num1, mul(num2, div(num1, num2)));

        // Result has the same sign as the dividend (Solidity-style behavior)
        ensures is_zero(result) || sign(result) == sign(num1);
    }

    spec abs {
        aborts_if is_neg(v) && v.bits <= BITS_MIN_I64 with EOVERFLOW;

        ensures is_neg(v) ==> is_zero(wrapping_add(abs(v), v));
        ensures is_neg(v) ==> abs(v).bits == twos_complement(v.bits);
        ensures !is_neg(v) ==> abs(v).bits == v.bits;

        ensures !is_neg(v) ==> eq(abs(v), v);
    }

    spec abs_u64 {
        aborts_if is_neg(v) && v.bits < BITS_MIN_I64 with EOVERFLOW;

        ensures is_neg(v) ==> result == twos_complement(v.bits);
        ensures !is_neg(v) ==> result == v.bits;
    }

    spec min {
        ensures to_num(a) <= to_num(b) ==> to_num(result) == to_num(a);
        ensures to_num(a) > to_num(b) ==> to_num(result) == to_num(b);
    }

    spec max {
        ensures to_num(a) >= to_num(b) ==> to_num(result) == to_num(a);
        ensures to_num(a) < to_num(b) ==> to_num(result) == to_num(b);
    }

    // ref: https://github.com/aptos-labs/aptos-core/blob/9927f302155040cc5d4efc8d16ef53f554e66a14/third_party/move/move-prover/tests/sources/functional/math8.move#L74
    spec pow {
        pragma opaque;
        // Limits to 2 unrolls of the while loop.
        // If a spec function is defined in a recursive way, when the while loop in the corresponding non-recursive
        // move function is expected to execute more than certain times, SMT solver cannot prove they are equivalent.
        pragma unroll = 2;

        // Blanket aborts with overflow if any intermediate multiplication overflows
        aborts_with EOVERFLOW;

        // Final result relationship
        ensures result == spec_pow(base, exponent);
    }

    spec fun spec_pow(n: I64, e: u64): I64 {
        if (e == 0) {
            from(1)
        }
        else {
            if (e == 1) {
                n
            }
            else {
                if (e == 2) {
                    mul(n, n)
                }
                else {
                    if (e == 3) {
                        mul(n, mul(n, n))
                    }
                    else {
                        mul(n, mul(n, mul(n, n)))
                    }
                }
            }
        }
    }

    spec sign {
        // Result must be 0 or 1 (unsigned 8-bit)
        ensures result == 0 || result == 1;

        // If the number is negative, sign is 1
        ensures is_neg(v) ==> result == 1;

        // If the number is non-negative, sign is 0
        ensures !is_neg(v) ==> result == 0;
    }

    spec zero {
        // The result must have zero bits
        ensures is_zero(result);

        // The result is not negative
        ensures !is_neg(result);

        // The result is equal to itself by to_num
        ensures to_num(result) == 0;

        // Negative zero is zero
        ensures eq(neg_from(0), zero());
        ensures eq(neg(zero()), zero());
    }

    spec is_zero {
        // Returns true iff the bit representation is 0
        ensures result == (v.bits == 0);

        // If the number is zero, to_num is 0
        ensures result ==> to_num(v) == 0;

        // If the number is not zero, to_num is non-zero
        ensures !result ==> to_num(v) != 0;
    }

    spec is_neg {
        // Directly linked to the sign function
        ensures result == (sign(v) == 1);

        // If result is true, the number is negative in two's complement
        ensures result ==> v.bits >= BITS_MIN_I64;

        // If result is false, the number is non-negative
        ensures !result ==> v.bits < BITS_MIN_I64;
    }

    spec cmp {
        // Result must be one of LT, EQ, or GT
        ensures result == LT || result == EQ || result == GT;

        // Equality case
        ensures num1.bits == num2.bits ==> result == EQ;

        // Negative vs positive
        ensures sign(num1) > sign(num2) ==> result == LT;
        ensures sign(num1) < sign(num2) ==> result == GT;

        // Same sign, different magnitude
        ensures sign(num1) == sign(num2) && num1.bits > num2.bits ==> result == GT;
        ensures sign(num1) == sign(num2) && num1.bits < num2.bits ==> result == LT;
    }

    spec eq {
        // Result is true iff both are bitwise equal
        ensures result == (num1.bits == num2.bits);

        // Equivalence with cmp
        ensures result == (cmp(num1, num2) == EQ);

        // If a = b, then b = a
        ensures eq(num1, num2) ==> eq(num2, num1);
    }

    spec gt {
        // Result is true iff cmp returns GT
        ensures result == (cmp(num1, num2) == GT);

        // If gt is true, then not equal
        ensures result ==> !eq(num1, num2);

        // If gt is true, then lt is false
        ensures gt(num1, num2) ==> lt(num2, num1);
    }

    spec gte {
        // Only returns true if num1 is equal to or greater than num2
        ensures result == (cmp(num1, num2) == EQ || cmp(num1, num2) == GT);

        // Never returns true if num1 < num2
        ensures cmp(num1, num2) == LT ==> result == false;

        // If a >= b, then b <= a
        ensures gte(num1, num2) ==> lte(num2, num1);
    }

    spec lt {
        // Only returns true if num1 is strictly less than num2
        ensures result == (cmp(num1, num2) == LT);

        // Never returns true if num1 >= num2
        ensures (cmp(num1, num2) == EQ || cmp(num1, num2) == GT) ==> result == false;

        // If a < b, then b > a
        ensures lt(num1, num2) ==> gt(num2, num1);
    }

    spec lte {
        // Only returns true if num1 is equal to or less than num2
        ensures result == (cmp(num1, num2) == EQ || cmp(num1, num2) == LT);

        // Never returns true if num1 > num2
        ensures cmp(num1, num2) == GT ==> result == false;

        // If a <= b, then b >= a
        ensures lte(num1, num2) ==> gte(num2, num1);
    }

    spec twos_complement {
        ensures v == 0 ==> result == 0;
        ensures v > 0 ==> result + v == TWO_POW_64;
    }
}
