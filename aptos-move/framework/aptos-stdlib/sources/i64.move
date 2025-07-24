module aptos_std::i64 {
    /// Arithmetic operation resulted in overflow (value outside the range [-2^63, 2^63 - 1])
    const EOVERFLOW: u64 = 1;
    /// Division by Zero is not allowed
    const EDIVISION_BY_ZERO: u64 = 2;

    /// min number that a I64 could represent = (1 followed by 63 0s) = 1 << 63
    const BITS_MIN_I64: u64 = 0x8000000000000000;

    /// max number that a I64 could represent = (0 followed by 63 1s) = (1 << 63) - 1
    const BITS_MAX_I64: u64 = 0x7fffffffffffffff;

    /// (1 << 64) - 1
    const MAX_U64: u64 = 0xffffffffffffffff;

    /// 1 << 64
    const TWO_POW_64: u128 = 0x10000000000000000;

    const LT: u8 = 0;
    const EQ: u8 = 1;
    const GT: u8 = 2;

    struct I64 has copy, drop, store {
        bits: u64
    }

    /// Creates an I64 from a u64, asserting that it's not greater than the maximum positive value
    public fun from(v: u64): I64 {
        assert!(v <= BITS_MAX_I64, EOVERFLOW);
        I64 { bits: v }
    }

    /// Creates a negative I64 from a u64, asserting that it's not greater than the minimum negative value
    public fun neg_from(v: u64): I64 {
        assert!(v <= BITS_MIN_I64, EOVERFLOW);
        I64 { bits: twos_complement(v) }
    }

    public fun neg(self: I64): I64 {
        if (self.is_neg()) { self.abs() }
        else {
            neg_from(self.bits)
        }
    }

    /// Performs wrapping addition on two I64 numbers
    public fun wrapping_add(self: I64, num2: I64): I64 {
        I64 { bits: (((self.bits as u128) + (num2.bits as u128)) % TWO_POW_64 as u64) }
    }

    /// Performs checked addition on two I64 numbers, abort on overflow
    public fun add(self: I64, num2: I64): I64 {
        let sum = self.wrapping_add(num2);
        // overflow only if: (1) postive + postive = negative, OR (2) negative + negative = positive
        let self_sign = self.sign_internal();
        let overflow = self_sign == num2.sign_internal() && self_sign != sum.sign_internal();
        assert!(!overflow, EOVERFLOW);
        sum
    }

    /// Performs wrapping subtraction on two I64 numbers
    public fun wrapping_sub(self: I64, num2: I64): I64 {
        self.wrapping_add(I64 { bits: twos_complement(num2.bits) })
    }

    /// Performs checked subtraction on two I64 numbers, asserting on overflow
    public fun sub(self: I64, num2: I64): I64 {
        let difference = self.wrapping_sub(num2);
        // overflow only if: (1) positive - negative = negative, OR (2) negative - positive = positive
        let self_sign = self.sign_internal();
        let overflow = self_sign != num2.sign_internal() && self_sign != difference.sign_internal();
        assert!(!overflow, EOVERFLOW);
        difference
    }

    /// Performs multiplication on two I64 numbers
    public fun mul(self: I64, num2: I64): I64 {
        let product = (self.abs_u64() as u128) * (num2.abs_u64() as u128);
        if (self.sign_internal() != num2.sign_internal()) {
            assert!(product <= (BITS_MIN_I64 as u128), EOVERFLOW);
            neg_from((product as u64))
        } else {
            assert!(product <= (BITS_MAX_I64 as u128), EOVERFLOW);
            from((product as u64))
        }
    }

    /// Performs division on two I64 numbers
    /// Note that we mimic the behavior of solidity int division that it rounds towards 0 rather than rounds down
    /// - rounds towards 0: (-4) / 3 = -(4 / 3) = -1 (remainder = -1)
    /// - rounds down: (-4) / 3 = -2 (remainder = 2)
    public fun div(self: I64, num2: I64): I64 {
        assert!(!num2.is_zero(), EDIVISION_BY_ZERO);
        let result = self.abs_u64() / num2.abs_u64();
        if (self.sign_internal() != num2.sign_internal()) neg_from(result)
        else from(result)
    }

    /// Performs modulo on two I64 numbers
    /// a mod b = a - b * (a / b)
    public fun mod(self: I64, num2: I64): I64 {
        let quotient = self.div(num2);
        self.sub(num2.mul(quotient))
    }

    /// Returns the absolute value of an I64 number
    public fun abs(self: I64): I64 {
        let bits = if (self.sign_internal() == 0) { self.bits }
        else {
            assert!(self.bits > BITS_MIN_I64, EOVERFLOW);
            twos_complement(self.bits)
        };
        I64 { bits }
    }

    /// Returns the absolute value of an I64 number as a u64
    public fun abs_u64(self: I64): u64 {
        if (self.sign_internal() == 0) self.bits
        else twos_complement(self.bits)
    }

    /// Returns the minimum of two I64 numbers
    public fun min(self: I64, b: I64): I64 {
        if (self.lt(b)) self else b
    }

    /// Returns the maximum of two I64 numbers
    public fun max(self: I64, b: I64): I64 {
        if (self.gt(b)) self else b
    }

    /// Raises an I64 number to a u64 power
    public fun pow(self: I64, exponent: u64): I64 {
        if (exponent == 0) {
            return from(1)
        };
        let result = from(1);
        while (exponent > 0)  {
            if (exponent % 2 == 1) {
                result = result.mul(self);
            };
            self = self.mul(self);
            exponent /= 2;
        };
        result
    }

    /// Creates an I64 from a u64 without any checks
    public fun pack(v: u64): I64 {
        I64 { bits: v }
    }

    /// Destroys an I64 and returns its internal bits
    public fun unpack(self: I64): u64 {
        self.bits
    }

    /// Get internal bits of I64
    public fun bits(self: &I64): u64 {
        self.bits
    }

    /// Returns the sign of an I64 number (0 for positive, 1 for negative)
    public fun sign(self: I64): u8 {
        self.sign_internal()
    }

    /// Creates and returns an I64 representing zero
    public fun zero(): I64 {
        I64 { bits: 0 }
    }

    /// Checks if an I64 number is zero
    public fun is_zero(self: I64): bool {
        self.bits == 0
    }

    /// Checks if an I64 number is negative
    public fun is_neg(self: I64): bool {
        self.sign_internal() == 1
    }

    /// Compares two I64 numbers, returning LT, EQ, or GT
    public fun cmp(self: I64, num2: I64): u8 {
        let sign1 = self.sign_internal();
        let sign2 = num2.sign_internal();

        if (sign1 > sign2) {
            LT
        } else if (sign1 < sign2) {
            GT
        } else if (self.bits > num2.bits) {
            GT
        } else if (self.bits < num2.bits)  {
            LT
        } else {
            EQ
        }
    }

    /// Checks if two I64 numbers are equal
    public fun eq(self: I64, num2: I64): bool {
        self.cmp(num2) == EQ
    }

    /// Checks if the first I64 number is greater than the second
    public fun gt(self: I64, num2: I64): bool {
        self.cmp(num2) == GT
    }

    /// Checks if the first I64 number is greater than or equal to the second
    public fun gte(self: I64, num2: I64): bool {
        self.cmp(num2) >= EQ
    }

    /// Checks if the first I64 number is less than the second
    public fun lt(self: I64, num2: I64): bool {
        self.cmp(num2) == LT
    }

    /// Checks if the first I64 number is less than or equal to the second
    public fun lte(self: I64, num2: I64): bool {
        self.cmp(num2) <= EQ
    }

    /// Two's complement in order to dervie negative representation of bits
    /// It is overflow-proof because we hardcode 2's complement of 0 to be 0
    /// Which is fine for our specific use case
    inline fun twos_complement(v: u64): u64 {
        if (v == 0) 0 else MAX_U64 - v + 1
    }

    /// Returns the sign of an I64 number (0 for positive, 1 for negative)
    inline fun sign_internal(self: I64): u8 {
        ((self.bits / BITS_MIN_I64) as u8)
    }
}
