/// Fixed-point decimal implementation, useful for financial applications where, for example, prices
/// need to be tracked between assets with disparate market values or decimal amounts.
///
/// Fixed-point decimal value are represented as a simple `u128` without a type wrapper, to optimize
/// performance. This enables, for example, prices to be arranged in total order within a sorted
/// collection, using simple arithmetic comparators for m-ary search tree traversal or similar.
///
/// This implementation provides enough precision such that an integer value of 1 multiplied by
/// the largest possible fixed-point decimal value (`MAX_DECIMAL_FIXED_u128`) will result in the
/// largest possible power of 10 that can fit in a `u64` (`MAX_U64_DECIMAL_u64`). Conversely,
/// `MAX_U64_DECIMAL_u64` multiplied by the smallest possible fixed-point decimal value (1 encoded
/// as a `u128`) will have a result of 1. For more, see `scale_int()` and assocated tests.
module aptos_std::fixed_decimal {

    /// Largest power of 10 that can fit in a `u64`. In Python:
    ///
    /// ```python
    /// import math
    /// print(f"{10 ** (int(math.log10(int('1' * 64, 2)))):_}")
    /// ```
    const MAX_U64_DECIMAL_u64: u64   = 10_000_000_000_000_000_000;
    const MAX_U64_DECIMAL_u128: u128 = 10_000_000_000_000_000_000;
    const MAX_U64_DECIMAL_u256: u256 = 10_000_000_000_000_000_000;
    const UNITY_u128: u128           = 10_000_000_000_000_000_000;
    const UNITY_u256: u256           = 10_000_000_000_000_000_000;
    const SCALE_FACTOR_u128: u128    = 10_000_000_000_000_000_000;
    const SCALE_FACTOR_u256: u256    = 10_000_000_000_000_000_000;

    /// Largest power of 10 that can fit in a `u64`, squared. In Python:
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

    #[view]
    public fun get_MAX_U64_DECIMAL(): u64 { MAX_U64_DECIMAL_u64 }

    public inline fun get_MAX_U64_DECIMAL_inline(): u64 { 10_000_000_000_000_000_000 }

    #[view]
    public fun get_MAX_DECIMAL_FIXED(): u128 { MAX_DECIMAL_FIXED_u128 }

    public inline fun get_MAX_DECIMAL_FIXED_inline(): u128 {
        100_000_000_000_000_000_000_000_000_000_000_000_000
    }

    #[view]
    public fun from_int(int: u64): u128 {
        assert!(int <= MAX_U64_DECIMAL_u64, E_INT_TOO_LARGE);
        (int as u128) * (SCALE_FACTOR_u128)
    }

    #[view]
    /// Inputs do not necessarily need to be within max representable `u64` value bounds. See tests.
    public fun from_ratio(numerator: u64, denominator: u64): u128 {
        assert!(denominator != 0, E_DIVIDE_BY_ZERO);
        let result = (numerator as u256) * (SCALE_FACTOR_u256) / (denominator as u256);
        assert!(result <= MAX_DECIMAL_FIXED_u256, E_OVERFLOW);
        (result as u128)
    }

    #[view]
    public fun add(fixed_l: u128, fixed_r: u128): u128 {
        assert!(fixed_l <= MAX_DECIMAL_FIXED_u128, E_FIXED_TOO_LARGE_LHS);
        assert!(fixed_r <= MAX_DECIMAL_FIXED_u128, E_FIXED_TOO_LARGE_RHS);
        let result = fixed_l + fixed_r;
        assert!(result <= MAX_DECIMAL_FIXED_u128, E_OVERFLOW);
        result
    }

    #[view]
    public fun subtract(fixed_l: u128, fixed_r: u128): u128 {
        assert!(fixed_l <= MAX_DECIMAL_FIXED_u128, E_FIXED_TOO_LARGE_LHS);
        assert!(fixed_r <= MAX_DECIMAL_FIXED_u128, E_FIXED_TOO_LARGE_RHS);
        assert!(fixed_l >= fixed_r, E_UNDERFLOW);
        fixed_l - fixed_r
    }

    #[view]
    public fun multiply(fixed_l: u128, fixed_r: u128): u128 {
        assert!(fixed_l <= MAX_DECIMAL_FIXED_u128, E_FIXED_TOO_LARGE_LHS);
        assert!(fixed_r <= MAX_DECIMAL_FIXED_u128, E_FIXED_TOO_LARGE_RHS);
        let result = (fixed_l as u256) * (fixed_r as u256) / (SCALE_FACTOR_u256);
        assert!(result <= MAX_DECIMAL_FIXED_u256, E_OVERFLOW);
        (result as u128)
    }

    #[view]
    public fun divide(fixed_l: u128, fixed_r: u128): u128 {
        assert!(fixed_l <= MAX_DECIMAL_FIXED_u128, E_FIXED_TOO_LARGE_LHS);
        assert!(fixed_r <= MAX_DECIMAL_FIXED_u128, E_FIXED_TOO_LARGE_RHS);
        assert!(fixed_r != 0, E_DIVIDE_BY_ZERO);
        let result = (fixed_l as u256) * SCALE_FACTOR_u256 / (fixed_r as u256);
        assert!(result <= MAX_DECIMAL_FIXED_u256, E_OVERFLOW);
        (result as u128)
    }

