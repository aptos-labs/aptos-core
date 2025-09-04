#[test_only]
module velor_framework::clamped_token_tests {
    use velor_framework::fungible_asset::{Self, Metadata, TestToken};
    use velor_framework::dispatchable_fungible_asset;
    use velor_framework::object;
    use 0xcafe::clamped_token;
    use std::option;

    #[test(creator = @0xcafe)]
    fun test_clamped(
        creator: &signer,
    ) {
        let (creator_ref, token_object) = fungible_asset::create_test_token(creator);
        let (mint, _, _, _) = fungible_asset::init_test_metadata(&creator_ref);
        let metadata = object::convert<TestToken, Metadata>(token_object);

        let creator_store = fungible_asset::create_test_store(creator, metadata);

        clamped_token::initialize(creator, &creator_ref);

        assert!(dispatchable_fungible_asset::derived_supply(metadata) == option::some(0), 2);
        // Mint
        let fa = fungible_asset::mint(&mint, 100);
        dispatchable_fungible_asset::deposit(creator_store, fa);

        assert!(dispatchable_fungible_asset::derived_balance(creator_store) == 100, 4);
        assert!(dispatchable_fungible_asset::derived_supply(metadata) == option::some(100), 5);

        let fa = dispatchable_fungible_asset::withdraw(creator, creator_store, 5);
        dispatchable_fungible_asset::deposit(creator_store, fa);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0, location = 0xcafe::clamped_token)]
    fun test_clamped_aborted(
        creator: &signer,
    ) {
        let (creator_ref, token_object) = fungible_asset::create_test_token(creator);
        let (mint, _, _, _) = fungible_asset::init_test_metadata(&creator_ref);
        let metadata = object::convert<TestToken, Metadata>(token_object);

        let creator_store = fungible_asset::create_test_store(creator, metadata);

        clamped_token::initialize(creator, &creator_ref);

        assert!(dispatchable_fungible_asset::derived_supply(metadata) == option::some(0), 2);
        // Mint
        let fa = fungible_asset::mint(&mint, 100);
        dispatchable_fungible_asset::deposit(creator_store, fa);

        // Failed to withdraw as it exceeds the withdraw limit.
        let fa = dispatchable_fungible_asset::withdraw(creator, creator_store, 20);
        dispatchable_fungible_asset::deposit(creator_store, fa);
    }
}
