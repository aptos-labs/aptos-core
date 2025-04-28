#[test_only]
module 0xcafe::ten_x_token {
    use aptos_framework::fungible_asset::{Self, FungibleAsset, RawBalanceRef, RawSupplyRef, TransferRef};
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_framework::object::{ConstructorRef, Object};
    use aptos_framework::function_info;

    use std::option;
    use std::option::Option;
    use std::signer;
    use std::string;

    struct BalanceStore has key {
        balance_ref: RawBalanceRef,
        supply_ref: RawSupplyRef,
    }

    public fun initialize(account: &signer, constructor_ref: &ConstructorRef) {
        assert!(signer::address_of(account) == @0xcafe, 1);
        let balance_ref = fungible_asset::generate_raw_balance_ref(constructor_ref);
        let supply_ref = fungible_asset::generate_raw_supply_ref(constructor_ref);
        move_to<BalanceStore>(account, BalanceStore { balance_ref, supply_ref });

        let balance_value = function_info::new_function_info(
            account,
            string::utf8(b"ten_x_token"),
            string::utf8(b"derived_balance"),
        );
        let supply_value = function_info::new_function_info(
            account,
            string::utf8(b"ten_x_token"),
            string::utf8(b"derived_supply"),
        );

        let withdraw = function_info::new_function_info(
            account,
            string::utf8(b"ten_x_token"),
            string::utf8(b"withdraw"),
        );

        let deposit = function_info::new_function_info(
            account,
            string::utf8(b"ten_x_token"),
            string::utf8(b"deposit"),
        );

        dispatchable_fungible_asset::register_dispatch_functions(
            constructor_ref,
            option::some(withdraw),
            option::some(deposit),
            option::some(balance_value)
        );
        dispatchable_fungible_asset::register_derive_supply_dispatch_function(
            constructor_ref,
            option::some(supply_value)
        );
    }

    public fun derived_balance<T: key>(store: Object<T>): u64 acquires BalanceStore {
        // Derived value is always 10x!
        fungible_asset::balance_with_ref(
            &borrow_global<BalanceStore>(@0xcafe).balance_ref,
            store
        ) * 10
    }

    public fun derived_supply<T: key>(metadata: Object<T>): Option<u128> acquires BalanceStore {
        // Derived supply is 10x.
        option::some(option::extract(&mut fungible_asset::supply_with_ref(
            &borrow_global<BalanceStore>(@0xcafe).supply_ref,
            metadata
        )) * 10)
    }

    public fun withdraw<T: key>(
        store: Object<T>,
        amount: u64,
        transfer_ref: &TransferRef,
    ): FungibleAsset {
        fungible_asset::withdraw_with_ref(transfer_ref, store, amount)
    }

    public fun deposit<T: key>(
        store: Object<T>,
        fa: FungibleAsset,
        transfer_ref: &TransferRef,
    ) {
        fungible_asset::deposit_with_ref(transfer_ref, store, fa)
    }
}
