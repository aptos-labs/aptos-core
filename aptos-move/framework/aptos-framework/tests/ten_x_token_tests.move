#[test_only]
module aptos_framework::ten_x_token_tests {
    use aptos_framework::fungible_asset::{Self, Metadata, TestToken};
    use aptos_framework::dispatchable_fungible_asset;
    use 0xcafe::ten_x_token;
    use aptos_framework::object;
    use std::option;

    #[test(creator = @0xcafe)]
    fun test_ten_x(
        creator: &signer,
    ) {
        let (creator_ref, token_object) = fungible_asset::create_test_token(creator);
        let (mint, _, _) = fungible_asset::init_test_metadata(&creator_ref);
        let metadata = object::convert<TestToken, Metadata>(token_object);

        let creator_store = fungible_asset::create_test_store(creator, metadata);

        ten_x_token::initialize(creator, &creator_ref);

        assert!(fungible_asset::supply(metadata) == option::some(0), 1);
        // Mint
        let fa = fungible_asset::mint(&mint, 100);
        assert!(fungible_asset::supply(metadata) == option::some(100), 2);
        // Deposit will cause an re-entrant call into dispatchable_fungible_asset
        dispatchable_fungible_asset::deposit(creator_store, fa);

        // The derived value is 10x
        assert!(dispatchable_fungible_asset::derived_balance(creator_store) == 1000, 5);
    }
}
