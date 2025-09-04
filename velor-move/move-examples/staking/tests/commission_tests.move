#[test_only]
module staking::commission_tests {
    use std::signer;
    use velor_std::math128;
    use velor_framework::account;
    use velor_framework::velor_account;
    use velor_framework::velor_coin;
    use velor_framework::velor_coin::VelorCoin;
    use velor_framework::coin;
    use velor_framework::timestamp;
    use staking::oracle;
    use staking::commission;

    const ONE_YEAR_IN_SECONDS: u64 = 31536000;
    const OPERATOR: address = @0x124;
    // 1 APT = 5.34378710 USD with 8 decimals of precision
    const APT_PRICE: u128 = 534378710;

    fun set_up() {
        timestamp::set_time_has_started_for_testing(&account::create_signer_for_test(@velor_framework));
        commission::init_for_test(&account::create_signer_for_test(@0xcafe));
        oracle::set_test_price(APT_PRICE);
    }

    #[test(manager = @0x123)]
    fun test_view_yearly_commission_amount(manager: &signer) {
        set_up();
        commission::set_yearly_commission_amount(manager, 1000);
        assert!(commission::yearly_commission_amount() == 1000);
    }

    #[test(manager = @0x123)]
    fun test_view_commission_owed(manager: &signer) {
        set_up();
        commission::set_yearly_commission_amount(manager, 1000);
        assert!(commission::commission_owed() == 0);

        // Half a year has passed, so the commission owed should be 500.
        timestamp::fast_forward_seconds(ONE_YEAR_IN_SECONDS / 2);
        assert!(commission::commission_owed() == 500);

        // Another half a year has passed, so the commission owed should be 1000.
        timestamp::fast_forward_seconds(ONE_YEAR_IN_SECONDS / 2);
        assert!(commission::commission_owed() == 1000);
    }

    #[test(manager = @0x123)]
    fun test_set_operator(manager: &signer) {
        set_up();
        let new_operator = account::create_signer_for_test(@0x234);
        commission::set_operator(manager, signer::address_of(&new_operator));
        assert!(commission::operator() == signer::address_of(&new_operator));
    }

    #[test(manager = @0x123)]
    #[expected_failure(abort_code = staking::commission::EOPERATOR_SAME_AS_OLD)]
    fun test_set_operator_same_account(manager: &signer) {
        set_up();
        commission::set_operator(manager, OPERATOR);
    }

    #[test(manager = @0x123)]
    #[expected_failure(abort_code = staking::commission::EINSUFFICIENT_BALANCE_FOR_DISTRIBUTION)]
    fun test_distribution_with_insufficient_balance(manager: &signer) {
        set_up();
        commission::set_yearly_commission_amount(manager, 100000);
        assert!(commission::commission_owed() == 0);

        // Mint 0.1 APT. Not enough to cover the min balance for distribution.
        mint_apt(10000000);
        timestamp::fast_forward_seconds(ONE_YEAR_IN_SECONDS);
        commission::distribute_commission(manager);
    }

    #[test(manager = @0x123)]
    fun test_distribution_with_no_debt(manager: &signer) {
        set_up();
        // Commission is 100,000 USD per year
        commission::set_yearly_commission_amount(manager, 100000);
        assert!(commission::commission_owed() == 0);

        // Send APT to the commission contract.
        let expected_commission_usd = 50000;
        let expected_apt_amount = usd_to_apt(expected_commission_usd);
        mint_apt(expected_apt_amount * 2);

        // Half a year has passed, so the commission owed should be 50,000 USD or ~9356 APT.
        timestamp::fast_forward_seconds(ONE_YEAR_IN_SECONDS / 2);
        assert!(commission::commission_owed() == expected_commission_usd);
        assert!(commission::commission_owed_in_apt() == expected_apt_amount);

        // Distribute the commission.
        commission::distribute_commission(manager);
        assert!(commission::commission_owed() == 0);
        assert!(coin::balance<VelorCoin>(signer::address_of(manager)) == expected_apt_amount);
        assert!(coin::balance<VelorCoin>(OPERATOR) == expected_apt_amount);
    }

    #[test(manager = @0x123)]
    fun test_distribution_with_debt(manager: &signer) {
        set_up();
        commission::set_yearly_commission_amount(manager, 100000);
        assert!(commission::commission_owed() == 0);

        // Send APT to the commission contract. But not enough to cover the commission owed
        let expected_commission_usd = 50000;
        let expected_apt_amount = usd_to_apt(expected_commission_usd);
        mint_apt(expected_apt_amount / 2);

        // Half a year has passed, so the commission owed should be 50,000.
        timestamp::fast_forward_seconds(ONE_YEAR_IN_SECONDS / 2);
        assert!(commission::commission_owed() == expected_commission_usd);
        assert!(commission::commission_owed_in_apt() == expected_apt_amount);

        // Distribute the commission. Only enough balance to cover half
        commission::distribute_commission(manager);
        // Off by $1 due to rounding error.
        let expected_debt = expected_commission_usd / 2 - 1;
        let expected_debt_in_apt = usd_to_apt(expected_debt);
        assert!(commission::commission_owed() == expected_debt);
        assert!(commission::commission_owed_in_apt() == expected_debt_in_apt);
        assert!(coin::balance<VelorCoin>(signer::address_of(manager)) == 0);
        assert!(coin::balance<VelorCoin>(OPERATOR) == expected_apt_amount / 2);
    }

    #[test]
    #[expected_failure(abort_code = staking::commission::EUNAUTHORIZED)]
    fun test_unauthorized_set_yearly_commission_amount() {
        set_up();
        let unauthorized = account::create_signer_for_test(@0x234);
        commission::set_yearly_commission_amount(&unauthorized, 1000);
    }

    #[test]
    #[expected_failure(abort_code = staking::commission::EUNAUTHORIZED)]
    fun test_unauthorized_set_operator() {
        set_up();
        let unauthorized = account::create_signer_for_test(@0x234);
        commission::set_operator(&unauthorized, OPERATOR);
    }

    fun mint_apt(amount: u64) {
        let (burn_cap, mint_cap) = velor_coin::initialize_for_test(
            &account::create_signer_for_test(@velor_framework));
        velor_account::deposit_coins(@staking, coin::mint(amount, &mint_cap));
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }


    inline fun usd_to_apt(amount_usd: u64): u64 {
        (math128::mul_div((amount_usd as u128) * math128::pow(10, 8), math128::pow(10, 8), APT_PRICE) as u64)
    }
}
