#[test_only]
module 0xcafe::deflation_token_tests {
    use aptos_framework::function_info;
    use aptos_framework::fungible_asset::{Self, Metadata, TestToken};
    use aptos_framework::dispatchable_fungible_asset;
    use 0xcafe::deflation_token;
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

        // Fix receive API will work because the tax is asserted on the caller side.
        dispatchable_fungible_asset::transfer_fixed_receive(creator, creator_store, aaron_store, 10);
        assert!(fungible_asset::balance(aaron_store) == 25, 5);
        assert!(fungible_asset::balance(creator_store) == 73, 5);

        // Derived value should be the same as balance.
        assert!(dispatchable_fungible_asset::derived_balance(aaron_store) == 25, 5);
        assert!(dispatchable_fungible_asset::derived_balance(creator_store) == 73, 5);
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    #[expected_failure(abort_code = 0x70002, location = aptos_framework::dispatchable_fungible_asset)]
    fun test_deflation_failed_fixed_send(
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

        // Fix send API will fail because the sender balance is not as expected
        dispatchable_fungible_asset::transfer_fixed_send(creator, creator_store, aaron_store, 10);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x1001A, location = aptos_framework::fungible_asset)]
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
    #[expected_failure(abort_code = 0x1001A, location = aptos_framework::fungible_asset)]
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
    #[expected_failure(abort_code = 0x8001B, location = aptos_framework::fungible_asset)]
    fun test_double_init(
        creator: &signer,
    ) {
        let (creator_ref, _) = fungible_asset::create_test_token(creator);
        let (_, _, _) = fungible_asset::init_test_metadata(&creator_ref);
        deflation_token::initialize(creator, &creator_ref);

        let withdraw = function_info::new_function_info(
            @0xcafe,
            string::utf8(b"deflation_token"),
            string::utf8(b"withdraw"),
        );

        let deposit = function_info::new_function_info(
            @0xcafe,
            string::utf8(b"deflation_token"),
            string::utf8(b"deposit"),
        );

        let value = function_info::new_function_info(
            @0xcafe,
            string::utf8(b"deflation_token"),
            string::utf8(b"derived_balance"),
        );

        // Re-registering the overload function should yield an error
        dispatchable_fungible_asset::register_dispatch_functions(&creator_ref, withdraw, deposit, value);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10017, location = aptos_framework::fungible_asset)]
    fun test_register_bad_withdraw(
        creator: &signer,
    ) {
        let (creator_ref, _) = fungible_asset::create_test_token(creator);

        let deposit = function_info::new_function_info(
            @0xcafe,
            string::utf8(b"deflation_token"),
            string::utf8(b"deposit"),
        );

        let value = function_info::new_function_info(
            @0xcafe,
            string::utf8(b"deflation_token"),
            string::utf8(b"derived_balance"),
        );

        // Change the deposit and withdraw function. Should give a type mismatch error.
        dispatchable_fungible_asset::register_dispatch_functions(&creator_ref, deposit, deposit, value);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10018, location = aptos_framework::fungible_asset)]
    fun test_register_bad_deposit(
        creator: &signer,
    ) {
        let (creator_ref, _) = fungible_asset::create_test_token(creator);

        let withdraw = function_info::new_function_info(
            @0xcafe,
            string::utf8(b"deflation_token"),
            string::utf8(b"withdraw"),
        );

        // Change the deposit and withdraw function. Should give a type mismatch error.
        dispatchable_fungible_asset::register_dispatch_functions(&creator_ref, withdraw, withdraw, withdraw);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10019, location = aptos_framework::fungible_asset)]
    fun test_register_bad_value(
        creator: &signer,
    ) {
        let (creator_ref, _) = fungible_asset::create_test_token(creator);

        let withdraw = function_info::new_function_info(
            @0xcafe,
            string::utf8(b"deflation_token"),
            string::utf8(b"withdraw"),
        );

        let deposit = function_info::new_function_info(
            @0xcafe,
            string::utf8(b"deflation_token"),
            string::utf8(b"deposit"),
        );

        // Change the deposit and withdraw function. Should give a type mismatch error.
        dispatchable_fungible_asset::register_dispatch_functions(&creator_ref, withdraw, deposit, deposit);
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
