#[test_only]
module 0xcafe::reentrant_token {
    use aptos_framework::fungible_asset::{Self, FungibleAsset, TransferRef};
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_framework::object::{ConstructorRef, Object};
    use aptos_framework::function_info;

    use std::string;

    public fun initialize(_account: &signer, constructor_ref: &ConstructorRef) {
        let withdraw = function_info::new_function_info(
            @0xcafe,
            string::utf8(b"reentrant_token"),
            string::utf8(b"withdraw"),
        );

        let deposit = function_info::new_function_info(
            @0xcafe,
            string::utf8(b"reentrant_token"),
            string::utf8(b"deposit"),
        );

        let value = function_info::new_function_info(
            @0xcafe,
            string::utf8(b"reentrant_token"),
            string::utf8(b"derived_balance"),
        );
        dispatchable_fungible_asset::register_dispatch_functions(constructor_ref, withdraw, deposit, value);
    }

    public fun withdraw<T: key>(
        _owner: address,
        store: Object<T>,
        amount: u64,
        transfer_ref: &TransferRef,
    ): FungibleAsset {
        fungible_asset::withdraw_with_ref(transfer_ref, store, amount)
    }

    public fun deposit<T: key>(
        store: Object<T>,
        fa: FungibleAsset,
        _transfer_ref: &TransferRef,
    ) {
        // Re-entering into dispatchable_fungible_asset. Will be rejected by the MoveVM runtime.
        dispatchable_fungible_asset::deposit(store, fa);
    }

    public fun derived_balance<T: key>(store: Object<T>): u64 {
        fungible_asset::balance(store)
    }
}
