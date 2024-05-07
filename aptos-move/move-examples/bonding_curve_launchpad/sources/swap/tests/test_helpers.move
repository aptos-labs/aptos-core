#[test_only]
module swap::test_helpers {
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::fungible_asset::{Self, FungibleAsset};
    use aptos_framework::object;
    use aptos_framework::primary_fungible_store;
    use std::option;
    use std::string;
    use swap::package_manager;
    use swap::liquidity_pool;

    public fun set_up(deployer: &signer) {
        package_manager::initialize_for_test(deployer);
        liquidity_pool::initialize();
    }

}
