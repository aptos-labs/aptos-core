#[test_only]
module aptos_framework::deflation_token {
    use aptos_framework::fungible_asset::{Self, BurnRef, FungibleAsset, TransferRef};
    use aptos_framework::overloadable_fungible_asset;
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
            @aptos_framework,
            string::utf8(b"deflation_token"),
            string::utf8(b"withdraw"),
        );

        let deposit = function_info::new_function_info(
            @aptos_framework,
            string::utf8(b"deflation_token"),
            string::utf8(b"deposit"),
        );
        overloadable_fungible_asset::register_overload_functions(constructor_ref, withdraw, deposit);
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
}
