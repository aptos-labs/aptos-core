#[test_only]
module aptos_framework::deflation_token_tests {
    use aptos_framework::function_info;
    use aptos_framework::fungible_asset::{Self, Metadata, TestToken};
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_framework::deflation_token;
    use aptos_framework::object;
    use std::option;
    use std::string;

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
        dispatchable_fungible_asset::deposit(creator_store, fa);
        // Withdraw
        let fa = dispatchable_fungible_asset::withdraw(creator, creator_store, 5);
        assert!(fungible_asset::supply(metadata) == option::some(100), 3);
        dispatchable_fungible_asset::deposit(aaron_store, fa);

        // Withdrawing 10 token will cause 1 token to be burned.
        let fa = dispatchable_fungible_asset::withdraw(creator, creator_store, 10);
        assert!(fungible_asset::supply(metadata) == option::some(99), 3);
        dispatchable_fungible_asset::deposit(aaron_store, fa);

        dispatchable_fungible_asset::transfer_fixed_send(creator, creator_store, aaron_store, 10);
        assert!(fungible_asset::balance(aaron_store) == 25, 5);
        assert!(fungible_asset::balance(creator_store) == 74, 5);

        dispatchable_fungible_asset::transfer_fixed_receive(creator, creator_store, aaron_store, 10);
        assert!(fungible_asset::balance(aaron_store) == 35, 5);
        assert!(fungible_asset::balance(creator_store) == 63, 5);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10003, location = aptos_framework::fungible_asset)]
    fun test_deflation_fa_deposit(
        creator: &signer,
    ) {
        let (creator_ref, token_object) = fungible_asset::create_test_token(creator);
        let (mint, _, _) = fungible_asset::init_test_metadata(&creator_ref);
        let metadata = object::convert<TestToken, Metadata>(token_object);

        let creator_store = fungible_asset::create_test_store(creator, metadata);

        deflation_token::initialize(creator, &creator_ref);

        assert!(fungible_asset::supply(metadata) == option::some(0), 1);
        // Mint
        let fa = fungible_asset::mint(&mint, 100);
        assert!(fungible_asset::supply(metadata) == option::some(100), 2);
        // Deposit would fail if using existing FA api.
        fungible_asset::deposit(creator_store, fa);
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    #[expected_failure(abort_code = 0x10003, location = aptos_framework::fungible_asset)]
    fun test_deflation_fa_withdraw(
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
        dispatchable_fungible_asset::deposit(creator_store, fa);
        // Withdraw would fail if using existing FA api.
        let fa = fungible_asset::withdraw(creator, creator_store, 5);
        assert!(fungible_asset::supply(metadata) == option::some(100), 3);
        dispatchable_fungible_asset::deposit(aaron_store, fa);
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    #[expected_failure(abort_code = 0x80005, location = aptos_framework::dispatchable_fungible_asset)]
    fun test_double_init(
        creator: &signer,
    ) {
        let (creator_ref, _) = fungible_asset::create_test_token(creator);
        let (_, _, _) = fungible_asset::init_test_metadata(&creator_ref);
        deflation_token::initialize(creator, &creator_ref);

        let withdraw = function_info::new_function_info(
            @aptos_framework,
            string::utf8(b"deflation_token"),
            string::utf8(b"withdraw"),
        );

        let deposit = function_info::new_function_info(
            @aptos_framework,
            string::utf8(b"deflation_token"),
            string::utf8(b"deposit"),
        );

        // Re-registering the overload function should yield an error
        dispatchable_fungible_asset::register_dispatch_functions(&creator_ref, withdraw, deposit);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10001, location = aptos_framework::dispatchable_fungible_asset)]
    fun test_register_bad_withdraw(
        creator: &signer,
    ) {
        let (creator_ref, _) = fungible_asset::create_test_token(creator);

        let deposit = function_info::new_function_info(
            @aptos_framework,
            string::utf8(b"deflation_token"),
            string::utf8(b"deposit"),
        );

        // Change the deposit and withdraw function. Should give a type mismatch error.
        dispatchable_fungible_asset::register_dispatch_functions(&creator_ref, deposit, deposit);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10002, location = aptos_framework::dispatchable_fungible_asset)]
    fun test_register_bad_deposit(
        creator: &signer,
    ) {
        let (creator_ref, _) = fungible_asset::create_test_token(creator);

        let withdraw = function_info::new_function_info(
            @aptos_framework,
            string::utf8(b"deflation_token"),
            string::utf8(b"withdraw"),
        );

        // Change the deposit and withdraw function. Should give a type mismatch error.
        dispatchable_fungible_asset::register_dispatch_functions(&creator_ref, withdraw, withdraw);
    }

    #[test(creator = @0xcafe)]
    fun test_calling_overloadable_api_on_regular_fa(
        creator: &signer,
    ) {
        let (creator_ref, token_object) = fungible_asset::create_test_token(creator);
        let (mint, _, _) = fungible_asset::init_test_metadata(&creator_ref);
        let metadata = object::convert<TestToken, Metadata>(token_object);

        let creator_store = fungible_asset::create_test_store(creator, metadata);

        assert!(fungible_asset::supply(metadata) == option::some(0), 1);
        // Mint
        let fa = fungible_asset::mint(&mint, 100);
        assert!(fungible_asset::supply(metadata) == option::some(100), 2);
        // Deposit would succeed
        dispatchable_fungible_asset::deposit(creator_store, fa);
    }
}
