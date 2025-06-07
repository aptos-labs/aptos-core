spec aptos_std::i128 {
    spec module {
        pragma aborts_if_is_strict;
    }

    /// Interprets the I128 `bits` field as a signed integer.
    spec fun to_num(self: I128): num {
        // Compare to 2^127: if gte, value is negative
        if (self.bits >= BITS_MIN_I128) {
            // Interpret bits as two's complement negative number
            (self.bits as num) - TWO_POW_U128
        } else {
            (self.bits as num)
        }
    }

    spec from {
        aborts_if v > BITS_MAX_I128 with EOVERFLOW;
    }

    spec neg_from {
        aborts_if v > BITS_MIN_I128 with EOVERFLOW;

        // neg_from(s) == twos_complement(v)
        ensures result.bits == twos_complement(v);
    }

    spec neg {
        // Abort if neg_from would overflow
        aborts_if !self.is_neg() && self.bits > BITS_MIN_I128 with EOVERFLOW;

        // Abort if abs(self) would overflow (MIN_I128 cannot be negated)
        aborts_if self.is_neg() && self.bits == BITS_MIN_I128 with EOVERFLOW;

        // Mathematical behavior
        ensures result.eq(self.mul(neg_from(1)));

        // Involution: neg(neg(v)) == v (if both directions do not abort)
        ensures result.neg().eq(self);
    }

    spec wrapping_add {
        ensures result.bits == (self.bits + num2.bits) % TWO_POW_U128;
    }

    spec add {
        pragma opaque;

        // Abort conditions
        // Overflow when: two positives yield negative, or two negatives yield positive
        aborts_if !self.is_neg() && !num2.is_neg() && self.wrapping_add(num2).is_neg() with EOVERFLOW;
        aborts_if self.is_neg() && num2.is_neg() && !self.wrapping_add(num2).is_neg() with EOVERFLOW;

        // Inverse property
        // add(a, -a) = 0
        ensures self.abs().eq(num2.abs()) && self.sign() != num2.sign() ==> result.is_zero();

        // Identity properties
        ensures num2.is_zero() ==> result.eq(self);
        ensures self.is_zero() ==> result.eq(num2);

        ensures to_num(result) == to_num(self) + to_num(num2);

        ensures result == self.wrapping_add(num2);
    }

    spec wrapping_sub {
        ensures result.bits == (self.bits + twos_complement(num2.bits)) % TWO_POW_U128;
    }

    spec sub {
        pragma opaque;
        // Function aborts if subtraction would overflow
        aborts_if !self.is_neg() && !from(twos_complement(num2.bits)).is_neg() && self.wrapping_add(
            from(twos_complement(num2.bits))
        ).is_neg() with EOVERFLOW;
        aborts_if self.is_neg() && from(twos_complement(num2.bits)).is_neg() && !self.wrapping_add(
            from(twos_complement(num2.bits))
        ).is_neg() with EOVERFLOW;

        // Subtracting zero returns the original number
        ensures self.is_zero() ==> result.bits == twos_complement(num2.bits);
        ensures num2.is_zero() ==> result.eq(self);

        // Subtracting a number from itself gives zero
        ensures self.eq(num2) ==> result.is_zero();

        // Subtraction behaves like adding the negative in num space
        ensures to_num(result) == to_num(self) + to_num(from(twos_complement(num2.bits)));

        ensures result == self.wrapping_sub(num2);
    }

    spec mul {
        // Abort conditions
        // If result should be negative (opposite signs), must not exceed abs(MIN_I128)
        aborts_if self.sign() != num2.sign() &&
            (self.abs_u128() as u256) * (num2.abs_u128() as u256) > (BITS_MIN_I128 as u256)
            with EOVERFLOW;

        // If result should be positive (same signs), must not exceed MAX_I64
        aborts_if self.sign() == num2.sign() &&
            (self.abs_u128() as u256) * (num2.abs_u128() as u256) > (BITS_MAX_I128 as u256)
            with EOVERFLOW;

        // result is positive, sign(self) == sign(num2)
        ensures !result.is_neg() && !result.is_zero() ==> self.sign() == num2.sign();

        // result is negative, sign(self) != sign(num2)
        ensures result.is_neg() && !result.is_zero() ==> self.sign() != num2.sign();

        // result is 0, self is zero or num2 is zero
        ensures result.is_zero() ==> self.is_zero() || num2.is_zero();

        // Behavior guarantees
        ensures result.eq(num2.mul(self));
        ensures to_num(result) == to_num(self) * to_num(num2);
    }

    spec div {
        // Abort conditions
        aborts_if num2.is_zero() with EDIVISION_BY_ZERO;

        // MIN_I64 / -1 = MAX_I64 + 1, which is too big to fit in an I64
        aborts_if self.sign() == num2.sign() && self.abs_u128() / num2.abs_u128() > BITS_MAX_I128 with EOVERFLOW;
        aborts_if self.sign() != num2.sign() && self.abs_u128() / num2.abs_u128() > BITS_MIN_I128 with EOVERFLOW;

        // Behavior guarantees
        // Division result always rounds toward zero.
        // The result multiplied back gives the truncated part of self
        ensures !num2.is_zero() ==>
            to_num(self) == to_num(result) * to_num(num2) + to_num(self.mod(num2));

        // Zero divided by anything is zero
        ensures self.is_zero() ==> result.is_zero();

        // Sign correctness
        // result is positive, sign(self) == sign(num2)
        ensures !result.is_neg() && !result.is_zero() ==> self.sign() == num2.sign();
        // result is negative, sign(self) != sign(num2)
        ensures result.is_neg() && !result.is_zero() ==> self.sign() != num2.sign();

        // Always round down
        // if self is positive, mul(num2, result) <= self
        ensures !self.is_neg() ==> num2.mul(result).lte(self);
        // if self is negative, mul(num2, result) >= self
        ensures self.is_neg() ==> num2.mul(result).gte(self);
    }

    spec mod {
        // pragma timeout = 120;

        // Abort conditions - enumerate abort cases
        aborts_with EDIVISION_BY_ZERO, EOVERFLOW;

        // Fundamental identity of mod: a mod b = a - b * (a / b)
        ensures result == self.wrapping_sub(num2.mul(self.div(num2)));

        // Result has the same sign as the dividend (Solidity-style behavior)
        ensures result.is_zero() || result.sign() == self.sign();
    }

    spec abs {
        aborts_if self.is_neg() && self.bits <= BITS_MIN_I128 with EOVERFLOW;

        ensures self.is_neg() ==> self.abs().wrapping_add(self).is_zero();
        ensures self.is_neg() ==> self.abs().bits == twos_complement(self.bits);
        ensures !self.is_neg() ==> self.abs().bits == self.bits;

        ensures !self.is_neg() ==> self.abs().eq(self);
    }

    spec abs_u128 {
        aborts_if self.is_neg() && self.bits < BITS_MIN_I128 with EOVERFLOW;

        ensures self.is_neg() ==> result == twos_complement(self.bits);
        ensures !self.is_neg() ==> result == self.bits;
    }

    spec min {
        ensures to_num(self) <= to_num(b) ==> to_num(result) == to_num(self);
        ensures to_num(self) > to_num(b) ==> to_num(result) == to_num(b);
    }

    spec max {
        ensures to_num(self) >= to_num(b) ==> to_num(result) == to_num(self);
        ensures to_num(self) < to_num(b) ==> to_num(result) == to_num(b);
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
        ensures result == spec_pow(self, exponent);
    }

    spec fun spec_pow(self: I128, e: u64): I128 {
        if (e == 0) {
            from(1)
        }
        else {
            if (e == 1) {
                self
            }
            else {
                if (e == 2) {
                    self.mul(self)
                }
                else {
                    if (e == 3) {
                        self.mul(self.mul(self))
                    }
                    else {
                        self.mul(self.mul(self.mul(self)))
                    }
                }
            }
        }
    }

    spec sign {
        // Result must be 0 or 1 (unsigned 8-bit)
        ensures result == 0 || result == 1;

        // If the number is negative, sign is 1
        ensures self.is_neg() ==> result == 1;

        // If the number is non-negative, sign is 0
        ensures !self.is_neg() ==> result == 0;
    }

    spec zero {
        // The result must have zero bits
        ensures result.is_zero();

        // The result is not negative
        ensures !result.is_neg();

        // The result is equal to itself by to_num
        ensures to_num(result) == 0;

        // Negative zero is zero
        ensures neg_from(0).eq(zero());
        ensures zero().neg().eq(zero());
    }

    spec is_zero {
        // Returns true iff the bit representation is 0
        ensures result == (self.bits == 0);

        // If the number is zero, to_num is 0
        ensures result ==> to_num(self) == 0;

        // If the number is not zero, to_num is non-zero
        ensures !result ==> to_num(self) != 0;
    }

    spec is_neg {
        // Directly linked to the sign function
        ensures result == (self.sign() == 1);

        // If result is true, the number is negative in two's complement
        ensures result ==> self.bits >= BITS_MIN_I128;

        // If result is false, the number is non-negative
        ensures !result ==> self.bits < BITS_MIN_I128;
    }

    spec cmp {
        // Result must be one of LT, EQ, or GT
        ensures result == LT || result == EQ || result == GT;

        // Equality case
        ensures self.bits == num2.bits ==> result == EQ;

        // Negative vs positive
        ensures self.sign() > num2.sign() ==> result == LT;
        ensures self.sign() < num2.sign() ==> result == GT;

        // Same sign, different magnitude
        ensures self.sign() == num2.sign() && self.bits > num2.bits ==> result == GT;
        ensures self.sign() == num2.sign() && self.bits < num2.bits ==> result == LT;
    }

    spec eq {
        // Result is true iff both are bitwise equal
        ensures result == (self.bits == num2.bits);

        // Equivalence with cmp
        ensures result == (self.cmp(num2) == EQ);

        // If a = b, then b = a
        ensures self.eq(num2) ==> num2.eq(self);
    }

    spec gt {
        // Result is true iff cmp returns GT
        ensures result == (self.cmp(num2) == GT);

        // If gt is true, then not equal
        ensures result ==> !self.eq(num2);

        // If gt is true, then lt is false
        ensures self.gt(num2) ==> num2.lt(self);
    }

    spec gte {
        // Only returns true if num1 is equal to or greater than num2
        ensures result == (self.cmp(num2) == EQ || self.cmp(num2) == GT);

        // Never returns true if self < num2
        ensures self.cmp(num2) == LT ==> result == false;

        // If a >= b, then b <= a
        ensures self.gte(num2) ==> num2.lte(self);
    }

    spec lt {
        // Only returns true if num1 is strictly less than num2
        ensures result == (self.cmp(num2) == LT);

        // Never returns true if self >= num2
        ensures (self.cmp(num2) == EQ || self.cmp(num2) == GT) ==> result == false;

        // If a < b, then b > a
        ensures self.lt(num2) ==> num2.gt(self);
    }

    spec lte {
        // Only returns true if num1 is equal to or less than num2
        ensures result == (self.cmp(num2) == EQ || self.cmp(num2) == LT);

        // Never returns true if self > num2
        ensures self.cmp(num2) == GT ==> result == false;

        // If a < b, then b > a
        ensures self.lte(num2) ==> num2.gte(self);
    }

    spec twos_complement {
        ensures v == 0 ==> result == 0;
        ensures v > 0 ==> result + v == TWO_POW_U128;
    }
}
