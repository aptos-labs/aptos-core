#[test_only]
module aptos_std::i128_tests {
    use aptos_std::i128;

    #[test]
    fun test_from() {
        assert!(i128::from(0).bits() == 0, 0);
        assert!(i128::from(1).bits() == 1, 0);
        assert!(i128::from(0x7fffffffffffffffffffffffffffffff).bits() == 0x7fffffffffffffffffffffffffffffff, 0);
    }

    #[test]
    #[expected_failure(abort_code = i128::EOVERFLOW)]
    fun test_from_overflow() {
        i128::from(0x80000000000000000000000000000000);
    }

    #[test]
    fun test_neg_from() {
        assert!(i128::neg_from(0).bits() == 0, 0);
        assert!(i128::neg_from(1).bits() == 0xffffffffffffffffffffffffffffffff, 0);
        assert!(i128::neg_from(0x80000000000000000000000000000000).bits() == 0x80000000000000000000000000000000, 0);
    }

    #[test]
    #[expected_failure(abort_code = i128::EOVERFLOW)]
    fun test_neg_from_overflow() {
        i128::neg_from(0x80000000000000000000000000000001);
    }

    #[test]
    fun test_neg() {
        assert!(i128::from(3).neg() == i128::neg_from(3), 0);
    }

    #[test]
    fun test_add() {
        assert!(i128::from(5).add(i128::neg_from(3)) == i128::from(2), 0);
    }

    #[test]
    fun test_sub() {
        assert!(i128::from(5).sub(i128::from(3)) == i128::from(2), 0);
    }

    #[test]
    fun test_mul() {
        assert!(i128::from(5).mul(i128::neg_from(3)) == i128::neg_from(15), 0);
    }

    #[test]
    fun test_div() {
        assert!(i128::from(3).div(i128::from(3)) == i128::from(1), 0);
        assert!(i128::from(4).div(i128::from(3)) == i128::from(1), 0);
        assert!(i128::from(5).div(i128::from(3)) == i128::from(1), 0);
        assert!(i128::neg_from(3).div(i128::from(3)) == i128::neg_from(1), 0);
        assert!(i128::neg_from(4).div(i128::from(3)) == i128::neg_from(1), 0);
        assert!(i128::neg_from(5).div(i128::from(3)) == i128::neg_from(1), 0);
    }

    #[test]
    fun test_wrapping_add() {
        assert!(i128::from(5).wrapping_add(i128::from(3)) == i128::from(8), 0);
        assert!(i128::from(0x7fffffffffffffffffffffffffffffff).wrapping_add(i128::from(1)).bits() == 0x80000000000000000000000000000000, 0);
        assert!(i128::neg_from(1).wrapping_add(i128::from(1)) == i128::zero(), 0);
    }

    #[test]
    fun test_wrapping_sub() {
        assert!(i128::from(5).wrapping_sub(i128::from(3)) == i128::from(2), 0);
        assert!(i128::neg_from(0x80000000000000000000000000000000).wrapping_sub(i128::from(1)).bits() == 0x7fffffffffffffffffffffffffffffff, 0);
        assert!(i128::from(0).wrapping_sub(i128::from(1)) == i128::neg_from(1), 0);
    }

    #[test]
    fun test_mod() {
        assert!(i128::from(10).mod(i128::from(3)) == i128::from(1), 0);
        assert!(i128::neg_from(10).mod(i128::from(3)) == i128::neg_from(1), 0);
        assert!(i128::from(10).mod(i128::neg_from(3)) == i128::from(1), 0);
        assert!(i128::neg_from(10).mod(i128::neg_from(3)) == i128::neg_from(1), 0);
    }

    #[test]
    fun test_abs() {
        assert!(i128::from(5).abs() == i128::from(5), 0);
        assert!(i128::neg_from(5).abs() == i128::from(5), 0);
        assert!(i128::zero().abs() == i128::zero(), 0);
    }

    #[test]
    fun test_abs_u128() {
        assert!(i128::from(5).abs_u128() == 5, 0);
        assert!(i128::neg_from(5).abs_u128() == 5, 0);
        assert!(i128::zero().abs_u128() == 0, 0);
    }