    #[view]
    public fun scale_int(int: u64, fixed: u128): u64 {
        assert!(int <= MAX_U64_DECIMAL_u64, E_INT_TOO_LARGE);
        assert!(fixed <= MAX_DECIMAL_FIXED_u128, E_FIXED_TOO_LARGE);
        let result = ((int as u256) * (fixed as u256)) / SCALE_FACTOR_u256;
        assert!(result <= MAX_U64_DECIMAL_u256, E_OVERFLOW);
        (result as u64)
    }

    #[view]
    public fun divide_int(int: u64, fixed: u128): u64 {
        assert!(int <= MAX_U64_DECIMAL_u64, E_INT_TOO_LARGE);
        assert!(fixed <= MAX_DECIMAL_FIXED_u128, E_FIXED_TOO_LARGE);
        assert!(fixed != 0, E_DIVIDE_BY_ZERO);
        let result = (int as u256) * SCALE_FACTOR_u256 / (fixed as u256);
        assert!(result <= MAX_U64_DECIMAL_u256, E_OVERFLOW);
        (result as u64)
    }

    /// For when division by zero will not happen, but overflow might. A performance optimization
    /// that enables low-cost checks from calling functions.
    public inline fun from_ratio_optimistic(numerator: u64, denominator: u64): (u256, bool) {
        let result = (numerator as u256) * (SCALE_FACTOR_u256) / (denominator as u256);
        (
            result, // Value before casting back to `u128`.
            result > MAX_DECIMAL_FIXED_u256, // True if result overflows a fixed decimal.
        )
    }

    /// For when integer and fixed decimal inputs are valid, but the result might overflow. A
    /// performance optimization that enables low-cost checks from calling functions.
    public inline fun scale_int_optimistic(int: u64, fixed: u128): (u256, bool) {
        let result = ((int as u256) * (fixed as u256)) / SCALE_FACTOR_u256;
        (
            result, // Value before casting back to `u64`.
            result > MAX_U64_DECIMAL_u256, // True if result overflows max power of ten in `u64`.
        )
    }

    /// For when integer and fixed decimal inputs are valid, and the result is known to not
    /// overflow. A performance optimization that enables low-cost checks from calling functions.
    public inline fun divide_int_optimistic(int: u64, fixed: u128): u64 {
        ((int as u256) * SCALE_FACTOR_u256 / (fixed as u256) as u64)
    }

    /// For fixed decimal inputs that are known to be valid, but the result might overflow. A
    /// performance optimization that enables low-cost checks from calling functions.
    public inline fun multiply_optimistic(fixed_l: u128, fixed_r: u128): (u256, bool) {
        let result = (fixed_l as u256) * (fixed_r as u256) / (SCALE_FACTOR_u256);
        (
            result, // Value before casting back to `u128`.
            result > MAX_DECIMAL_FIXED_u256 // True if result overflows a fixed decimal.
        )
    }

    /// For fixed decimal inputs that are known to be valid, where `fixed_r` >= `UNITY_u128`. A
    /// performance optimization that enables low-cost invocation from calling functions.
    public inline fun divide_optimistic(fixed_l: u128, fixed_r: u128): u128 {
        ((fixed_l as u256) * SCALE_FACTOR_u256 / (fixed_r as u256) as u128)
    }

    #[test_only]
    use std::vector;

    #[test_only]
    /// As a fixed-pont decimal.
    const RELATIVE_ERROR_TOLERANCE: u128 = 1;

    #[test_only]
    fun assert_relative_error(expected: u128, actual: u128) {
        let (error) = if (expected > actual) (expected - actual) else (actual - expected);
        let relative_error = divide(error, expected);
        assert!(relative_error <= RELATIVE_ERROR_TOLERANCE, 0);
    }

    #[test_only]
    fun assert_bilateral_conversion_precision(a: u64, b: u64) {
        assert_unilateral_conversion_precision(a, b);
        assert_unilateral_conversion_precision(b, a);
    }

    #[test_only]
    fun assert_unilateral_conversion_precision(a: u64, b: u64) {
        let c = from_ratio(a, b);
        assert_relative_error(from_int(a), multiply(from_int(b), c));
        assert_relative_error(from_int(b), divide(from_int(a), c));
    }

    #[test]
    fun test_conversion_precision() {
        let bigs = vector[
            MAX_U64_DECIMAL_u64,
            MAX_U64_DECIMAL_u64 - 1,
            9_876_543_210_987_654_321, // Causes failure.
            1_234_567_890_123_456_789, // Causes failure.
        ];
        let smalls = vector[
            1,
            2,
            3,
            12,
            123,
            1_234,
            12_345,
            123_456
        ];
        vector::for_each_ref(&bigs, |i_ref| {
            vector::for_each_ref(&smalls, |j_ref| {
                assert_bilateral_conversion_precision(*i_ref, *j_ref);
            })
        });
    }

