module aptos_std::fixed_decimal {

    /// Largest power of 10 that can fit in a `u64`.
    ///
    /// ```python
    /// import math
    /// print(f"{10 ** (int(math.log10(int('1' * 64, 2)))):_}")
    /// ```
    const MAX_U64_DECIMAL_u64: u64   = 10_000_000_000_000_000_000;
    const MAX_U64_DECIMAL_u128: u128 = 10_000_000_000_000_000_000;
    const MAX_U64_DECIMAL_u256: u256 = 10_000_000_000_000_000_000;
    const UNITY_u128: u128           = 10_000_000_000_000_000_000;
    const SCALE_FACTOR_u128: u128    = 10_000_000_000_000_000_000;
    const SCALE_FACTOR_u256: u256    = 10_000_000_000_000_000_000;

    /// Largest power of 10 that can fit in a `u64`, squared.
    ///
    /// ```python
    /// import math
    /// print(f"{(10 ** (int(math.log10(int('1' * 64, 2))))) ** 2:_}")
    /// ```
    const MAX_DECIMAL_FIXED_u128: u128 = 100_000_000_000_000_000_000_000_000_000_000_000_000;
    const MAX_DECIMAL_FIXED_u256: u256 = 100_000_000_000_000_000_000_000_000_000_000_000_000;

    /// Integer input exceeded the largest power of 10 that can fit in a `u64`.
    const E_INT_TOO_LARGE: u64 = 0;
    /// Decimal fixed point input exceeded the maximum allowed value.
    const E_FIXED_TOO_LARGE: u64 = 1;
    /// The operation overflowed.
    const E_OVERFLOW: u64 = 2;
    /// Decimal fixed point input on left hand side exceeded the maximum allowed value.
    const E_FIXED_TOO_LARGE_LHS: u64 = 3;
    /// Decimal fixed point input on right hand side exceeded the maximum allowed value.
    const E_FIXED_TOO_LARGE_RHS: u64 = 4;
    /// The operation underflowed.
    const E_UNDERFLOW: u64 = 5;
    /// Dividing by zero is not permitted.
    const E_DIVIDE_BY_ZERO: u64 = 6;

    public fun from_int(int: u64): u128 {
        assert!(int <= MAX_U64_DECIMAL_u64, E_INT_TOO_LARGE);
        (int as u128) * (SCALE_FACTOR_u128)
    }

    /// Inputs do not necessarily need to be within max representable `u64` value bounds. See tests.
    public fun from_ratio(numerator: u64, denominator: u64): u128 {
        assert!(denominator != 0, E_DIVIDE_BY_ZERO);
        let result = (numerator as u256) * (SCALE_FACTOR_u256) / (denominator as u256);
        assert!(result <= MAX_DECIMAL_FIXED_u256, E_OVERFLOW);
        (result as u128)
    }

    public fun add(fixed_l: u128, fixed_r: u128): u128 {
        assert!(fixed_l <= MAX_DECIMAL_FIXED_u128, E_FIXED_TOO_LARGE_LHS);
        assert!(fixed_r <= MAX_DECIMAL_FIXED_u128, E_FIXED_TOO_LARGE_RHS);
        let result = fixed_l + fixed_r;
        assert!(result <= MAX_DECIMAL_FIXED_u128, E_OVERFLOW);
        result
    }

    public fun subtract(fixed_l: u128, fixed_r: u128): u128 {
        assert!(fixed_l <= MAX_DECIMAL_FIXED_u128, E_FIXED_TOO_LARGE_LHS);
        assert!(fixed_r <= MAX_DECIMAL_FIXED_u128, E_FIXED_TOO_LARGE_RHS);
        assert!(fixed_l >= fixed_r, E_UNDERFLOW);
        fixed_l - fixed_r
    }