    #[test]
    fun test_min_max() {
        assert!(i128::from(5).min(i128::from(3)) == i128::from(3), 0);
        assert!(i128::from(3).min(i128::from(5)) == i128::from(3), 0);
        assert!(i128::neg_from(5).min(i128::from(3)) == i128::neg_from(5), 0);
        
        assert!(i128::from(5).max(i128::from(3)) == i128::from(5), 0);
        assert!(i128::from(3).max(i128::from(5)) == i128::from(5), 0);
        assert!(i128::neg_from(5).max(i128::from(3)) == i128::from(3), 0);
    }

    #[test]
    fun test_pow() {
        assert!(i128::from(2).pow(0) == i128::from(1), 0);
        assert!(i128::from(2).pow(3) == i128::from(8), 0);
        assert!(i128::neg_from(2).pow(3) == i128::neg_from(8), 0);
        assert!(i128::neg_from(2).pow(4) == i128::from(16), 0);
    }

    #[test]
    fun test_pack_unpack() {
        assert!(i128::pack(0).unpack() == 0, 0);
        assert!(i128::pack(0xffffffffffffffffffffffffffffffff).unpack() == 0xffffffffffffffffffffffffffffffff, 0);
        assert!(i128::pack(0x80000000000000000000000000000000).unpack() == 0x80000000000000000000000000000000, 0);
    }

    #[test]
    fun test_bits() {
        assert!(i128::from(5).bits() == 5, 0);
        assert!(i128::neg_from(5).bits() == 0xfffffffffffffffffffffffffffffffb, 0);
    }

    #[test]
    fun test_sign() {
        assert!(i128::from(5).sign() == 0, 0);
        assert!(i128::neg_from(5).sign() == 1, 0);
        assert!(i128::zero().sign() == 0, 0);
    }

    #[test]
    fun test_zero_is_zero() {
        assert!(i128::zero().is_zero(), 0);
        assert!(!i128::from(1).is_zero(), 0);
        assert!(!i128::neg_from(1).is_zero(), 0);
    }

    #[test]
    fun test_is_neg() {
        assert!(!i128::from(5).is_neg(), 0);
        assert!(i128::neg_from(5).is_neg(), 0);
        assert!(!i128::zero().is_neg(), 0);
    }

    #[test]
    fun test_cmp() {
        assert!(i128::from(5).cmp(i128::from(3)) == 2, 0);
        assert!(i128::from(3).cmp(i128::from(5)) == 0, 0);
        assert!(i128::from(5).cmp(i128::from(5)) == 1, 0);
        assert!(i128::neg_from(5).cmp(i128::from(5)) == 0, 0);
        assert!(i128::from(5).cmp(i128::neg_from(5)) == 2, 0);
    }

    #[test]
    fun test_eq() {
        assert!(i128::from(5).eq(i128::from(5)), 0);
        assert!(!i128::from(5).eq(i128::from(3)), 0);
        assert!(!i128::from(5).eq(i128::neg_from(5)), 0);
    }

    #[test]
    fun test_gt_gte() {
        assert!(i128::from(5).gt(i128::from(3)), 0);
        assert!(!i128::from(3).gt(i128::from(5)), 0);
        assert!(!i128::from(5).gt(i128::from(5)), 0);
        assert!(i128::from(5).gt(i128::neg_from(5)), 0);

        assert!(i128::from(5).gte(i128::from(3)), 0);
        assert!(!i128::from(3).gte(i128::from(5)), 0);
        assert!(i128::from(5).gte(i128::from(5)), 0);
        assert!(i128::from(5).gte(i128::neg_from(5)), 0);
    }

    #[test]
    fun test_lt_lte() {
        assert!(!i128::from(5).lt(i128::from(3)), 0);
        assert!(i128::from(3).lt(i128::from(5)), 0);
        assert!(!i128::from(5).lt(i128::from(5)), 0);
        assert!(i128::neg_from(5).lt(i128::from(5)), 0);

        assert!(!i128::from(5).lte(i128::from(3)), 0);
        assert!(i128::from(3).lte(i128::from(5)), 0);
        assert!(i128::from(5).lte(i128::from(5)), 0);
        assert!(i128::neg_from(5).lte(i128::from(5)), 0);
    }
}