    #[test]
    fun test_constant_getters() {
        assert!(get_MAX_U64_DECIMAL() == MAX_U64_DECIMAL_u64, 0);
        assert!(get_MAX_U64_DECIMAL_inline() == MAX_U64_DECIMAL_u64, 0);
        assert!(get_MAX_DECIMAL_FIXED() == MAX_DECIMAL_FIXED_u128, 0);
        assert!(get_MAX_DECIMAL_FIXED_inline() == MAX_DECIMAL_FIXED_u128, 0);
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
    fun test_from_ratio_optimistic() {
        let (result, overflows) = from_ratio_optimistic(5, 2);
        assert!(result == 5 * UNITY_u256 / 2, 0);
        assert!(!overflows, 0);
        let (result, overflows) = from_ratio_optimistic(MAX_U64_DECIMAL_u64 + 1, 1);
        assert!(result == (MAX_U64_DECIMAL_u256 + 1) * SCALE_FACTOR_u256, 0);
        assert!(overflows, 0);
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

    #[test]
    fun test_scale_int_optmistic() {
        let (result, overflows) = scale_int_optimistic(1, UNITY_u128);
        assert!(result == 1, 0);
        assert!(!overflows, 0);
        let (result, overflows) = scale_int_optimistic(MAX_U64_DECIMAL_u64, UNITY_u128 * 2);
        assert!(result == 2 * MAX_U64_DECIMAL_u256, 0);
        assert!(overflows, 0);
    }

    #[test]
    fun test_divide_int() {
        assert!(divide_int(1, UNITY_u128) == 1, 0);
        assert!(divide_int(1, 1) == MAX_U64_DECIMAL_u64, 0);
        assert!(divide_int(MAX_U64_DECIMAL_u64, MAX_DECIMAL_FIXED_u128) == 1, 0);
    }

    #[test, expected_failure(abort_code = E_INT_TOO_LARGE, location = Self)]
    fun test_divide_int_int_too_large() {
        divide_int(MAX_U64_DECIMAL_u64 + 1, UNITY_u128);
    }

    #[test, expected_failure(abort_code = E_FIXED_TOO_LARGE, location = Self)]
    fun test_divide_int_fixed_too_large() {
        divide_int(1, MAX_DECIMAL_FIXED_u128 + 1);
    }

    #[test, expected_failure(abort_code = E_DIVIDE_BY_ZERO, location = Self)]
    fun test_divide_int_divide_by_zero() {
        divide_int(1, 0);
    }

    #[test, expected_failure(abort_code = E_OVERFLOW, location = Self)]
    fun test_divide_int_overflow() {
        divide_int(MAX_U64_DECIMAL_u64 / 2 + 1, UNITY_u128 / 2);
    }

    #[test]
    fun test_divide_int_optimistic() {
        assert!(divide_int_optimistic(1, UNITY_u128) == 1, 0);
        assert!(divide_int_optimistic(1, 1) == MAX_U64_DECIMAL_u64, 0);
        assert!(divide_int_optimistic(MAX_U64_DECIMAL_u64, MAX_DECIMAL_FIXED_u128) == 1, 0);
    }


    #[test]
    fun test_multiply_optimistic() {
        let (result, overflows) = multiply_optimistic(UNITY_u128, UNITY_u128);
        assert!(result == (UNITY_u128 as u256), 0);
        assert!(!overflows, 0);
        let (result, overflows) = multiply_optimistic(MAX_DECIMAL_FIXED_u128, 1);
        assert!(result == (UNITY_u128 as u256), 0);
        assert!(!overflows, 0);
        let (result, overflows) = multiply_optimistic(1, MAX_DECIMAL_FIXED_u128);
        assert!(result == (UNITY_u128 as u256), 0);
        assert!(!overflows, 0);
        let (result, overflows) = multiply_optimistic(MAX_DECIMAL_FIXED_u128, 2 * UNITY_u128);
        assert!(result == (2 * MAX_DECIMAL_FIXED_u128 as u256), 0);
        assert!(overflows, 0);
    }

    #[test]
    fun test_divide_optimistic() {
        assert!(divide_optimistic(UNITY_u128, 1) == MAX_DECIMAL_FIXED_u128, 0);
        assert!(divide_optimistic(UNITY_u128, UNITY_u128) == UNITY_u128, 0);
        assert!(divide_optimistic(0, UNITY_u128) == 0, 0);
        assert!(divide_optimistic(1, UNITY_u128) == 1, 0);
        assert!(divide_optimistic(MAX_DECIMAL_FIXED_u128, MAX_DECIMAL_FIXED_u128) == UNITY_u128, 0);
        assert!(divide_optimistic(MAX_DECIMAL_FIXED_u128, UNITY_u128) == MAX_DECIMAL_FIXED_u128, 0);
    }

}