    public fun multiply(fixed_l: u128, fixed_r: u128): u128 {
        assert!(fixed_l <= MAX_DECIMAL_FIXED_u128, E_FIXED_TOO_LARGE_LHS);
        assert!(fixed_r <= MAX_DECIMAL_FIXED_u128, E_FIXED_TOO_LARGE_RHS);
        let result = (fixed_l as u256) * (fixed_r as u256) / (SCALE_FACTOR_u256);
        assert!(result <= MAX_DECIMAL_FIXED_u256, E_OVERFLOW);
        (result as u128)
    }

    public fun divide(fixed_l: u128, fixed_r: u128): u128 {
        assert!(fixed_l <= MAX_DECIMAL_FIXED_u128, E_FIXED_TOO_LARGE_LHS);
        assert!(fixed_r <= MAX_DECIMAL_FIXED_u128, E_FIXED_TOO_LARGE_RHS);
        assert!(fixed_r != 0, E_DIVIDE_BY_ZERO);
        let result = (fixed_l as u256) * SCALE_FACTOR_u256 / (fixed_r as u256);
        assert!(result <= MAX_DECIMAL_FIXED_u256, E_OVERFLOW);
        (result as u128)
    }

    public fun scale_int(int: u64, fixed: u128): u64 {
        assert!(int <= MAX_U64_DECIMAL_u64, E_INT_TOO_LARGE);
        assert!(fixed <= MAX_DECIMAL_FIXED_u128, E_FIXED_TOO_LARGE);
        let result = ((int as u256) * (fixed as u256)) / SCALE_FACTOR_u256;
        assert!(result <= MAX_U64_DECIMAL_u256, E_OVERFLOW);
        (result as u64)
    }

    #[test]
    fun test_from_int() {
        assert!(from_int(1) == UNITY_u128, 0);
        assert!(from_int(MAX_U64_DECIMAL_u64) == MAX_DECIMAL_FIXED_u128, 0);
    }

    #[test, expected_failure(abort_code = E_INT_TOO_LARGE, location = Self)]
    fun test_from_int_too_large() {
        from_int(MAX_U64_DECIMAL_u64 + 1);
    }

    #[test]
    fun test_from_ratio() {
        assert!(
            from_ratio(
                ((3 * MAX_U64_DECIMAL_u128 / 2) as u64),
                MAX_U64_DECIMAL_u64
            ) == 3 * UNITY_u128 / 2,
            0
        );
        assert!(from_ratio(5, 2) == 5 * UNITY_u128 / 2, 0);
    }

    #[test, expected_failure(abort_code = E_DIVIDE_BY_ZERO, location = Self)]
    fun test_from_ratio_divide_by_zero() {
        from_ratio(1, 0);
    }

    #[test, expected_failure(abort_code = E_OVERFLOW, location = Self)]
    fun test_from_ratio_overflow() {
        from_ratio(MAX_U64_DECIMAL_u64 + 1, 1);
    }

    #[test]
    fun test_add() {
        assert!(add(1, 1) == 2, 0);
    }

    #[test, expected_failure(abort_code = E_FIXED_TOO_LARGE_LHS, location = Self)]
    fun test_add_too_large_lhs() {
        add(MAX_DECIMAL_FIXED_u128 + 1, 1);
    }

    #[test, expected_failure(abort_code = E_FIXED_TOO_LARGE_RHS, location = Self)]
    fun test_add_too_large_rhs() {
        add(1, MAX_DECIMAL_FIXED_u128 + 1);
    }

    #[test, expected_failure(abort_code = E_OVERFLOW, location = Self)]
    fun test_add_overflow() {
        add(1, MAX_DECIMAL_FIXED_u128);
    }

    #[test]
    fun test_subtract() {
        assert!(subtract(1, 1) == 0, 0);
    }

    #[test, expected_failure(abort_code = E_FIXED_TOO_LARGE_LHS, location = Self)]
    fun test_subtract_too_large_lhs() {
        subtract(MAX_DECIMAL_FIXED_u128 + 1, 1);
    }

    #[test, expected_failure(abort_code = E_FIXED_TOO_LARGE_RHS, location = Self)]
    fun test_subtract_too_large_rhs() {
        subtract(1, MAX_DECIMAL_FIXED_u128 + 1);
    }

