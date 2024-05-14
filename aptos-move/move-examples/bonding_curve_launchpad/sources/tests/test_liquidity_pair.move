#[test_only]
module resource_account::test_liquidity_pair {
    use resource_account::liquidity_pair;

    //---------------------------View Tests---------------------------
    #[test(deployer = @resource_account)]
    #[expected_failure(abort_code = 111, location = liquidity_pair)]
    public fun test_insignificant_fa_swap(deployer: &signer) {
        liquidity_pair::initialize_for_test(deployer);
        liquidity_pair::get_amount_out(1_000_000_000, 1_000_000_000, true, 0);
    }
    #[test(deployer = @resource_account)]
    #[expected_failure(abort_code = 111, location = liquidity_pair)]
    public fun test_insignificant_apt_swap(deployer: &signer) {
        liquidity_pair::initialize_for_test(deployer);
        liquidity_pair::get_amount_out(1_000_000_000, 1_000_000_000, false, 0);
    }

}
