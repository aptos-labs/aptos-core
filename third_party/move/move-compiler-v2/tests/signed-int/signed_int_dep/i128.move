/// Implements the `i128` type. The type name `i128` is a shortcut for the `struct I128` defined in this module. One can use `+`, `-`, `==`, etc. with `i128`, which are mapped to Move functions in this module.
module std::i128 {
    /// Arithmetic operation resulted in overflow (value outside the range [-2^127, 2^127 - 1])
    const EOVERFLOW: u64 = 1;
    /// Division by Zero is not allowed
    const EDIVISION_BY_ZERO: u64 = 2;

    /// min number that a I128 could represent = (1 followed by 127 0s) = 1 << 127
    const BITS_MIN_I128: u128 = 0x80000000000000000000000000000000;

    /// max number that a I128 could represent = (0 followed by 127 1s) = (1 << 127) - 1
    const BITS_MAX_I128: u128 = 0x7fffffffffffffffffffffffffffffff;

    /// (1 << 128) - 1
    const MAX_U128: u128 = 0xffffffffffffffffffffffffffffffff;

    /// 1 << 128
    const TWO_POW_128: u256 = 0x100000000000000000000000000000000;

    const LT: u8 = 0;
    const EQ: u8 = 1;
    const GT: u8 = 2;

    /// Implementation of the `i128` primitive type. Do not use this type directly, instead use `i128`.
    struct I128 has copy, drop, store {
        bits: u128
    }

    /// Creates an I128 from a u128, asserting that it's not greater than the maximum positive value
    public fun from(v: u128): I128 {
        assert!(v <= BITS_MAX_I128, EOVERFLOW);
        I128 { bits: v }
    }

    /// Creates a negative I128 from a u128, asserting that it's not greater than the minimum negative value
    public fun neg_from(v: u128): I128 {
        assert!(v <= BITS_MIN_I128, EOVERFLOW);
        I128 { bits: twos_complement(v) }
    }

    public fun neg(self: I128): I128 {
        if (self.is_neg()) { self.abs() }
        else {
            neg_from(self.bits)
        }
    }

    /// Performs wrapping addition on two I128 numbers
    public fun wrapping_add(self: I128, num2: I128): I128 {
        I128 { bits: (((self.bits as u256) + (num2.bits as u256)) % TWO_POW_128 as u128) }
    }

    /// Performs checked addition on two I128 numbers, abort on overflow
    public fun add(self: I128, num2: I128): I128 {
        let sum = self.wrapping_add(num2);
        // overflow only if: (1) postive + postive = negative, OR (2) negative + negative = positive
        let self_sign = self.sign_internal();
        let overflow = self_sign == num2.sign() && self_sign != sum.sign();
        assert!(!overflow, EOVERFLOW);
        sum
    }

    /// Performs wrapping subtraction on two I128 numbers
    public fun wrapping_sub(self: I128, num2: I128): I128 {
        self.wrapping_add(I128 { bits: twos_complement(num2.bits) })
    }

    /// Performs checked subtraction on two I128 numbers, asserting on overflow
    public fun sub(self: I128, num2: I128): I128 {
        let difference = self.wrapping_sub(num2);
        // overflow only if: (1) positive - negative = negative, OR (2) negative - positive = positive
        let self_sign = self.sign_internal();
        let overflow = self_sign != num2.sign_internal() && self_sign != difference.sign_internal();
        assert!(!overflow, EOVERFLOW);
        difference
    }

    /// Performs multiplication on two I128 numbers
    public fun mul(self: I128, num2: I128): I128 {
        let product = (self.abs_u128() as u256) * (num2.abs_u128() as u256);
        if (self.sign_internal() != num2.sign_internal()) {
            assert!(product <= (BITS_MIN_I128 as u256), EOVERFLOW);
            neg_from((product as u128))
        } else {
            assert!(product <= (BITS_MAX_I128 as u256), EOVERFLOW);
            from((product as u128))
        }
    }

    /// Performs division on two I128 numbers
    /// Note that we mimic the behavior of solidity int division that it rounds towards 0 rather than rounds down
    /// - rounds towards 0: (-4) / 3 = -(4 / 3) = -1 (remainder = -1)
    /// - rounds down: (-4) / 3 = -2 (remainder = 2)
    public fun div(self: I128, num2: I128): I128 {
        assert!(!num2.is_zero(), EDIVISION_BY_ZERO);
        let result = self.abs_u128() / num2.abs_u128();
        if (self.sign_internal() != num2.sign_internal()) neg_from(result)
        else from(result)
    }

