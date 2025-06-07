module aptos_std::i128 {
    const EOVERFLOW: u64 = 0;
    const EDIVISION_BY_ZERO: u64 = 1;

    /// min number that a I128 could represent = (1 followed by 127 0s) = 1 << 127
    const BITS_MIN_I128: u128 = 1 << 127;

    /// max number that a I128 could represent = (0 followed by 127 1s) = (1 << 127) - 1
    const BITS_MAX_I128: u128 = 0x7fffffffffffffffffffffffffffffff;

    /// (1 << 128) - 1
    const MAX_U128: u128 = 340282366920938463463374607431768211455;

    /// 1 << 128
    const TWO_POW_U128: u256 = 340282366920938463463374607431768211456;

    const LT: u8 = 0;
    const EQ: u8 = 1;
    const GT: u8 = 2;

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

    public fun neg(v: I128): I128 {
        if (is_neg(v)) { abs(v) }
        else {
            neg_from(v.bits)
        }
    }

    /// Performs wrapping addition on two I128 numbers
    public fun wrapping_add(num1: I128, num2: I128): I128 {
        I128 { bits: (((num1.bits as u256) + (num2.bits as u256)) % TWO_POW_U128 as u128) }
    }

    /// Performs checked addition on two I128 numbers, abort on overflow
    public fun add(num1: I128, num2: I128): I128 {
        let sum = wrapping_add(num1, num2);
        // overflow only if: (1) postive + postive = negative, OR (2) negative + negative = positive
        let is_num1_neg = is_neg(num1);
        let is_num2_neg = is_neg(num2);
        let is_sum_neg = is_neg(sum);
        let overflow = (is_num1_neg && is_num2_neg && !is_sum_neg) || (!is_num1_neg && !is_num2_neg && is_sum_neg);
        assert!(!overflow, EOVERFLOW);
        sum
    }

    /// Performs wrapping subtraction on two I128 numbers
    public fun wrapping_sub(num1: I128, num2: I128): I128 {
        wrapping_add(num1, I128 { bits: twos_complement(num2.bits) })
    }

    /// Performs checked subtraction on two I128 numbers, asserting on overflow
    public fun sub(num1: I128, num2: I128): I128 {
        add(num1, I128 { bits: twos_complement(num2.bits) })
    }

    /// Performs multiplication on two I128 numbers
    public fun mul(num1: I128, num2: I128): I128 {
        let product = (abs_u128(num1) as u256) * (abs_u128(num2) as u256);
        if (sign(num1) != sign(num2)) {
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
    public fun div(num1: I128, num2: I128): I128 {
        assert!(!is_zero(num2), EDIVISION_BY_ZERO);
        let result = abs_u128(num1) / abs_u128(num2);
        if (sign(num1) != sign(num2)) neg_from(result)
        else from(result)
    }

    /// Performs modulo on two I128 numbers
    /// a mod b = a - b * (a / b)
    public fun mod(num1: I128, num2: I128): I128 {
        let quotient = div(num1, num2);
        sub(num1, mul(num2, quotient))
    }

    /// Returns the absolute value of an I128 number
    public fun abs(v: I128): I128 {
        let bits = if (sign(v) == 0) { v.bits }
        else {
            assert!(v.bits > BITS_MIN_I128, EOVERFLOW);
            twos_complement(v.bits)
        };
        I128 { bits }
    }

    /// Returns the absolute value of an I128 number as a u128
    public fun abs_u128(v: I128): u128 {
        if (sign(v) == 0) v.bits
        else twos_complement(v.bits)
    }

    /// Returns the minimum of two I128 numbers
    public fun min(a: I128, b: I128): I128 {
        if (lt(a, b)) a else b
    }

    /// Returns the maximum of two I128 numbers
    public fun max(a: I128, b: I128): I128 {
        if (gt(a, b)) a else b
    }

    /// Raises an I128 number to a u64 power
    public fun pow(base: I128, exponent: u64): I128 {
        if (exponent == 0) {
            return from(1)
        };
        let result = from(1);
        while (exponent > 0) {
            if (exponent % 2 == 1) {
                result = mul(result, base);
            };
            base = mul(base, base);
            exponent >>= 1;
        };
        result
    }

    /// Creates an I128 from a u128 without any checks
    public fun pack(v: u128): I128 {
        I128 { bits: v }
    }

    /// Get internal bits of I128
    public fun unpack(v: I128): u128 {
        v.bits
    }

    /// Returns the sign of an I128 number (0 for positive, 1 for negative)
    public fun sign(v: I128): u8 {
        ((v.bits / BITS_MIN_I128) as u8)
    }

    /// Creates and returns an I128 representing zero
    public fun zero(): I128 {
        I128 { bits: 0 }
    }

    /// Checks if an I128 number is zero
    public fun is_zero(v: I128): bool {
        v.bits == 0
    }

    /// Checks if an I128 number is negative
    public fun is_neg(v: I128): bool {
        sign(v) == 1
    }

    /// Compares two I128 numbers, returning LT, EQ, or GT
    public fun cmp(num1: I128, num2: I128): u8 {
        let sign1 = sign(num1);
        let sign2 = sign(num2);

        if (num1.bits == num2.bits) {
            EQ
        } else if (sign1 > sign2) {
            LT
        } else if (sign1 < sign2) {
            GT
        } else if (num1.bits > num2.bits) {
            GT
        } else {
            LT
        }
    }

    /// Checks if two I128 numbers are equal
    public fun eq(num1: I128, num2: I128): bool {
        cmp(num1, num2) == EQ
    }

    /// Checks if the first I128 number is greater than the second
    public fun gt(num1: I128, num2: I128): bool {
        cmp(num1, num2) == GT
    }

    /// Checks if the first I128 number is greater than or equal to the second
    public fun gte(num1: I128, num2: I128): bool {
        cmp(num1, num2) >= EQ
    }

    /// Checks if the first I128 number is less than the second
    public fun lt(num1: I128, num2: I128): bool {
        cmp(num1, num2) == LT
    }

    /// Checks if the first I128 number is less than or equal to the second
    public fun lte(num1: I128, num2: I128): bool {
        cmp(num1, num2) <= EQ
    }

    /// Two's complement in order to dervie negative representation of bits
    /// It is overflow-proof because we hardcode 2's complement of 0 to be 0
    /// Which is fine for our specific use case
    fun twos_complement(v: u128): u128 {
        if (v == 0) 0
        else MAX_U128 - v + 1
    }
}
