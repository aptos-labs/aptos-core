/// 256-bit multiply-then-divide, plus the bit scans the tick bitmap and the
/// tick math both need.
module bench::clmm_full_math {
    /// Divisor was zero.
    const EDIVIDE_BY_ZERO: u64 = 1;
    /// `most_significant_bit` / `least_significant_bit` of zero is undefined.
    const EZERO_HAS_NO_BIT: u64 = 2;

    const MAX_U256: u256 =
        115792089237316195423570985008687907853269984665640564039457584007913129639935;

    /// `a * b / denominator`, truncated. The product is kept in `u256`, so the
    /// call aborts when `a * b` does not fit rather than losing the high half.
    public fun mul_div(a: u256, b: u256, denominator: u256): u256 {
        assert!(denominator != 0, EDIVIDE_BY_ZERO);
        (a * b) / denominator
    }

    /// `mul_div` rounded away from zero.
    public fun mul_div_rounding_up(a: u256, b: u256, denominator: u256): u256 {
        assert!(denominator != 0, EDIVIDE_BY_ZERO);
        let product = a * b;
        let quotient = product / denominator;
        if (product % denominator != 0) {
            quotient + 1
        } else {
            quotient
        }
    }

    /// `a / denominator` rounded away from zero.
    public fun div_rounding_up(a: u256, denominator: u256): u256 {
        assert!(denominator != 0, EDIVIDE_BY_ZERO);
        let quotient = a / denominator;
        if (a % denominator != 0) {
            quotient + 1
        } else {
            quotient
        }
    }

    /// Index of the highest set bit, counting from zero.
    public fun most_significant_bit(x: u256): u8 {
        assert!(x != 0, EZERO_HAS_NO_BIT);
        let bit = 0u8;
        if (x >= (1u256 << 128)) {
            x = x >> 128;
            bit = bit + 128;
        };
        if (x >= (1u256 << 64)) {
            x = x >> 64;
            bit = bit + 64;
        };
        if (x >= (1u256 << 32)) {
            x = x >> 32;
            bit = bit + 32;
        };
        if (x >= (1u256 << 16)) {
            x = x >> 16;
            bit = bit + 16;
        };
        if (x >= (1u256 << 8)) {
            x = x >> 8;
            bit = bit + 8;
        };
        if (x >= (1u256 << 4)) {
            x = x >> 4;
            bit = bit + 4;
        };
        if (x >= (1u256 << 2)) {
            x = x >> 2;
            bit = bit + 2;
        };
        if (x >= 2) {
            bit = bit + 1;
        };
        bit
    }

    /// Index of the lowest set bit, counting from zero.
    public fun least_significant_bit(x: u256): u8 {
        assert!(x != 0, EZERO_HAS_NO_BIT);
        let bit = 0u8;
        if (x & ((1u256 << 128) - 1) == 0) {
            x = x >> 128;
            bit = bit + 128;
        };
        if (x & ((1u256 << 64) - 1) == 0) {
            x = x >> 64;
            bit = bit + 64;
        };
        if (x & ((1u256 << 32) - 1) == 0) {
            x = x >> 32;
            bit = bit + 32;
        };
        if (x & ((1u256 << 16) - 1) == 0) {
            x = x >> 16;
            bit = bit + 16;
        };
        if (x & 255 == 0) {
            x = x >> 8;
            bit = bit + 8;
        };
        if (x & 15 == 0) {
            x = x >> 4;
            bit = bit + 4;
        };
        if (x & 3 == 0) {
            x = x >> 2;
            bit = bit + 2;
        };
        if (x & 1 == 0) {
            bit = bit + 1;
        };
        bit
    }

    public fun max_u256(): u256 {
        MAX_U256
    }

    #[test_only]
    const MAX_U128: u256 = 340282366920938463463374607431768211455;

    #[test]
    fun test_mul_div_basic() {
        assert!(mul_div(6, 7, 2) == 21, 0);
        assert!(mul_div(0, 12345, 7) == 0, 0);
        assert!(mul_div(1, 1, 1) == 1, 0);
    }

