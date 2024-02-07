module aptos_std::ratio {

    use std::math64;
    use std::math128;

    /// Special cases defined as follows:
    /// - infinity := n:0 (n nonzero)
    /// - zero := 0:d (d nonzero)
    /// - NaN := 0:0
    struct Ratio has copy, drop, store {
        /// Numerator.
        n: u64,
        /// Denominator.
        d: u64
    }

    /// Input was not a number.
    const E_NAN: u64 = 0;
    /// Input on the left hand side of the comparator was not a number.
    const E_NAN_LHS: u64 = 1;
    /// Input on the right hand side of the comparator was not a number.
    const E_NAN_RHS: u64 = 2;
    /// Input was inifinity.
    const E_INFINITY: u64 = 3;
    /// Result overflows a u64.
    const E_OVERFLOW: u64 = 4;
    /// Zero times infinity is undefined.
    const E_ZERO_TIMES_INFINITY: u64 = 5;
    /// Result numerator overflows a u64.
    const E_OVERFLOW_NUMERATOR: u64 = 6;
    /// Result denominator overflows a u64.
    const E_OVERFLOW_DENOMINATOR: u64 = 7;
    /// Result underflows numerator.
    const E_UNDERFLOW_NUMERATOR: u64 = 8;
    /// Attempting to subtract infinity.
    const E_SUBTRACT_INFINITY: u64 = 9;
    /// Attempting to divide by zero.
    const E_DIVIDE_BY_ZERO: u64 = 10;
    /// May not divide infinity by infinity.
    const E_INFINITY_DIVIDED_BY_INFINITY: u64 = 11;

    const U64_MAX: u128 = 0xffffffffffffffff;

    public fun from_terms(x: u64, y: u64): Ratio { Ratio { n: x, d: y } }

    public fun inverse(r: Ratio): Ratio { Ratio { n: r.d, d: r.n } }

    public fun from_int(i: u64): Ratio { Ratio { n: i, d: 1 } }

    public fun is_zero(r: Ratio): bool { r.n == 0 && r.d != 0 }

    public fun is_unity(r: Ratio): bool { r.n != 0 && r.n == r.d }

    public fun is_infinity(r: Ratio): bool { r.n != 0 && r.d == 0 }

    public fun is_nan(r: Ratio): bool { r.n == 0 && r.d == 0 }

    public fun is_special(r: Ratio): bool { r.n == 0 || r.d == 0 }

    public fun to_terms(r: Ratio): (u64, u64) { (r.n, r.d) }

    public fun identical(a: Ratio, b: Ratio): bool { a == b }

    public fun to_quotient_and_remainder(r: Ratio): (u64, u64) {
        assert!(!is_nan(r), E_NAN);
        assert!(!is_infinity(r), E_INFINITY);
        to_quotient_and_remainder_unchecked(r)
    }

    public fun to_quotient_and_remainder_unchecked(r: Ratio): (u64, u64) {
        (r.n / r.d, r.n % r.d)
    }

    public fun reduce(r: Ratio): Ratio {
        assert!(!is_nan(r), E_NAN);
        reduce_unchecked(r)
    }

    public fun reduce_unchecked(r: Ratio): Ratio {
        let gcd = math64::gcd(r.n, r.d);
        Ratio { n: r.n / gcd, d: r.d / gcd }
    }

    public fun less_than(l: Ratio, r: Ratio): bool {
        assert!(!is_nan(l), E_NAN_LHS);
        assert!(!is_nan(r), E_NAN_RHS);
        less_than_unchecked(l, r)
    }

    public fun less_than_or_equal(l: Ratio, r: Ratio): bool {
        assert!(!is_nan(l), E_NAN_LHS);
        assert!(!is_nan(r), E_NAN_RHS);
        less_than_or_equal_unchecked(l, r)
    }

    public fun equal(l: Ratio, r: Ratio): bool {
        assert!(!is_nan(l), E_NAN_LHS);
        assert!(!is_nan(r), E_NAN_RHS);
        equal_unchecked(l, r)
    }

    public fun greater_than_or_equal(l: Ratio, r: Ratio): bool {
        assert!(!is_nan(l), E_NAN_LHS);
        assert!(!is_nan(r), E_NAN_RHS);
        greater_than_or_equal_unchecked(l, r)
    }

    public fun greater_than(l: Ratio, r: Ratio): bool {
        assert!(!is_nan(l), E_NAN_LHS);
        assert!(!is_nan(r), E_NAN_RHS);
        greater_than_unchecked(l, r)
    }

    public fun less_than_unchecked(l: Ratio, r: Ratio): bool {
       ((l.n as u128) * (r.d as u128)) < ((r.n as u128) * (l.d as u128))
    }

    public fun less_than_or_equal_unchecked(l: Ratio, r: Ratio): bool {
       ((l.n as u128) * (r.d as u128)) <= ((r.n as u128) * (l.d as u128))
    }

    public fun equal_unchecked(l: Ratio, r: Ratio): bool {
       ((l.n as u128) * (r.d as u128)) == ((r.n as u128) * (l.d as u128))
    }

    public fun greater_than_or_equal_unchecked(l: Ratio, r: Ratio): bool {
       ((l.n as u128) * (r.d as u128)) >= ((r.n as u128) * (l.d as u128))
    }

    public fun greater_than_unchecked(l: Ratio, r: Ratio): bool {
       ((l.n as u128) * (r.d as u128)) > ((r.n as u128) * (l.d as u128))
    }

    public inline fun sort(x: Ratio, y: Ratio): (Ratio, Ratio) {
        if (less_than(x, y)) (x, y) else (y, x)
    }

    public inline fun sort_unchecked(x: Ratio, y: Ratio): (Ratio, Ratio) {
        if (less_than_unchecked(x, y)) (x, y) else (y, x)
    }

    public fun times_int_as_int(r: Ratio, i: u64): u64 {
        assert!(!is_nan(r), E_NAN);
        assert!(!is_infinity(r), E_INFINITY);
        let result = ((r.n as u128) * (i as u128)) / (r.d as u128);
        assert!(result <= U64_MAX, E_OVERFLOW);
        (result as u64)
    }

    public fun times_int_as_int_unchecked(r: Ratio, i: u64): u64 {
        (((r.n as u128) * (i as u128)) / (r.d as u128) as u64)
    }

    /// inf * inf = inf
    /// inf * n = inf
    /// inf * 0 = error
    public fun multiply(x: Ratio, y: Ratio): Ratio {
        let (l, r) = sort(x, y);
        let zero_times_infinity = is_zero(l) && is_infinity(r);
        assert!(!zero_times_infinity, E_ZERO_TIMES_INFINITY);
        let n = (l.n as u128) * (r.n as u128);
        let d = (l.d as u128) * (r.d as u128);
        let gcd = math128::gcd(n, d);
        let n_reduced = n / gcd;
        assert!(n_reduced <= U64_MAX, E_OVERFLOW_NUMERATOR);
        let d_reduced = d / gcd;
        assert!(d_reduced <= U64_MAX, E_OVERFLOW_DENOMINATOR);
        Ratio { n: (n_reduced as u64), d: (d_reduced as u64) }
    }

    public fun multiply_unchecked(x: Ratio, y: Ratio): Ratio {
        let n = (x.n as u128) * (y.n as u128);
        let d = (x.d as u128) * (y.d as u128);
        let gcd = math128::gcd(n, d);
        Ratio { n: ((n / gcd) as u64), d: ((d / gcd) as u64) }
    }

    /// inf / inf = error
    /// inf / N = inf
    /// N / inf = 0
    /// 0 / N = 0
    /// 0 / inf = 0
    /// _ / 0 = error
    public fun divide(l: Ratio, r: Ratio): Ratio {
        assert!(!is_nan(l), E_NAN_LHS);
        assert!(!is_nan(r), E_NAN_RHS);
        assert!(!is_zero(r), E_DIVIDE_BY_ZERO);
        if (is_infinity(l)) {
            assert!(!is_infinity(r), E_INFINITY_DIVIDED_BY_INFINITY);
            return Ratio { n: 1, d: 0 }
        };
        let n = ((l.n as u128) * (r.d as u128));
        let d = ((r.n as u128) * (l.d as u128));
        let gcd = math128::gcd(n, d);
        let n_reduced = n / gcd;
        assert!(n_reduced <= U64_MAX, E_OVERFLOW_NUMERATOR);
        let d_reduced = d / gcd;
        assert!(d_reduced <= U64_MAX, E_OVERFLOW_DENOMINATOR);
        Ratio { n: (n_reduced as u64), d: (d_reduced as u64) }
    }

    public fun divide_unchecked(l: Ratio, r: Ratio): Ratio {
        let n = ((l.n as u128) * (r.d as u128));
        let d = ((r.n as u128) * (l.d as u128));
        let gcd = math128::gcd(n, d);
        Ratio { n: (n / gcd as u64), d: (d / gcd as u64) }
    }

    /// inf + inf = inf
    /// inf + n = inf
    /// inf + 0 = inf
    public fun add(l: Ratio, r: Ratio): Ratio {
        assert!(!is_nan(l), E_NAN_LHS);
        assert!(!is_nan(r), E_NAN_RHS);
        if (l.d == 0 || r.d == 0) return Ratio { n: 1, d: 0 };
        let l_d = (l.d as u128);
        let r_d = (r.d as u128);
        let n = ((l.n as u128) * r_d) + ((r.n as u128) * l_d);
        let d = (l_d * r_d);
        let gcd = math128::gcd(n, d);
        let n_reduced = n / gcd;
        assert!(n_reduced <= U64_MAX, E_OVERFLOW_NUMERATOR);
        let d_reduced = d / gcd;
        assert!(d_reduced <= U64_MAX, E_OVERFLOW_DENOMINATOR);
        Ratio { n: (n_reduced as u64), d: (d_reduced as u64) }
    }

    public fun add_unchecked(l: Ratio, r: Ratio): Ratio {
        let l_d = (l.d as u128);
        let r_d = (r.d as u128);
        let n = ((l.n as u128) * r_d) + ((r.n as u128) * l_d);
        let d = (l_d * r_d);
        let gcd = math128::gcd(n, d);
        Ratio { n: (n / gcd as u64), d: (d / gcd as u64) }
    }

    /// Infinity minus n = infinity
    /// Infinity minus 0 = infinity
    /// _ - inf = error
    public fun subtract(l: Ratio, r: Ratio): Ratio {
        assert!(!is_nan(l), E_NAN_LHS);
        assert!(!is_nan(r), E_NAN_RHS);
        assert!(!is_infinity(r), E_SUBTRACT_INFINITY);
        if (is_infinity(l)) return Ratio { n: 1, d: 0 };
        let l_d = (l.d as u128);
        let r_d = (r.d as u128);
        let a = ((l.n as u128) * r_d);
        let b = ((r.n as u128) * l_d);
        assert!(a >= b, E_UNDERFLOW_NUMERATOR);
        let n = a - b;
        let d = (l_d * r_d);
        let gcd = math128::gcd(n, d);
        let n_reduced = n / gcd;
        assert!(n_reduced <= U64_MAX, E_OVERFLOW_NUMERATOR);
        let d_reduced = d / gcd;
        assert!(d_reduced <= U64_MAX, E_OVERFLOW_DENOMINATOR);
        Ratio { n: (n_reduced as u64), d: (d_reduced as u64) }
    }

    public fun subtract_unchecked(l: Ratio, r: Ratio): Ratio {
        let l_d = (l.d as u128);
        let r_d = (r.d as u128);
        let n = ((l.n as u128) * r_d) - ((r.n as u128) * l_d);
        let d = (l_d * r_d);
        let gcd = math128::gcd(n, d);
        Ratio { n: (n / gcd as u64), d: (d / gcd as u64) }
    }

    #[test]
    fun test_assorted() {
        let zero = from_int(0);
        let unity = from_terms(5, 5);
        let infinity = from_terms(20, 0);
        let nan = from_terms(0, 0);
        let (n, d) = to_terms(zero);
        assert!(n == 0, 0);
        assert!(d == 1, 0);
        let (n, d) = to_terms(unity);
        assert!(n == 5, 0);
        assert!(d == 5, 0);
        assert!(inverse(from_terms(2, 3)) == from_terms(3, 2), 0);
        assert!(is_zero(zero), 0);
        assert!(!is_zero(unity), 0);
        assert!(!is_zero(infinity), 0);
        assert!(!is_zero(nan), 0);
        assert!(!is_unity(zero), 0);
        assert!(is_unity(unity), 0);
        assert!(!is_unity(infinity), 0);
        assert!(!is_unity(nan), 0);
        assert!(!is_infinity(zero), 0);
        assert!(!is_infinity(unity), 0);
        assert!(is_infinity(infinity), 0);
        assert!(!is_infinity(nan), 0);
        assert!(!is_nan(zero), 0);
        assert!(!is_nan(unity), 0);
        assert!(!is_nan(infinity), 0);
        assert!(is_nan(nan), 0);
        assert!(is_special(zero), 0);
        assert!(!is_special(unity), 0);
        assert!(is_special(infinity), 0);
        assert!(is_special(nan), 0);
        assert!(reduce(from_terms(9, 12)) == from_terms(3, 4), 0);
        let (quotient, remainder) = to_quotient_and_remainder(from_terms(12, 9));
        assert!(quotient == 1, 0);
        assert!(remainder == 3, 0);
        let (quotient, remainder) = to_quotient_and_remainder(from_terms(9, 12));
        assert!(quotient == 0, 0);
        assert!(remainder == 9, 0);
        assert!(identical(infinity, infinity), 0);
        assert!(!identical(infinity, zero), 0);
        assert!(!identical(from_terms(9, 12), from_terms(3, 4)), 0);
        assert!(!less_than(zero, zero), 0);
        assert!(less_than(zero, infinity), 0);
        assert!(less_than(zero, unity), 0);
        assert!(less_than_or_equal(zero, zero), 0);
        assert!(less_than_or_equal(zero, infinity), 0);
        assert!(less_than_or_equal(zero, unity), 0);
        assert!(less_than_or_equal(unity, unity), 0);
        assert!(less_than_or_equal(infinity, from_terms(25, 0)), 0);
        assert!(!less_than(unity, zero), 0);
        assert!(!less_than(unity, unity), 0);
        assert!(less_than(unity, infinity), 0);
        assert!(equal(infinity, from_terms(15, 0)), 0);
        assert!(equal(unity, from_terms(1, 1)), 0);
        assert!(equal(zero, from_terms(0, 1234)), 0);
        assert!(!greater_than(zero, zero), 0);
        assert!(!greater_than(zero, infinity), 0);
        assert!(!greater_than(zero, unity), 0);
        assert!(greater_than(infinity, zero), 0);
        assert!(greater_than(unity, zero), 0);
        assert!(greater_than_or_equal(zero, zero), 0);
        assert!(!greater_than_or_equal(zero, infinity), 0);
        assert!(greater_than_or_equal(unity, zero), 0);
        assert!(greater_than_or_equal(unity, unity), 0);
        assert!(greater_than_or_equal(infinity, from_terms(25, 0)), 0);
        let (small, large) = sort(zero, unity);
        assert!(small == zero, 0);
        assert!(large == unity, 0);
        let (small, large) = sort(unity, zero);
        assert!(small == zero, 0);
        assert!(large == unity, 0);
        let (small, large) = sort_unchecked(zero, unity);
        assert!(small == zero, 0);
        assert!(large == unity, 0);
        let (small, large) = sort_unchecked(unity, zero);
        assert!(small == zero, 0);
        assert!(large == unity, 0);
        assert!(times_int_as_int(from_terms(3, 4), 12) == 9, 0);
        assert!(times_int_as_int_unchecked(from_terms(10, 5), 4) == 8, 0);
        assert!(multiply(from_terms(3, 4), from_terms(1, 3)) == from_terms(1, 4), 0);
        assert!(reduce(multiply(infinity, infinity)) == from_terms(1, 0), 0);
        assert!(reduce(multiply(infinity, unity)) == from_terms(1, 0), 0);
        assert!(multiply_unchecked(from_terms(3, 4), from_terms(1, 3)) == from_terms(1, 4), 0);
        assert!(reduce(add(infinity, infinity)) == from_terms(1, 0), 0);
        assert!(reduce(add(zero, zero)) == from_terms(0, 1), 0);
        assert!(reduce(add(infinity, unity)) == from_terms(1, 0), 0);
        assert!(reduce(add(infinity, zero)) == from_terms(1, 0), 0);
        assert!(add(from_terms(2, 4), from_terms(3, 7)) == from_terms(13, 14), 0);
        assert!(add_unchecked(from_terms(2, 4), from_terms(3, 7)) == from_terms(13, 14), 0);
        assert!(subtract(from_terms(13, 14), from_terms(3, 7)) == from_terms(1, 2), 0);
        assert!(subtract_unchecked(from_terms(13, 14), from_terms(3, 7)) == from_terms(1, 2), 0);
        assert!(reduce(subtract(infinity, from_terms(3, 7))) == from_terms(1, 0), 0);
        assert!(reduce(subtract(infinity, zero)) == from_terms(1, 0), 0);
        assert!(divide(from_terms(1, 4), from_terms(1, 3)) == from_terms(3, 4), 0);
        assert!(divide(from_terms(1, 0), from_terms(1, 3)) == from_terms(1, 0), 0);
        assert!(divide_unchecked(from_terms(2, 8), from_terms(1, 3)) == from_terms(3, 4), 0);
    }

    #[test, expected_failure(abort_code=E_NAN, location = Self)]
    fun test_reduce_nan() {
        reduce(from_terms(0, 0));
    }

    #[test, expected_failure(abort_code=E_NAN_LHS, location = Self)]
    fun test_less_than_nan_lhs() {
        less_than(from_terms(0, 0), from_terms(1, 1));
    }

    #[test, expected_failure(abort_code=E_NAN_RHS, location = Self)]
    fun test_less_than_nan_rhs() {
        less_than(from_terms(1, 1), from_terms(0, 0));
    }

    #[test, expected_failure(abort_code=E_NAN_LHS, location = Self)]
    fun test_less_than_or_equal_nan_lhs() {
        less_than_or_equal(from_terms(0, 0), from_terms(1, 1));
    }

    #[test, expected_failure(abort_code=E_NAN_RHS, location = Self)]
    fun test_less_than_or_equal_nan_rhs() {
        less_than_or_equal(from_terms(1, 1), from_terms(0, 0));
    }

    #[test, expected_failure(abort_code=E_NAN_LHS, location = Self)]
    fun test_equal_nan_lhs() {
        equal(from_terms(0, 0), from_terms(1, 1));
    }

    #[test, expected_failure(abort_code=E_NAN_RHS, location = Self)]
    fun test_equal_nan_rhs() {
        equal(from_terms(1, 1), from_terms(0, 0));
    }

    #[test, expected_failure(abort_code=E_NAN_LHS, location = Self)]
    fun test_greater_than_nan_lhs() {
        greater_than(from_terms(0, 0), from_terms(1, 1));
    }

    #[test, expected_failure(abort_code=E_NAN_RHS, location = Self)]
    fun test_greater_than_nan_rhs() {
        greater_than(from_terms(1, 1), from_terms(0, 0));
    }

    #[test, expected_failure(abort_code=E_NAN_LHS, location = Self)]
    fun test_greater_than_or_equal_nan_lhs() {
        greater_than_or_equal(from_terms(0, 0), from_terms(1, 1));
    }

    #[test, expected_failure(abort_code=E_NAN_RHS, location = Self)]
    fun test_greater_than_or_equal_nan_rhs() {
        greater_than_or_equal(from_terms(1, 1), from_terms(0, 0));
    }

    #[test, expected_failure(abort_code=E_NAN, location = Self)]
    fun test_to_quotient_and_remainder_nan() {
        to_quotient_and_remainder(from_terms(0, 0));
    }

    #[test, expected_failure(abort_code=E_INFINITY, location = Self)]
    fun test_to_quotient_and_remainder_infinity() {
        to_quotient_and_remainder(from_terms(5, 0));
    }

    #[test, expected_failure(abort_code=E_NAN, location = Self)]
    fun test_times_int_as_int_nan() {
        times_int_as_int(from_terms(0, 0), 1);
    }

    #[test, expected_failure(abort_code=E_INFINITY, location = Self)]
    fun test_times_int_as_int_infinity() {
        times_int_as_int(from_terms(1, 0), 1);
    }

    #[test, expected_failure(abort_code=E_OVERFLOW, location = Self)]
    fun test_times_int_as_int_overflow() {
        times_int_as_int(from_terms(2, 1), (U64_MAX as u64) / 2 + 1);
    }

    #[test, expected_failure(abort_code=E_ZERO_TIMES_INFINITY, location = Self)]
    fun test_multiply_zero_times_infinity() {
        multiply(from_terms(0, 1), from_terms(1, 0));
    }

    #[test, expected_failure(abort_code=E_OVERFLOW_NUMERATOR, location = Self)]
    fun test_multiply_overflow_numerator() {
        multiply(from_terms(3, 1), from_terms((U64_MAX as u64) / 2, 1));
    }

    #[test, expected_failure(abort_code=E_OVERFLOW_DENOMINATOR, location = Self)]
    fun test_multiply_overflow_denominator() {
        multiply(from_terms(1, 3), from_terms(1, (U64_MAX as u64) / 2));
    }

    #[test, expected_failure(abort_code=E_NAN_LHS, location = Self)]
    fun test_add_nan_lhs() {
        add(from_terms(0, 0), from_terms(1, (U64_MAX as u64) / 2));
    }

    #[test, expected_failure(abort_code=E_NAN_RHS, location = Self)]
    fun test_add_nan_rhs() {
        add(from_terms(1, (U64_MAX as u64) / 2), from_terms(0, 0));
    }

    #[test, expected_failure(abort_code=E_OVERFLOW_NUMERATOR, location = Self)]
    fun test_add_overflow_numerator() {
        add(from_terms((U64_MAX as u64), 1), from_terms(1, 1));
    }

    #[test, expected_failure(abort_code=E_OVERFLOW_DENOMINATOR, location = Self)]
    fun test_add_overflow_denominator() {
        add(from_terms(1, (U64_MAX / 2 as u64)), from_terms(1, 3));
    }

    #[test, expected_failure(abort_code=E_UNDERFLOW_NUMERATOR, location = Self)]
    fun test_subtract_underflow_numerator() {
        subtract(from_terms(1, 3), from_terms(1, 2));
    }

    #[test, expected_failure(abort_code=E_SUBTRACT_INFINITY, location = Self)]
    fun test_subtract_infinity() {
        subtract(from_terms(1, 3), from_terms(1, 0));
    }

    #[test, expected_failure(abort_code=E_NAN_LHS, location = Self)]
    fun test_subtract_nan_lhs() {
        subtract(from_terms(0, 0), from_terms(1, 1));
    }

    #[test, expected_failure(abort_code=E_NAN_RHS, location = Self)]
    fun test_subtract_nan_rhs() {
        subtract(from_terms(1, 1), from_terms(0, 0));
    }

    #[test, expected_failure(abort_code=E_OVERFLOW_NUMERATOR, location = Self)]
    fun test_subtract_overflow_numeraor() {
        subtract(from_terms((U64_MAX as u64), 1), from_terms(1, 3));
    }

    #[test, expected_failure(abort_code=E_OVERFLOW_DENOMINATOR, location = Self)]
    fun test_subtract_overflow_denominator() {
        subtract(from_terms(1, 3), from_terms(1, (U64_MAX / 2 as u64)));
    }

    #[test, expected_failure(abort_code=E_NAN_LHS, location = Self)]
    fun test_divide_nan_lhs() {
        divide(from_terms(0, 0), from_terms(1, 1));
    }

    #[test, expected_failure(abort_code=E_NAN_RHS, location = Self)]
    fun test_divide_nan_rhs() {
        divide(from_terms(1, 1), from_terms(0, 0));
    }

    #[test, expected_failure(abort_code=E_DIVIDE_BY_ZERO, location = Self)]
    fun test_divide_by_zero() {
        divide(from_terms(1, 1), from_terms(0, 1));
    }

    #[test, expected_failure(abort_code=E_INFINITY_DIVIDED_BY_INFINITY, location = Self)]
    fun test_divide_infinity_by_infinity() {
        divide(from_terms(1, 0), from_terms(1, 0));
    }

    #[test, expected_failure(abort_code=E_OVERFLOW_NUMERATOR, location = Self)]
    fun test_divide_overflow_numerator() {
        divide(from_terms(3, 1), from_terms(1, (U64_MAX as u64) / 2));
    }

    #[test, expected_failure(abort_code=E_OVERFLOW_DENOMINATOR, location = Self)]
    fun test_divide_overflow_denominator() {
        divide(from_terms(1, 3), from_terms((U64_MAX as u64) / 2, 1));
    }

}