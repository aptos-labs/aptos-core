#[test_only]
module bonding_curve_launchpad::test_liquidity_pair {
    use bonding_curve_launchpad::liquidity_pair;

    //---------------------------View Tests---------------------------
    #[test(deployer = @bonding_curve_launchpad)]
    #[expected_failure(abort_code = 111, location = liquidity_pair)]
    public fun test_insignificant_fa_swap(deployer: &signer) {
        liquidity_pair::initialize_for_test(deployer);
        liquidity_pair::get_amount_out(1_000_000_000, 1_000_000_000, true, 0);
    }
    #[test(deployer = @bonding_curve_launchpad)]
    #[expected_failure(abort_code = 111, location = liquidity_pair)]
    public fun test_insignificant_apt_swap(deployer: &signer) {
        liquidity_pair::initialize_for_test(deployer);
        liquidity_pair::get_amount_out(1_000_000_000, 1_000_000_000, false, 0);
    }

}
