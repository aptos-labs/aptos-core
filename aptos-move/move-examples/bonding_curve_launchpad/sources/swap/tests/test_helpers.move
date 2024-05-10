#[test_only]
module swap::test_helpers {
    use swap::package_manager;
    use swap::liquidity_pool;

    public fun set_up(deployer: &signer) {
        package_manager::initialize_for_test(deployer);
        liquidity_pool::initialize();
    }

}
