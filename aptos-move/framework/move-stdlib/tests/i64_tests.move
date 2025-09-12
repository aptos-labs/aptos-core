#[test_only]
module std::i64_tests {
    use std::i64;

    #[test]
    fun test_from() {
        assert!(i64::from(0).bits() == 0, 0);
        assert!(i64::from(1).bits() == 1, 0);
        assert!(i64::from(0x7fffffffffffffff).bits() == 0x7fffffffffffffff, 0);
    }

    #[test]
    #[expected_failure(abort_code = i64::EOVERFLOW)]
    fun test_from_overflow() {
        i64::from(0x8000000000000000);
    }

    #[test]
    fun test_neg_from() {
        assert!(i64::neg_from(0).bits() == 0, 0);
        assert!(i64::neg_from(1).bits() == 0xffffffffffffffff, 0);
        assert!(i64::neg_from(0x8000000000000000).bits() == 0x8000000000000000, 0);
    }

    #[test]
    #[expected_failure(abort_code = i64::EOVERFLOW)]
    fun test_neg_from_overflow() {
        i64::neg_from(0x8000000000000001);
    }

    #[test]
    fun test_neg() {
        assert!(i64::from(3).neg() == i64::neg_from(3), 0);
    }

    #[test]
    fun test_add() {
        assert!(i64::from(5).add(i64::neg_from(3)) == i64::from(2), 0);
    }

    #[test]
    fun test_sub() {
        assert!(i64::from(5).sub(i64::from(3)) == i64::from(2), 0);
    }

    #[test]
    fun test_mul() {
        assert!(i64::from(5).mul(i64::neg_from(3)) == i64::neg_from(15), 0);
    }

    #[test]
    fun test_div() {
        assert!(i64::from(3).div(i64::from(3)) == i64::from(1), 0);
        assert!(i64::from(4).div(i64::from(3)) == i64::from(1), 0);
        assert!(i64::from(5).div(i64::from(3)) == i64::from(1), 0);
        assert!(i64::neg_from(3).div(i64::from(3)) == i64::neg_from(1), 0);
        assert!(i64::neg_from(4).div(i64::from(3)) == i64::neg_from(1), 0);
        assert!(i64::neg_from(5).div(i64::from(3)) == i64::neg_from(1), 0);
    }

    #[test]
    fun test_wrapping_add() {
        assert!(i64::from(5).wrapping_add(i64::from(3)) == i64::from(8), 0);
        assert!(i64::from(0x7fffffffffffffff).wrapping_add(i64::from(1)).bits() == 0x8000000000000000, 0);
        assert!(i64::neg_from(1).wrapping_add(i64::from(1)) == i64::zero(), 0);
    }

    #[test]
    fun test_wrapping_sub() {
        assert!(i64::from(5).wrapping_sub(i64::from(3)) == i64::from(2), 0);
        assert!(i64::neg_from(0x8000000000000000).wrapping_sub(i64::from(1)).bits() == 0x7fffffffffffffff, 0);
        assert!(i64::from(0).wrapping_sub(i64::from(1)) == i64::neg_from(1), 0);
    }

    #[test]
    fun test_mod() {
        assert!(i64::from(10).mod(i64::from(3)) == i64::from(1), 0);
        assert!(i64::neg_from(10).mod(i64::from(3)) == i64::neg_from(1), 0);
        assert!(i64::from(10).mod(i64::neg_from(3)) == i64::from(1), 0);
        assert!(i64::neg_from(10).mod(i64::neg_from(3)) == i64::neg_from(1), 0);
    }

    #[test]
    fun test_abs() {
        assert!(i64::from(5).abs() == i64::from(5), 0);
        assert!(i64::neg_from(5).abs() == i64::from(5), 0);
        assert!(i64::zero().abs() == i64::zero(), 0);
    }

    #[test]
    fun test_abs_u64() {
        assert!(i64::from(5).abs_u64() == 5, 0);
        assert!(i64::neg_from(5).abs_u64() == 5, 0);
        assert!(i64::zero().abs_u64() == 0, 0);
    }

    #[test]
    fun test_min_max() {
        assert!(i64::from(5).min(i64::from(3)) == i64::from(3), 0);
        assert!(i64::from(3).min(i64::from(5)) == i64::from(3), 0);
        assert!(i64::neg_from(5).min(i64::from(3)) == i64::neg_from(5), 0);

        assert!(i64::from(5).max(i64::from(3)) == i64::from(5), 0);
        assert!(i64::from(3).max(i64::from(5)) == i64::from(5), 0);
        assert!(i64::neg_from(5).max(i64::from(3)) == i64::from(3), 0);
    }

