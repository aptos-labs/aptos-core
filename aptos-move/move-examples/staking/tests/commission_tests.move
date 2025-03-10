#[test_only]
module staking::commission_tests {
    use std::signer;
    use aptos_framework::account;
    use aptos_framework::aptos_account;
    use aptos_framework::aptos_coin;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    use aptos_framework::timestamp;
    use staking::oracle;
    use staking::commission;

    const ONE_YEAR_IN_SECONDS: u64 = 31536000;
    const OPERATOR: address = @0x124;

    fun set_up() {
        timestamp::set_time_has_started_for_testing(&account::create_signer_for_test(@aptos_framework));
        commission::init_for_test(&account::create_signer_for_test(@0xcafe));
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

    // Test update operator by manager
    #[test(manager = @0x123)]
    fun test_set_operator(manager: &signer) {
        set_up();
        let new_operator = account::create_signer_for_test(@0x234);
        commission::set_operator(manager, signer::address_of(&new_operator));
        assert!(commission::operator() == signer::address_of(&new_operator));
    }

    #[test(manager = @0x123)]
    fun test_distribution(manager: &signer) {
        set_up();
        commission::set_yearly_commission_amount(manager, 1000);
        assert!(commission::commission_owed() == 0);

        // Send APT to the commission contract.
        mint_apt(1100);
        oracle::set_test_price(100000000);

        // Half a year has passed, so the commission owed should be 500.
        timestamp::fast_forward_seconds(ONE_YEAR_IN_SECONDS / 2);
        assert!(commission::commission_owed() == 500);

        // Distribute the commission.
        commission::distribute_commission(manager);
        assert!(commission::commission_owed() == 0);
        assert!(coin::balance<AptosCoin>(signer::address_of(manager)) == 600);
        assert!(coin::balance<AptosCoin>(OPERATOR) == 500);
    }

    #[test(manager = @0x123)]
    fun test_distribution_with_debt(manager: &signer) {
        set_up();
        commission::set_yearly_commission_amount(manager, 1000);
        assert!(commission::commission_owed() == 0);

        // Send APT to the commission contract. But not enough to cover the commission owed
        mint_apt(400);
        oracle::set_test_price(100000000);

        // Half a year has passed, so the commission owed should be 500.
        timestamp::fast_forward_seconds(ONE_YEAR_IN_SECONDS / 2);
        assert!(commission::commission_owed() == 500);

        // Distribute the commission.
        commission::distribute_commission(manager);
        // Debt of 100.
        assert!(commission::commission_owed() == 100, commission::commission_owed());
        assert!(coin::balance<AptosCoin>(signer::address_of(manager)) == 0);
        assert!(coin::balance<AptosCoin>(OPERATOR) == 400);
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
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(
            &account::create_signer_for_test(@aptos_framework));
        aptos_account::deposit_coins(@staking, coin::mint(amount, &mint_cap));
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }
}
