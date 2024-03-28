module publisher::full_coverage {
    use aptos_std::math64;

    /// Requested withdrawal amount exceeds expected value of pool.
    const E_WITHDRAWAL_TOO_BIG: u64 = 0;

    public fun socialize_withdrawal_amount(
        requested_withdrawal_amount: u64,
        expected_value_in_pool: u64,
        actual_value_in_pool: u64,
    ): u64 {
        assert!(requested_withdrawal_amount <= expected_value_in_pool, E_WITHDRAWAL_TOO_BIG);
        if (actual_value_in_pool > expected_value_in_pool) {
            requested_withdrawal_amount
        } else if (requested_withdrawal_amount == 0) {
            0
        } else {
            math64::mul_div_unchecked( // <---------- Can be tested to 100% coverage
                requested_withdrawal_amount,
                actual_value_in_pool,
                expected_value_in_pool,
            )
        }
    }

    #[test]
    fun test_socialize_withdrawal_amount() {
        assert!(socialize_withdrawal_amount(100, 200, 200) == 100, 0);
        assert!(socialize_withdrawal_amount(100, 200, 201) == 100, 0);
        assert!(socialize_withdrawal_amount(100, 200, 0) == 0, 0);
        assert!(socialize_withdrawal_amount(100, 200, 100) == 50, 0);
        assert!(socialize_withdrawal_amount(0, 0, 25) == 0, 0);
        assert!(socialize_withdrawal_amount(0, 0, 0) == 0, 0);
    }

    #[test, expected_failure(abort_code = E_WITHDRAWAL_TOO_BIG)]
    fun test_socialize_withdrawal_amount_withdrawal_too_big() {
        socialize_withdrawal_amount(1, 0, 0);
    }
}