    #[test, expected_failure(abort_code = E_UNDERFLOW, location = Self)]
    fun test_subtract_underflow() {
        subtract(1, MAX_DECIMAL_FIXED_u128);
    }

    #[test]
    fun test_multiply() {
        assert!(multiply(UNITY_u128, UNITY_u128) == UNITY_u128, 0);
        assert!(multiply(MAX_DECIMAL_FIXED_u128, 1) == UNITY_u128, 0);
        assert!(multiply(1, MAX_DECIMAL_FIXED_u128) == UNITY_u128, 0);
    }

    #[test, expected_failure(abort_code = E_FIXED_TOO_LARGE_LHS, location = Self)]
    fun test_multiply_too_large_lhs() {
        multiply(MAX_DECIMAL_FIXED_u128 + 1, 1);
    }

    #[test, expected_failure(abort_code = E_FIXED_TOO_LARGE_RHS, location = Self)]
    fun test_multiply_too_large_rhs() {
        multiply(1, MAX_DECIMAL_FIXED_u128 + 1);
    }

    #[test, expected_failure(abort_code = E_OVERFLOW, location = Self)]
    fun test_multiply_overflow() {
        multiply(MAX_DECIMAL_FIXED_u128, UNITY_u128 + 1);
    }

    #[test]
    fun test_divide() {
        assert!(divide(UNITY_u128, 1) == MAX_DECIMAL_FIXED_u128, 0);
        assert!(divide(UNITY_u128, UNITY_u128) == UNITY_u128, 0);
        assert!(divide(0, UNITY_u128) == 0, 0);
        assert!(divide(1, UNITY_u128) == 1, 0);
        assert!(divide(MAX_DECIMAL_FIXED_u128, MAX_DECIMAL_FIXED_u128) == UNITY_u128, 0);
        assert!(divide(MAX_DECIMAL_FIXED_u128, UNITY_u128) == MAX_DECIMAL_FIXED_u128, 0);
    }

    #[test, expected_failure(abort_code = E_FIXED_TOO_LARGE_LHS, location = Self)]
    fun test_divide_too_large_lhs() {
        divide(MAX_DECIMAL_FIXED_u128 + 1, 1);
    }

    #[test, expected_failure(abort_code = E_FIXED_TOO_LARGE_RHS, location = Self)]
    fun test_divide_too_large_rhs() {
        divide(1, MAX_DECIMAL_FIXED_u128 + 1);
    }

    #[test, expected_failure(abort_code = E_DIVIDE_BY_ZERO, location = Self)]
    fun test_divide_divide_by_zero() {
        divide(1, 0);
    }

    #[test, expected_failure(abort_code = E_OVERFLOW, location = Self)]
    fun test_divide_overflow() {
        divide(MAX_DECIMAL_FIXED_u128, UNITY_u128 - 1);
    }

    #[test]
    fun test_scale_int() {
        assert!(scale_int(1, UNITY_u128) == 1, 0);
        assert!(scale_int(MAX_U64_DECIMAL_u64, 1) == 1, 0);
        assert!(scale_int(1, MAX_DECIMAL_FIXED_u128) == MAX_U64_DECIMAL_u64, 0);
        assert!(scale_int(1, 0) == 0, 0);
    }

    #[test, expected_failure(abort_code = E_INT_TOO_LARGE, location = Self)]
    fun test_scale_int_int_too_large() {
        scale_int(MAX_U64_DECIMAL_u64 + 1, 1);
    }

    #[test, expected_failure(abort_code = E_FIXED_TOO_LARGE, location = Self)]
    fun test_scale_int_fixed_too_large() {
        scale_int(1, MAX_DECIMAL_FIXED_u128 + 1);
    }

    #[test, expected_failure(abort_code = E_OVERFLOW, location = Self)]
    fun test_scale_int_overflow() {
        scale_int(MAX_U64_DECIMAL_u64, MAX_DECIMAL_FIXED_u128);
    }

}