    /// Performs modulo on two I128 numbers
    /// a mod b = a - b * (a / b)
    public fun mod(self: I128, num2: I128): I128 {
        let quotient = self.div(num2);
        self.sub(num2.mul(quotient))
    }

    /// Returns the absolute value of an I128 number
    public fun abs(self: I128): I128 {
        let bits = if (self.sign_internal() == 0) { self.bits }
        else {
            assert!(self.bits > BITS_MIN_I128, EOVERFLOW);
            twos_complement(self.bits)
        };
        I128 { bits }
    }

    /// Returns the absolute value of an I128 number as a u128
    public fun abs_u128(self: I128): u128 {
        if (self.sign_internal() == 0) self.bits
        else twos_complement(self.bits)
    }

    /// Returns the minimum of two I128 numbers
    public fun min(self: I128, b: I128): I128 {
        if (self.lt(b)) self else b
    }

    /// Returns the maximum of two I128 numbers
    public fun max(self: I128, b: I128): I128 {
        if (self.gt(b)) self else b
    }

    /// Raises an I128 number to a u64 power
    public fun pow(self: I128, exponent: u64): I128 {
        if (exponent == 0) {
            return from(1)
        };
        let result = from(1);
        while (exponent > 0) {
            if (exponent % 2 == 1) {
                result = result.mul(self);
            };
            self = self.mul(self);
            exponent /= 2;
        };
        result
    }

    /// Creates an I128 from a u128 without any checks
    public fun pack(v: u128): I128 {
        I128 { bits: v }
    }

    /// Destroys an I128 and returns its internal bits
    public fun unpack(self: I128): u128 {
        self.bits
    }

    /// Get internal bits of I128
    public fun bits(self: &I128): u128 {
        self.bits
    }

    /// Returns the sign of an I128 number (0 for positive, 1 for negative)
    public fun sign(self: I128): u8 {
        self.sign_internal()
    }

    /// Creates and returns an I128 representing zero
    public fun zero(): I128 {
        I128 { bits: 0 }
    }

    /// Checks if an I128 number is zero
    public fun is_zero(self: I128): bool {
        self.bits == 0
    }

    /// Checks if an I128 number is negative
    public fun is_neg(self: I128): bool {
        self.sign_internal() == 1
    }

    /// Compares two I128 numbers, returning LT, EQ, or GT
    public fun cmp(self: I128, num2: I128): u8 {
        let sign1 = self.sign_internal();
        let sign2 = num2.sign_internal();

        if (sign1 > sign2) {
            LT
        } else if (sign1 < sign2) {
            GT
        } else if (self.bits > num2.bits) {
            GT
        } else if (self.bits < num2.bits) {
            LT
        } else {
            EQ
        }
    }

    /// Checks if two I128 numbers are equal
    public fun eq(self: I128, num2: I128): bool {
        self.bits == num2.bits
    }

    /// Checks if two I128 numbers are not equal
    public fun neq(self: I128, num2: I128): bool {
        self.bits != num2.bits
    }

    /// Checks if the first I128 number is greater than the second
    public fun gt(self: I128, num2: I128): bool {
        self.cmp(num2) == GT
    }

    /// Checks if the first I128 number is greater than or equal to the second
    public fun gte(self: I128, num2: I128): bool {
        self.cmp(num2) >= EQ
    }

    /// Checks if the first I128 number is less than the second
    public fun lt(self: I128, num2: I128): bool {
        self.cmp(num2) == LT
    }

    /// Checks if the first I128 number is less than or equal to the second
    public fun lte(self: I128, num2: I128): bool {
        self.cmp(num2) <= EQ
    }

    /// Get the sign and the absolute value of an I128 number
    public fun into_inner(self: I128): (bool, u128) {
        (self.sign_internal() == 0, self.abs_u128())
    }

    /// Two's complement in order to dervie negative representation of bits
    /// It is overflow-proof because we hardcode 2's complement of 0 to be 0
    /// Which is fine for our specific use case
    inline fun twos_complement(v: u128): u128 {
        if (v == 0) 0 else MAX_U128 - v + 1
    }

    /// Returns the sign of an I128 number (0 for positive, 1 for negative)
    inline fun sign_internal(self: I128): u8 {
        ((self.bits / BITS_MIN_I128) as u8)
    }
}
