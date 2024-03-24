#[test_only]
module aptos_framework::deflation_token_tests {
    use aptos_framework::fungible_asset::{Self, Metadata, TestToken};
    use aptos_framework::overloadable_fungible_asset;
    use aptos_framework::deflation_token;
    use aptos_framework::object;
    use std::option;

    #[test(creator = @0xcafe, aaron = @0xface)]
    fun test_deflation_e2e_basic_flow(
        creator: &signer,
        aaron: &signer,
    ) {
        let (creator_ref, token_object) = fungible_asset::create_test_token(creator);
        let (mint, _, _) = fungible_asset::init_test_metadata(&creator_ref);
        let metadata = object::convert<TestToken, Metadata>(token_object);

        let creator_store = fungible_asset::create_test_store(creator, metadata);
        let aaron_store = fungible_asset::create_test_store(aaron, metadata);

        deflation_token::initialize(creator, &creator_ref);

        assert!(fungible_asset::supply(metadata) == option::some(0), 1);
        // Mint
        let fa = fungible_asset::mint(&mint, 100);
        assert!(fungible_asset::supply(metadata) == option::some(100), 2);
        // Deposit
        overloadable_fungible_asset::deposit(creator_store, fa);
        // Withdraw
        let fa = overloadable_fungible_asset::withdraw(creator, creator_store, 5);
        assert!(fungible_asset::supply(metadata) == option::some(100), 3);
        overloadable_fungible_asset::deposit(aaron_store, fa);

        // Withdrawing 10 token will cause 1 token to be burned.
        let fa = overloadable_fungible_asset::withdraw(creator, creator_store, 10);
        assert!(fungible_asset::supply(metadata) == option::some(99), 3);
        overloadable_fungible_asset::deposit(aaron_store, fa);
    }
}
