#[test_only]
module bonding_curve_launchpad::test_liquidity_pairs {
    use bonding_curve_launchpad::liquidity_pairs;

    //---------------------------View Tests---------------------------
    #[test(deployer = @bonding_curve_launchpad)]
    #[expected_failure(abort_code = liquidity_pairs::ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INSIGNIFICANT, location = liquidity_pairs)]
    public fun test_insignificant_fa_swap(deployer: &signer) {
        liquidity_pairs::initialize_for_test(deployer);
        liquidity_pairs::get_amount_out(1_000_000_000, 1_000_000_000, true, 0);
    }

    #[test(deployer = @bonding_curve_launchpad)]
    #[expected_failure(abort_code = liquidity_pairs::ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INSIGNIFICANT, location = liquidity_pairs)]
    public fun test_insignificant_apt_swap(deployer: &signer) {
        liquidity_pairs::initialize_for_test(deployer);
        liquidity_pairs::get_amount_out(1_000_000_000, 1_000_000_000, false, 0);
    }
}
