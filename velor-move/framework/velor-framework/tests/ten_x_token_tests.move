#[test_only]
module velor_framework::ten_x_token_tests {
    use velor_framework::fungible_asset::{Self, Metadata, TestToken};
    use velor_framework::dispatchable_fungible_asset;
    use velor_framework::primary_fungible_store;
    use velor_framework::object;
    use 0xcafe::ten_x_token;
    use std::option;
    use std::signer;

    #[test(creator = @0xcafe)]
    fun test_ten_x(
        creator: &signer,
    ) {
        let (creator_ref, token_object) = fungible_asset::create_test_token(creator);
        let (mint, _, _, _) = fungible_asset::init_test_metadata(&creator_ref);
        let metadata = object::convert<TestToken, Metadata>(token_object);

        let creator_store = fungible_asset::create_test_store(creator, metadata);

        ten_x_token::initialize(creator, &creator_ref);

        assert!(dispatchable_fungible_asset::derived_supply(metadata) == option::some(0), 2);
        // Mint
        let fa = fungible_asset::mint(&mint, 100);
        dispatchable_fungible_asset::deposit(creator_store, fa);

        // The derived value is 10x
        assert!(dispatchable_fungible_asset::derived_balance(creator_store) == 1000, 4);

        // The derived supply is 10x
        assert!(dispatchable_fungible_asset::derived_supply(metadata) == option::some(1000), 5);
    }

    #[test(creator = @0xcafe)]
    fun test_ten_x_pfs(
        creator: &signer,
    ) {
        let (creator_ref, token_object) = fungible_asset::create_test_token(creator);
        let (mint, _, _) = primary_fungible_store::init_test_metadata_with_primary_store_enabled(&creator_ref);
        let metadata = object::convert<TestToken, Metadata>(token_object);

        ten_x_token::initialize(creator, &creator_ref);
        let creator_address = signer::address_of(creator);

        let fa = fungible_asset::mint(&mint, 100);
        primary_fungible_store::deposit(creator_address, fa);

        // The derived value is 10x
        assert!(primary_fungible_store::balance(creator_address, metadata) == 1000, 4);
        assert!(primary_fungible_store::is_balance_at_least(creator_address, metadata, 1000), 4);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code=0x1001C, location=velor_framework::fungible_asset)]
    fun ten_x_balance_abort(
        creator: &signer,
    ) {
        let (creator_ref, token_object) = fungible_asset::create_test_token(creator);
        fungible_asset::init_test_metadata(&creator_ref);
        let metadata = object::convert<TestToken, Metadata>(token_object);

        let creator_store = fungible_asset::create_test_store(creator, metadata);

        ten_x_token::initialize(creator, &creator_ref);
        assert!(fungible_asset::balance(creator_store) == 0, 1);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code=0x1001C, location=velor_framework::fungible_asset)]
    fun ten_x_supply_abort(
        creator: &signer,
    ) {
        let (creator_ref, token_object) = fungible_asset::create_test_token(creator);
        fungible_asset::init_test_metadata(&creator_ref);
        let metadata = object::convert<TestToken, Metadata>(token_object);

        ten_x_token::initialize(creator, &creator_ref);

        assert!(fungible_asset::supply(metadata) == option::some(0), 2);
    }
}
