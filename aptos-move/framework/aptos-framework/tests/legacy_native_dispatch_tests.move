#[test_only]
/// Tests exercising the legacy native dispatch path, i.e. with the `FUNCTION_VALUE_DISPATCH`
/// feature disabled. The regular dispatch test suites cover the function-value dispatch path,
/// which is enabled by default in tests.
module aptos_framework::legacy_native_dispatch_tests {
    use aptos_framework::deflation_token;
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_framework::fungible_asset::{Self, Metadata, TestToken};
    use aptos_framework::reentrant_token;
    use aptos_framework::ten_x_token;
    use std::features;
    use std::option;

    fun disable_function_value_dispatch(fx: &signer) {
        features::change_feature_flags_for_testing(
            fx,
            vector[],
            vector[features::get_function_value_dispatch_feature()],
        );
    }

    #[test(fx = @aptos_framework, creator = @aptos_framework, aaron = @0xface)]
    fun test_deflation_native_dispatch(
        fx: &signer,
        creator: &signer,
        aaron: &signer,
    ) {
        disable_function_value_dispatch(fx);

        let (creator_ref, token_object) = fungible_asset::create_test_token(creator);
        let (mint, _, _, _) = fungible_asset::init_test_metadata(&creator_ref);
        let metadata = token_object.convert<TestToken, Metadata>();

        let creator_store = fungible_asset::create_test_store(creator, metadata);
        let aaron_store = fungible_asset::create_test_store(aaron, metadata);

        deflation_token::initialize(creator, &creator_ref);

        let fa = mint.mint(100);
        dispatchable_fungible_asset::deposit(creator_store, fa);

        // Withdrawing 10 tokens will cause 1 token to be burned by the deflation hook.
        let fa = dispatchable_fungible_asset::withdraw(creator, creator_store, 10);
        assert!(fungible_asset::supply(metadata) == option::some(99), 1);
        dispatchable_fungible_asset::deposit(aaron_store, fa);

        assert!(fungible_asset::balance(creator_store) == 89, 2);
        assert!(fungible_asset::balance(aaron_store) == 10, 3);

        dispatchable_fungible_asset::transfer_assert_minimum_deposit(
            creator, creator_store, aaron_store, 10, 10
        );
        assert!(fungible_asset::balance(creator_store) == 78, 4);
        assert!(fungible_asset::balance(aaron_store) == 20, 5);
    }

    #[test(fx = @aptos_framework, creator = @aptos_framework)]
    fun test_derived_hooks_native_dispatch(
        fx: &signer,
        creator: &signer,
    ) {
        disable_function_value_dispatch(fx);

        let (creator_ref, token_object) = fungible_asset::create_test_token(creator);
        let (mint, _, _, _) = fungible_asset::init_test_metadata(&creator_ref);
        let metadata = token_object.convert<TestToken, Metadata>();

        let creator_store = fungible_asset::create_test_store(creator, metadata);

        ten_x_token::initialize(creator, &creator_ref);

        assert!(dispatchable_fungible_asset::derived_supply(metadata) == option::some(0), 1);
        let fa = mint.mint(100);
        dispatchable_fungible_asset::deposit(creator_store, fa);

        // The derived balance and supply are 10x.
        assert!(dispatchable_fungible_asset::derived_balance(creator_store) == 1000, 2);
        assert!(dispatchable_fungible_asset::is_derived_balance_at_least(creator_store, 1000), 3);
        assert!(dispatchable_fungible_asset::derived_supply(metadata) == option::some(1000), 4);
    }

    #[test(fx = @aptos_framework, creator = @aptos_framework, aaron = @0xface)]
    fun test_dispatch_without_function_reflection(
        fx: &signer,
        creator: &signer,
        aaron: &signer,
    ) {
        // Enable FUNCTION_VALUE_DISPATCH then disable its prerequisite FUNCTION_REFLECTION;
        // the conjunction is false so dispatch must fall back to the legacy native path.
        features::change_feature_flags_for_testing(
            fx,
            vector[features::get_function_value_dispatch_feature()],
            vector[features::get_function_reflection_feature()],
        );

        let (creator_ref, token_object) = fungible_asset::create_test_token(creator);
        let (mint, _, _, _) = fungible_asset::init_test_metadata(&creator_ref);
        let metadata = token_object.convert<TestToken, Metadata>();

        let creator_store = fungible_asset::create_test_store(creator, metadata);
        let aaron_store = fungible_asset::create_test_store(aaron, metadata);

        deflation_token::initialize(creator, &creator_ref);

        let fa = mint.mint(100);
        dispatchable_fungible_asset::deposit(creator_store, fa);

        // Withdrawing 10 tokens burns 1 token via the deflation hook.
        dispatchable_fungible_asset::transfer(creator, creator_store, aaron_store, 10);
        assert!(fungible_asset::supply(metadata) == option::some(99), 1);
        assert!(fungible_asset::balance(creator_store) == 89, 2);
        assert!(fungible_asset::balance(aaron_store) == 10, 3);
    }

    #[test(fx = @aptos_framework, creator = @aptos_framework)]
    #[expected_failure(major_status = 4037, location = aptos_framework::dispatchable_fungible_asset)]
    fun test_reentrant_deposit_native_dispatch(
        fx: &signer,
        creator: &signer,
    ) {
        disable_function_value_dispatch(fx);

        let (creator_ref, token_object) = fungible_asset::create_test_token(creator);
        let (mint, _, _, _) = fungible_asset::init_test_metadata(&creator_ref);
        let metadata = token_object.convert<TestToken, Metadata>();

        let creator_store = fungible_asset::create_test_store(creator, metadata);

        reentrant_token::initialize(creator, &creator_ref);

        let fa = mint.mint(100);
        // Deposit causes a re-entrant call into dispatchable_fungible_asset.
        dispatchable_fungible_asset::deposit(creator_store, fa);
    }
}
