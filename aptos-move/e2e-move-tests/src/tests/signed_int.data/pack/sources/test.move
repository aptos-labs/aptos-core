 /// Module for testing signed integers
 module 0x99::signed_int {
    use std::i64;
    use std::i128;

    // test `from`
    // create an `i64/i128` from a positive `u64/u128`, and then check if the result equals to `i64/i128` created from literals
    fun test_from() {
        assert!(i64::from(0) == 0i64, 0);
        assert!(i64::from(1) == 1, 0);
        assert!(i64::from(0x7fffffffffffffff) == 0x7fffffffffffffff, 0);
        assert!(i128::from(0) == 0, 0);
        assert!(i128::from(1) == 1, 0);
        assert!(i128::from(0x7fffffffffffffffffffffffffffffff) == 0x7fffffffffffffffffffffffffffffff, 0);
    }

    // test `from`:
    // create an `i64/i128` from a "negative" `u64/u128`, and then check if the result equals to `i64/i128` created from literals
    fun test_neg_from() {
        assert!(i64::neg_from(0) == 0, 0);
        assert!(i64::neg_from(1) == -1, 0);
        assert!(i64::neg_from(0x8000000000000000) == -0x8000000000000000, 0);
    }

    // test `neg` with built-in `-` operator
    fun test_neg(){
        assert!(i64::from(0).neg() == 0, 0);
        assert!(i64::from(3).neg() == -3, 0);
        assert!(i64::neg_from(3).neg() == 3, 0);

        assert!(i128::from(0).neg() == 0, 0);
        assert!(i128::from(3).neg() == -3, 0);
        assert!(i128::neg_from(3).neg() == 3, 0);

        let a = -3i64;
        assert!(-a == 3, 0);
        assert!(--a == a, 0);

        let a = -3i128;
        assert!(-a == 3, 0);
        assert!(--a == a, 0);

        let a = 3i64;
        assert!(-a == -3, 0);
        assert!(--a == a, 0);

        let a = 3i128;
        assert!(-a == -3, 0);
        assert!(--a == a, 0);
    }

    // test `add` with built-in `-` operator
    fun test_add() {
        assert!(i64::from(5).add(i64::neg_from(3)) == 2, 0);
        assert!(i128::from(5).add(i128::neg_from(3)) == 2, 0);
        assert!(i64::from(3).add(i64::neg_from(5)) == -2, 0);
        assert!(i128::from(3).add(i128::neg_from(5)) == -2, 0);
        assert!(i64::from(1).add(i64::neg_from(1)) == 0, 0);
        assert!(i128::from(1).add(i128::neg_from(1)) == 0, 0);

        assert!(5 + (-3) == i64::from(2), 0);
        assert!(5 + (-3) == i128::from(2), 0);

        assert!(-5 + 3 == i64::neg_from(2), 0);
        assert!(-5 + 3 == i128::neg_from(2), 0);

        assert!(-1 + 1 == i64::from(0), 0);
        assert!(-1 + 1 == i128::from(0), 0);

        assert!(5 + (-3) == 2i64, 0);
        assert!(5 + (-3) == 2i128, 0);

        assert!(-5 + 3 == -2i64, 0);
        assert!(-5 + 3 == -2i128, 0);

        assert!(-1 + 1 == 0i64, 0);
        assert!(-1 + 1 == 0i128, 0);

        let a = 1i64;
        let b = 2i64;
        b += a;
        assert!(a + b == 4, 0);

        let a = -1i64;
        let b = -2i64;
        b += a;
        assert!(a + b == -4, 0);

        let a = 1i128;
        let b = 2i128;
        b += a;
        assert!(a + b == 4, 0);

        let a = -1i128;
        let b = -2i128;
        b += a;
        assert!(a + b == -4, 0);
    }

    // test `sub` with built-in `-` operator
    fun test_sub() {
        assert!(i64::from(5).sub(i64::neg_from(3)) == 8, 0);
        assert!(i128::from(5).sub(i128::neg_from(3)) == 8, 0);
        assert!(i64::from(3).sub(i64::neg_from(5)) == 8, 0);
        assert!(i128::from(3).sub(i128::neg_from(5)) == 8, 0);
        assert!(i64::neg_from(1).sub(i64::from(1)) == -2, 0);
        assert!(i128::neg_from(1).sub(i128::from(1)) == -2, 0);

        assert!(5 - (-3) == i64::from(8), 0);
        assert!(5 - (-3) == i128::from(8), 0);

        assert!(3 - (-5) == i64::from(8), 0);
        assert!(3 - (-5) == i128::from(8), 0);

        assert!(-1 - 1 == i64::neg_from(2), 0);
        assert!(-1 - 1 == i128::neg_from(2), 0);

        assert!(5 - (-3) == 8i64, 0);
        assert!(5 - (-3) == 8i128, 0);

        assert!(3 - (-5) == 8i64, 0);
        assert!(3 - (-5) == 8i128, 0);

        assert!(-1 - 1 == -2i64, 0);
        assert!(-1 - 1 == -2i128, 0);

        let a = 1i64;
        let b = 2i64;
        assert!(a - b == -1, 0);

        let a = -1i64;
        let b = -2i64;
        assert!(a - b == 1, 0);

        let a = 1i128;
        let b = 2i128;
        assert!(a - b == -1, 0);

        let a = -1i128;
        let b = -2i128;
        assert!(a - b == 1, 0);
    }

    // test `mul` with built-in `*` operator
    fun test_mul() {
        assert!(i64::from(5).mul(i64::neg_from(3)) == -15i64, 0);
        assert!(i128::from(5).mul(i128::neg_from(3)) == -15i128, 0);

        assert!(i64::neg_from(5).mul(i64::neg_from(3)) == 15i64, 0);
        assert!(i128::neg_from(5).mul(i128::neg_from(3)) == 15i128, 0);

        assert!(i64::from(5).mul(i64::from(3)) == 15i64, 0);
        assert!(i128::from(5).mul(i128::from(3)) == 15i128, 0);

        assert!(i64::from(0).mul(i64::from(3)) == 0, 0);
        assert!(i128::from(0).mul(i128::from(3)) == 0, 0);


        assert!(5 * (-3) == i64::neg_from(15), 0);
        assert!(5 * (-3) == i128::neg_from(15), 0);

        assert!((-5) * (-3) == i64::from(15), 0);
        assert!((-5) * (-3) == i128::from(15), 0);

        assert!(5 * 3 == i64::from(15), 0);
        assert!(5 * 3 == i128::from(15), 0);

        assert!(0 * 3 == i64::from(0), 0);
        assert!(0 * 3 == i128::from(0), 0);

        assert!(5 * (-3) == -15i64, 0);
        assert!(5 * (-3) == -15i128, 0);

        assert!((-5) * (-3) == 15i64, 0);
        assert!((-5) * (-3) == 15i128, 0);

        assert!(5 * 3 == 15i64, 0);
        assert!(5 * 3 == 15i128, 0);

        assert!(0 * 3 == 0i64, 0);
        assert!(0 * 3 == 0i128, 0);
    }

    // test `div` with built-in `/` operator
    fun test_div() {
        assert!(i64::from(3).div(i64::from(3)) == 1, 0);
        assert!(i64::from(4).div(i64::from(3)) == 1, 0);
        assert!(i64::from(5).div(i64::from(3)) == 1, 0);
        assert!(i64::neg_from(3).div(i64::from(3)) == -1, 0);
        assert!(i64::neg_from(4).div(i64::from(3)) == -1, 0);
        assert!(i64::neg_from(5).div(i64::from(3)) == -1, 0);

        assert!(i128::from(3).div(i128::from(3)) == 1, 0);
        assert!(i128::from(4).div(i128::from(3)) == 1, 0);
        assert!(i128::from(5).div(i128::from(3)) == 1, 0);
        assert!(i128::neg_from(3).div(i128::from(3)) == -1, 0);
        assert!(i128::neg_from(4).div(i128::from(3)) == -1, 0);
        assert!(i128::neg_from(5).div(i128::from(3)) == -1, 0);

        assert!(3 / 3 == 1i64, 0);
        assert!(4 / 3 == 1i64, 0);
        assert!(5 / 3 == 1i64, 0);
        assert!((-3) / 3 == -1i64, 0);
        assert!((-4) / 3 == -1i64, 0);
        assert!((-5) / 3 == -1i64, 0);

        assert!(3 / 3 == 1i128, 0);
        assert!(4 / 3 == 1i128, 0);
        assert!(5 / 3 == 1i128, 0);
        assert!((-3) / 3 == -1i128, 0);
        assert!((-4) / 3 == -1i128, 0);
        assert!((-5) / 3 == -1i128, 0);

    }

    // test `mod` with built-in `%` operator
    fun test_mod() {
        assert!(i64::from(10).mod(i64::from(3)) == 1, 0);
        assert!(i64::neg_from(10).mod(i64::from(3)) == -1, 0);
        assert!(i64::from(10).mod(i64::neg_from(3)) == 1, 0);
        assert!(i64::neg_from(10).mod(i64::neg_from(3)) == -1, 0);

        assert!(i128::from(10).mod(i128::from(3)) == 1, 0);
        assert!(i128::neg_from(10).mod(i128::from(3)) == -1, 0);
        assert!(i128::from(10).mod(i128::neg_from(3)) == 1, 0);
        assert!(i128::neg_from(10).mod(i128::neg_from(3)) == -1, 0);

        assert!(10 % 3 == 1i64, 0);
        assert!((-10) % 3 == -1i64, 0);
        assert!(10 % (-3) == 1i64, 0);
        assert!((-10) % (-3) == -1i64, 0);

        assert!(10 % 3 == 1i128, 0);
        assert!((-10) % 3 == -1i128, 0);
        assert!(10 % (-3) == 1i128, 0);
        assert!((-10) % (-3) == -1i128, 0);
    }

    // test `gt/gte` with built-in `> >=` operator
    fun test_gt_gte() {
        assert!(i64::from(5) > (i64::from(3)), 0);
        assert!(!(i64::from(3) > i64::from(5)), 0);
        assert!(!(i64::from(5) > i64::from(5)), 0);
        assert!(i64::from(5) > (i64::neg_from(5)), 0);
        assert!(i64::neg_from(2) > (i64::neg_from(3)), 0);

        assert!(i64::from(5) >= (i64::from(3)), 0);
        assert!(!(i64::from(3) >= i64::from(5)), 0);
        assert!(i64::from(5) >= (i64::from(5)), 0);
        assert!(i64::from(5) >= (i64::neg_from(5)), 0);
        assert!(i64::neg_from(2) >= (i64::neg_from(3)), 0);

        assert!(i128::from(5) > (i128::from(3)), 0);
        assert!(!(i128::from(3) > i128::from(5)), 0);
        assert!(!(i128::from(5) > i128::from(5)), 0);
        assert!(i128::from(5) > (i128::neg_from(5)), 0);
        assert!(i128::neg_from(2) > (i128::neg_from(3)), 0);

        assert!(i128::from(5) >= (i128::from(3)), 0);
        assert!(!(i128::from(3) >= i128::from(5)), 0);
        assert!(i128::from(5) >= (i128::from(5)), 0);
        assert!(i128::from(5) >= (i128::neg_from(5)), 0);
        assert!(i128::neg_from(2) >= (i128::neg_from(3)), 0);


        assert!(5 > 3i64, 0);
        assert!(!(3 > 5i64), 0);
        assert!(!(5 > 5i64), 0);
        assert!(5 > -5i64, 0);
        assert!(-2 > -3i64, 0);

        assert!(5 >= 3i64, 0);
        assert!(!(3 >= 5i64), 0);
        assert!(5 >= 5i64, 0);
        assert!(5 >= -5i64, 0);
        assert!(-2 >= -3i64, 0);

        assert!(5 > 3i128, 0);
        assert!(!(3 > 5i128), 0);
        assert!(!(5 > 5i128), 0);
        assert!(5 > -5i128, 0);
        assert!(-2 > -3i128, 0);

        assert!(5 >= 3i128, 0);
        assert!(!(3 >= 5i128), 0);
        assert!(5 >= 5i128, 0);
        assert!(5 >= -5i128, 0);
        assert!(-2 >= -3i128, 0);
    }

    // test `lt/lte` with built-in `< <=` operator
    fun test_lt_lte() {
        assert!(!(i64::from(5) < i64::from(3)), 0);
        assert!(i64::from(3) < i64::from(5), 0);
        assert!(!(i64::from(5) < i64::from(5)), 0);
        assert!(i64::from(5) > (i64::neg_from(5)), 0);
        assert!(!(i64::neg_from(2) < i64::neg_from(3)), 0);

        assert!(!(i64::from(5) <= i64::from(3)), 0);
        assert!(i64::from(3) <= i64::from(5), 0);
        assert!(i64::from(5) <= (i64::from(5)), 0);
        assert!(!(i64::from(5) <= i64::neg_from(5)), 0);
        assert!(!(i64::neg_from(2) <= i64::neg_from(3)), 0);

        assert!(!(i128::from(5) < i128::from(3)), 0);
        assert!(i128::from(3) < i128::from(5), 0);
        assert!(!(i128::from(5) < i128::from(5)), 0);
        assert!(i128::from(5) > (i128::neg_from(5)), 0);
        assert!(!(i128::neg_from(2) < i128::neg_from(3)), 0);

        assert!(!(i128::from(5) <= i128::from(3)), 0);
        assert!(i128::from(3) <= i128::from(5), 0);
        assert!(i128::from(5) <= (i128::from(5)), 0);
        assert!(!(i128::from(5) <= i128::neg_from(5)), 0);
        assert!(!(i128::neg_from(2) <= i128::neg_from(3)), 0);


        assert!(!(5 < 3i64), 0);
        assert!(3 < 5i64, 0);
        assert!(!(5 < 5i64), 0);
        assert!(!(5 < -5i64), 0);
        assert!(!(-2 < -3i64), 0);

        assert!(!(5 <= 3i64), 0);
        assert!(3 <= 5i64, 0);
        assert!(5 <= 5i64, 0);
        assert!(!(5 <= -5i64), 0);
        assert!(!(-2 <= -3i64), 0);

        assert!(!(5 < 3i128), 0);
        assert!(3 < 5i128, 0);
        assert!(!(5 < 5i128), 0);
        assert!(!(5 < -5i128), 0);
        assert!(!(-2 < -3i128), 0);

        assert!(!(5 <= 3i128), 0);
        assert!(3 <= 5i128, 0);
        assert!(5 <= 5i128, 0);
        assert!(!(5 <= -5i128), 0);
        assert!(!(-2 <= -3i128), 0);
    }

    // test `eq` with built-in `==` operator
    fun test_eq() {
        assert!(i64::from(5) == 5, 0);
        assert!(-5 == i64::neg_from(5), 0);
        assert!(5i64 == 5, 0);
        assert!(-5i64 == -5, 0);

        assert!(i128::from(5) == 5, 0);
        assert!(-5 == i128::neg_from(5), 0);
        assert!(5i128 == 5, 0);
        assert!(-5i128 == -5, 0);
    }

    // test `neq` with built-in `!=` operator
    fun test_neq() {
        assert!(!(i64::from(5) != 5), 0);
        assert!(5 != i64::neg_from(5), 0);
        assert!(!(5i64 != 5), 0);
        assert!(!(-5i64 != -5), 0);

        assert!(!(i128::from(5) != 5), 0);
        assert!(5 != i128::neg_from(5), 0);
        assert!(!(5i128 != 5), 0);
        assert!(!(-5i128 != -5), 0);
    }

    // test `cmp` method from `std::i64/i128` together with `i64/i128` literals
    fun test_cmp() {
        assert!(i64::cmp(5, 3) == 2, 0);
        assert!(i64::cmp(3, 5) == 0, 0);
        assert!(i64::cmp(5, 5) == 1, 0);
        assert!(i64::cmp(-5, 5) == 0, 0);
        assert!(i64::cmp(5, -5) == 2, 0);
        assert!(i64::cmp(-1, -2) == 2, 0);

        assert!(i128::cmp(5, 3) == 2, 0);
        assert!(i128::cmp(3, 5) == 0, 0);
        assert!(i128::cmp(5, 5) == 1, 0);
        assert!(i128::cmp(-5, 5) == 0, 0);
        assert!(i128::cmp(5, -5) == 2, 0);
        assert!(i128::cmp(-1, -2) == 2, 0);
    }

    // test `wrapping_add` method from `std::i64/i128` together with `i64/i128` literals
    fun test_wrapping_add() {
        assert!(i64::wrapping_add(6, 3) == 9, 0);
        assert!(i64::wrapping_add(0x7fffffffffffffff, 1) == -0x8000000000000000, 0);
        assert!(i64::wrapping_add(-1, 1) == 0, 0);

        assert!(i128::wrapping_add(6, 3) == 9, 0);
        assert!(i128::wrapping_add(0x7fffffffffffffffffffffffffffffff, 1) == -0x80000000000000000000000000000000, 0);
        assert!(i128::wrapping_add(-1, 1) == 0, 0);
    }

    // test `wrapping_sub` method from `std::i64/i128` together with `i64/i128` literals
    fun test_wrapping_sub() {
        assert!(i64::wrapping_sub(6, 3) == 3, 0);
        assert!(i64::wrapping_sub(-0x8000000000000000, 1) == 0x7fffffffffffffff, 0);
        assert!(i64::wrapping_sub(1, 1) == 0, 0);

        assert!(i128::wrapping_sub(6, 3) == 3, 0);
        assert!(i128::wrapping_sub(-0x80000000000000000000000000000000, 1) == 0x7fffffffffffffffffffffffffffffff, 0);
        assert!(i128::wrapping_sub(1, 1) == 0, 0);
    }

    // test `abs` method from `std::i64/i128` together with `i64/i128` literals
    fun test_abs() {
        assert!(i64::abs(5) == 5, 0);
        assert!(i64::abs(-5) == 5, 0);
        assert!(i64::abs(-0) == 0, 0);

        assert!(i128::abs(5) == 5, 0);
        assert!(i128::abs(-5) == 5, 0);
        assert!(i128::abs(-0) == 0, 0);
    }

     // test `abs_u64/abs_u128` method from `std::i64/i128` together with `i64/i128` literals
    fun test_abs_u64() {
        assert!(i64::abs_u64(5) == 5, 0);
        assert!(i64::abs_u64(-5) == 5, 0);
        assert!(i64::abs_u64(-0) == 0, 0);

        assert!(i128::abs_u128(5) == 5, 0);
        assert!(i128::abs_u128(-5) == 5, 0);
        assert!(i128::abs_u128(-0) == 0, 0);
    }

    // test `min/max` method from `std::i64/i128` together with `i64/i128` literals
    fun test_min_max() {
        assert!(i64::min(3, 3) == 3, 0);
        assert!(i64::min(3, 5) == 3, 0);
        assert!(i64::min(3, -5) == -5, 0);

        assert!(i64::max(3, 3) == 3, 0);
        assert!(i64::max(3, 5) == 5, 0);
        assert!(i64::max(3, -5) == 3, 0);

        assert!(i128::min(3, 3) == 3, 0);
        assert!(i128::min(3, 5) == 3, 0);
        assert!(i128::min(3, -5) == -5, 0);

        assert!(i128::max(3, 3) == 3, 0);
        assert!(i128::max(3, 5) == 5, 0);
        assert!(i128::max(3, -5) == 3, 0);
    }

    // test `pow` method from `std::i64/i128` together with `i64/i128` literals
    fun test_pow() {
        assert!(i64::pow(2, 0) == 1, 0);
        assert!(i64::pow(2, 3) == 8, 0);
        assert!(i64::pow(-2, 3) == -8, 0);
        assert!(i64::pow(-2, 4) == 16, 0);

        assert!(i128::pow(2, 0) == 1, 0);
        assert!(i128::pow(2, 3) == 8, 0);
        assert!(i128::pow(-2, 3) == -8, 0);
        assert!(i128::pow(-2, 4) == 16, 0);
    }

    // test `zero/is_zero` method from `std::i64/i128` together with `i64/i128` literals
    fun test_zero() {
        assert!(i64::is_zero(0), 0);
        assert!(i64::is_zero(i64::zero()), 0);
        assert!(!i64::is_zero(1), 0);
        assert!(!i64::is_zero(-1), 0);

        assert!(i128::is_zero(0), 0);
        assert!(i128::is_zero(i128::zero()), 0);
        assert!(!i128::is_zero(1), 0);
        assert!(!i128::is_zero(-1), 0);
    }

    // test `sign/is_neg` method from `std::i64/i128` together with `i64/i128` literals
    fun test_sign() {
        assert!(i64::sign(5) == 0, 0);
        assert!(i64::sign(-5) == 1, 0);
        assert!(i64::sign(0) == 0, 0);

        assert!(!i64::is_neg(5), 0);
        assert!(i64::is_neg(-5), 0);
        assert!(!i64::is_neg(0), 0);

        assert!(i128::sign(5) == 0, 0);
        assert!(i128::sign(-5) == 1, 0);
        assert!(i128::sign(0) == 0, 0);

        assert!(!i128::is_neg(5), 0);
        assert!(i128::is_neg(-5), 0);
        assert!(!i128::is_neg(0), 0);

    }

    // test `bits/pack/unpack` method from `std::i64/i128` together with `i64/i128` literals
    fun test_bits_pack_unpack() {
        assert!(i64::bits(&5) == 5, 0);
        assert!(i64::bits(&(-5)) == 0xfffffffffffffffb, 0);

        assert!(i64::pack(0) == 0, 0);
        assert!(i64::pack(5) == 5, 0);
        assert!(i64::pack(0xfffffffffffffffb) == -5, 0);

        assert!(i64::unpack(0) == 0, 0);
        assert!(i64::unpack(5) == 5, 0);
        assert!(i64::unpack(-5) == 0xfffffffffffffffb, 0);

        assert!(i128::bits(&5) == 5, 0);
        assert!(i128::bits(&(-5)) == 0xfffffffffffffffffffffffffffffffb, 0);

        assert!(i128::pack(0) == 0, 0);
        assert!(i128::pack(5) == 5, 0);
        assert!(i128::pack(0xfffffffffffffffffffffffffffffffb) == -5, 0);

        assert!(i128::unpack(0) == 0, 0);
        assert!(i128::unpack(5) == 5, 0);
        assert!(i128::unpack(-5) == 0xfffffffffffffffffffffffffffffffb, 0);
    }

    entry fun test_entry() {
        test_from();
        test_neg_from();
        test_neg();
        test_add();
        test_sub();
        test_mul();
        test_div();
        test_mod();
        test_gt_gte();
        test_lt_lte();
        test_eq();
        test_neq();

        test_cmp();
        test_wrapping_add();
        test_wrapping_sub();
        test_abs();
        test_min_max();
        test_pow();
        test_zero();
        test_sign();
        test_bits_pack_unpack();
    }
}
