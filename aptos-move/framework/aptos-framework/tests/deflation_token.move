#[test_only]
module 0xcafe::deflation_token {
    use aptos_framework::fungible_asset::{Self, BurnRef, FungibleAsset, TransferRef};
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_framework::object::{ConstructorRef, Object};
    use aptos_framework::function_info;

    use std::signer;
    use std::string;

    struct BurnStore has key {
        burn_ref: BurnRef,
    }

    public fun initialize(account: &signer, constructor_ref: &ConstructorRef) {
        let burn_ref = fungible_asset::generate_burn_ref(constructor_ref);

        assert!(signer::address_of(account) == @0xcafe, 1);
        move_to<BurnStore>(account, BurnStore { burn_ref });

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
        dispatchable_fungible_asset::register_dispatch_functions(constructor_ref, withdraw, deposit, value);
    }

    public fun withdraw<T: key>(
        _owner: address,
        store: Object<T>,
        amount: u64,
        transfer_ref: &TransferRef,
    ): FungibleAsset acquires BurnStore {
        // For every withdraw, we burn 10% from the store.
        let burn_amount = amount / 10;
        if (burn_amount > 0) {
            fungible_asset::burn_from(&borrow_global<BurnStore>(@0xcafe).burn_ref, store, burn_amount);
        };

        fungible_asset::withdraw_with_ref(transfer_ref, store, amount)
    }

    public fun deposit<T: key>(
        store: Object<T>,
        fa: FungibleAsset,
        transfer_ref: &TransferRef,
    ) {
        fungible_asset::deposit_with_ref(transfer_ref, store, fa);
    }

    public fun derived_balance<T: key>(store: Object<T>): u64 {
        fungible_asset::balance(store)
    }

    #[test_only]
    use std::option;
    #[test_only]
    use aptos_framework::object;
    #[test_only]
    use aptos_framework::fungible_asset::{Metadata, TestToken};

    #[test(creator = @0xcafe)]
    #[expected_failure(major_status=4037, location=aptos_framework::dispatchable_fungible_asset)]
    fun test_self_reentrancy(
        creator: &signer,
    ) {
        let (creator_ref, token_object) = fungible_asset::create_test_token(creator);
        let (mint, _, _) = fungible_asset::init_test_metadata(&creator_ref);
        let metadata = object::convert<TestToken, Metadata>(token_object);

        let creator_store = fungible_asset::create_test_store(creator, metadata);

        initialize(creator, &creator_ref);

        assert!(fungible_asset::supply(metadata) == option::some(0), 1);
        // Mint
        let fa = fungible_asset::mint(&mint, 100);
        assert!(fungible_asset::supply(metadata) == option::some(100), 2);
        // Deposit will cause an re-entrant call into self module.
        dispatchable_fungible_asset::deposit(creator_store, fa);
    }
}