    #[test]
    fun test_pow() {
        assert!(i64::from(2).pow(0) == i64::from(1), 0);
        assert!(i64::from(2).pow(3) == i64::from(8), 0);
        assert!(i64::neg_from(2).pow(3) == i64::neg_from(8), 0);
        assert!(i64::neg_from(2).pow(4) == i64::from(16), 0);
    }

    #[test]
    fun test_pack_unpack() {
        assert!(i64::pack(0).unpack() == 0, 0);
        assert!(i64::pack(0xffffffffffffffff).unpack() == 0xffffffffffffffff, 0);
        assert!(i64::pack(0x8000000000000000).unpack() == 0x8000000000000000, 0);
    }

    #[test]
    fun test_bits() {
        assert!(i64::from(5).bits() == 5, 0);
        assert!(i64::neg_from(5).bits() == 0xfffffffffffffffb, 0);
    }

    #[test]
    fun test_sign() {
        assert!(i64::from(5).sign() == 0, 0);
        assert!(i64::neg_from(5).sign() == 1, 0);
        assert!(i64::zero().sign() == 0, 0);
    }

    #[test]
    fun test_zero_is_zero() {
        assert!(i64::zero().is_zero(), 0);
        assert!(!i64::from(1).is_zero(), 0);
        assert!(!i64::neg_from(1).is_zero(), 0);
    }

    #[test]
    fun test_is_neg() {
        assert!(!i64::from(5).is_neg(), 0);
        assert!(i64::neg_from(5).is_neg(), 0);
        assert!(!i64::zero().is_neg(), 0);
    }

    #[test]
    fun test_cmp() {
        assert!(i64::from(5).cmp(i64::from(3)) == 2, 0);
        assert!(i64::from(3).cmp(i64::from(5)) == 0, 0);
        assert!(i64::from(5).cmp(i64::from(5)) == 1, 0);
        assert!(i64::neg_from(5).cmp(i64::from(5)) == 0, 0);
        assert!(i64::from(5).cmp(i64::neg_from(5)) == 2, 0);
        assert!(i64::neg_from(1).cmp(i64::neg_from(2)) == 2, 0);
    }

    #[test]
    fun test_eq() {
        assert!(i64::from(5).eq(i64::from(5)), 0);
        assert!(!i64::from(5).eq(i64::from(3)), 0);
        assert!(!i64::from(5).eq(i64::neg_from(5)), 0);
    }

    #[test]
    fun test_neq() {
        assert!(!i64::from(5).neq(i64::from(5)), 0);
        assert!(i64::from(5).neq(i64::from(3)), 0);
        assert!(i64::from(5).neq(i64::neg_from(5)), 0);
    }

    #[test]
    fun test_into_inner() {
        let (pos, amt) = i64::from(5).into_inner();
        assert!(pos && amt == 5);

        let (pos, amt) = i64::neg_from(5).into_inner();
        assert!(!pos && amt == 5);

        let (pos, amt) = i64::neg_from(0).into_inner();
        assert!(pos && amt == 0);
    }

    #[test]
    fun test_gt_gte() {
        assert!(i64::from(5).gt(i64::from(3)), 0);
        assert!(!i64::from(3).gt(i64::from(5)), 0);
        assert!(!i64::from(5).gt(i64::from(5)), 0);
        assert!(i64::from(5).gt(i64::neg_from(5)), 0);

        assert!(i64::from(5).gte(i64::from(3)), 0);
        assert!(!i64::from(3).gte(i64::from(5)), 0);
        assert!(i64::from(5).gte(i64::from(5)), 0);
        assert!(i64::from(5).gte(i64::neg_from(5)), 0);
    }

    #[test]
    fun test_lt_lte() {
        assert!(!i64::from(5).lt(i64::from(3)), 0);
        assert!(i64::from(3).lt(i64::from(5)), 0);
        assert!(!i64::from(5).lt(i64::from(5)), 0);
        assert!(i64::neg_from(5).lt(i64::from(5)), 0);

        assert!(!i64::from(5).lte(i64::from(3)), 0);
        assert!(i64::from(3).lte(i64::from(5)), 0);
        assert!(i64::from(5).lte(i64::from(5)), 0);
        assert!(i64::neg_from(5).lte(i64::from(5)), 0);
    }
}