    // Largest product that still fits: (2^128 - 1)^2 == 2^256 - 2^129 + 1.
    #[test]
    fun test_mul_div_at_u256_boundary() {
        let product = MAX_U128 * MAX_U128;
        assert!(mul_div(MAX_U128, MAX_U128, 1) == product, 0);
        assert!(mul_div(MAX_U256, 1, 1) == MAX_U256, 0);
        assert!(mul_div(MAX_U256, 1, MAX_U256) == 1, 0);
        assert!(mul_div(MAX_U256, 0, MAX_U256) == 0, 0);
        assert!(mul_div(MAX_U256, 1, 2) == (MAX_U256 - 1) / 2, 0);
    }

    #[test]
    fun test_mul_div_rounding_up_at_u256_boundary() {
        assert!(mul_div_rounding_up(MAX_U256, 1, 1) == MAX_U256, 0);
        assert!(mul_div_rounding_up(MAX_U256, 1, MAX_U256) == 1, 0);
        assert!(mul_div_rounding_up(MAX_U128, MAX_U128, 1) == MAX_U128 * MAX_U128, 0);
    }

    // Rounding up adds exactly one whenever the division leaves a remainder,
    // and nothing at all when it divides evenly.
    #[test]
    fun test_rounding_up_differs_by_one() {
        assert!(mul_div_rounding_up(MAX_U256, 1, 2) == mul_div(MAX_U256, 1, 2) + 1, 0);
        assert!(mul_div_rounding_up(7, 1, 3) == mul_div(7, 1, 3) + 1, 0);
        assert!(mul_div_rounding_up(1, 1, MAX_U256) == mul_div(1, 1, MAX_U256) + 1, 0);
        assert!(mul_div_rounding_up(6, 7, 2) == mul_div(6, 7, 2), 0);
        assert!(mul_div_rounding_up(MAX_U256, 1, MAX_U256) == mul_div(MAX_U256, 1, MAX_U256), 0);
    }

    #[test]
    fun test_div_rounding_up() {
        assert!(div_rounding_up(7, 3) == 3, 0);
        assert!(div_rounding_up(6, 3) == 2, 0);
        assert!(div_rounding_up(0, 3) == 0, 0);
        assert!(div_rounding_up(MAX_U256, 2) == (MAX_U256 - 1) / 2 + 1, 0);
    }

    #[test]
    #[expected_failure(abort_code = EDIVIDE_BY_ZERO, location = Self)]
    fun test_mul_div_by_zero() {
        mul_div(1, 1, 0);
    }

    #[test]
    #[expected_failure(abort_code = EDIVIDE_BY_ZERO, location = Self)]
    fun test_mul_div_rounding_up_by_zero() {
        mul_div_rounding_up(1, 1, 0);
    }

    #[test]
    fun test_most_significant_bit() {
        assert!(most_significant_bit(1) == 0, 0);
        assert!(most_significant_bit(2) == 1, 0);
        assert!(most_significant_bit(3) == 1, 0);
        assert!(most_significant_bit(255) == 7, 0);
        assert!(most_significant_bit(256) == 8, 0);
        assert!(most_significant_bit(1u256 << 96) == 96, 0);
        assert!(most_significant_bit(1u256 << 255) == 255, 0);
        assert!(most_significant_bit(MAX_U256) == 255, 0);
    }

    #[test]
    fun test_least_significant_bit() {
        assert!(least_significant_bit(1) == 0, 0);
        assert!(least_significant_bit(2) == 1, 0);
        assert!(least_significant_bit(3) == 0, 0);
        assert!(least_significant_bit(256) == 8, 0);
        assert!(least_significant_bit(1u256 << 96) == 96, 0);
        assert!(least_significant_bit(1u256 << 255) == 255, 0);
        assert!(least_significant_bit(MAX_U256) == 0, 0);
        assert!(least_significant_bit((1u256 << 200) + (1u256 << 37)) == 37, 0);
    }

    #[test]
    fun test_bit_scans_agree_on_single_bits() {
        let i = 0u8;
        while (i < 255) {
            let x = 1u256 << i;
            assert!(most_significant_bit(x) == i, 0);
            assert!(least_significant_bit(x) == i, 0);
            i = i + 1;
        };
    }

    #[test]
    #[expected_failure(abort_code = EZERO_HAS_NO_BIT, location = Self)]
    fun test_msb_of_zero() {
        most_significant_bit(0);
    }

    #[test]
    #[expected_failure(abort_code = EZERO_HAS_NO_BIT, location = Self)]
    fun test_lsb_of_zero() {
        least_significant_bit(0);
    }
}
