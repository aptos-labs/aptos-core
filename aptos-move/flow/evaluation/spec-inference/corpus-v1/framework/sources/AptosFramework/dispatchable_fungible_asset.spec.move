spec aptos_framework::dispatchable_fungible_asset {
    spec module {
        pragma verify = false;
    }

    spec dispatchable_withdraw {
        pragma opaque;
    }

    // Opaque, mirroring the natives they replace.

    spec dispatch_withdraw_hook {
        use 0x1::object;
        use 0x1::function_info;
        use 0x1::fungible_asset;
        // Override the module-wide skip: this direct function-value forwarder
        // has a complete behavioral contract and can be checked in isolation.
        pragma verify = true;
        pragma opaque;
        pragma aborts_if_is_partial = false;
        aborts_if aborts_of<function_info::load_function_value<| object::Object<T> , u64, &fungible_asset::TransferRef
            | (fungible_asset::FungibleAsset) has copy + drop>> (function);
        aborts_if !aborts_of<function_info::load_function_value<| object::Object<T> , u64, &fungible_asset::TransferRef
            | (fungible_asset::FungibleAsset) has copy + drop>> (function)
            && aborts_of<result_of<function_info::load_function_value<| object::Object<T> , u64, &fungible_asset::TransferRef
                | (fungible_asset::FungibleAsset) has copy + drop>> (function) >(
                store, amount, transfer_ref
            );
        ensures !aborts_of<function_info::load_function_value<| object::Object<T> , u64, &fungible_asset::TransferRef
            | (fungible_asset::FungibleAsset) has copy + drop>> (function)
            && !aborts_of<result_of<function_info::load_function_value<| object::Object<T> , u64, &fungible_asset::TransferRef
                | (fungible_asset::FungibleAsset) has copy + drop>> (function) >(
                store, amount, transfer_ref
            ) ==>
            result
                == result_of<result_of<function_info::load_function_value<| object::Object<T> , u64, &fungible_asset::TransferRef
                    | (fungible_asset::FungibleAsset) has copy + drop>> (function) >(
                    store, amount, transfer_ref
                );
    }

    spec dispatch_deposit_hook {
        use 0x1::object;
        use 0x1::function_info;
        use 0x1::fungible_asset;
        // See `dispatch_withdraw_hook`: the selected function-level override
        // is intentional while unrelated module contracts remain deferred.
        pragma verify = true;
        pragma opaque;
        pragma aborts_if_is_partial = false;
        aborts_if aborts_of<function_info::load_function_value<| object::Object<T> , fungible_asset::FungibleAsset, &fungible_asset::TransferRef
            | has copy + drop>> (function);
        aborts_if !aborts_of<function_info::load_function_value<| object::Object<T> , fungible_asset::FungibleAsset, &fungible_asset::TransferRef
            | has copy + drop>> (function)
            && aborts_of<result_of<function_info::load_function_value<| object::Object<T> , fungible_asset::FungibleAsset, &fungible_asset::TransferRef
                | has copy + drop>> (function) >(store, fa, transfer_ref);
        ensures !aborts_of<function_info::load_function_value<| object::Object<T> , fungible_asset::FungibleAsset, &fungible_asset::TransferRef
            | has copy + drop>> (function)
            && !aborts_of<result_of<function_info::load_function_value<| object::Object<T> , fungible_asset::FungibleAsset, &fungible_asset::TransferRef
                | has copy + drop>> (function) >(store, fa, transfer_ref) ==>
            ensures_of<result_of<function_info::load_function_value<| object::Object<T> , fungible_asset::FungibleAsset, &fungible_asset::TransferRef
                | has copy + drop>> (function) >(store, fa, transfer_ref);
    }

    spec dispatch_derived_balance_hook {
        use 0x1::object;
        use 0x1::function_info;
        pragma opaque;
        pragma aborts_if_is_partial = true;
        ensures [inferred = sathard] result
            == {
                let a =
                    ..S1 |~ result_of<function_info::load_function_value<| object::Object<T>
                        | (u64) has copy + drop>> (function);
                S1.. |~ result_of<a>(store)
            };
    }

    spec dispatch_derived_supply_hook {
        use 0x1::option;
        use 0x1::object;
        use 0x1::function_info;
        pragma opaque;
        pragma aborts_if_is_partial = true;
        ensures [inferred = sathard] result
            == {
                let a =
                    ..S1 |~ result_of<function_info::load_function_value<| object::Object<T>
                        | (option::Option<u128>) has copy + drop>> (function);
                S1.. |~ result_of<a>(metadata)
            };
    }

    spec dispatchable_deposit {
        pragma opaque;
    }

    spec dispatchable_derived_balance {
        pragma opaque;
    }

    spec dispatchable_derived_supply {
        pragma opaque;
    }

    spec withdraw {
        use 0x1::fungible_asset;
        pragma verify = true;
        modifies global<fungible_asset::FungibleStore>(
            aptos_framework::object::object_address(store)
        );
        modifies global<fungible_asset::ConcurrentFungibleBalance>(
            aptos_framework::object::object_address(store)
        );
        pragma opaque = true, aborts_if_is_partial = false;
        ensures [inferred]..S1 |~(
            ensures_of<fungible_asset::withdraw_sanity_check<T>> (owner, store, false)
        );
        aborts_if [inferred] S1 |~(
            aborts_of<fungible_asset::withdraw_dispatch_function<T>> (store)
        );
        aborts_if aborts_of<fungible_asset::withdraw_sanity_check<T>> (owner, store, false);
    }

    spec deposit {
        use 0x1::fungible_asset;
        pragma verify = true;
        modifies global<fungible_asset::FungibleStore>(
            aptos_framework::object::object_address(store)
        );
        modifies global<fungible_asset::ConcurrentFungibleBalance>(
            aptos_framework::object::object_address(store)
        );
        pragma opaque = true, aborts_if_is_partial = false;
        ensures [inferred]..S1 |~(
            ensures_of<fungible_asset::deposit_sanity_check<T>> (store, false)
        );
        aborts_if [inferred] S1 |~(
            aborts_of<fungible_asset::deposit_dispatch_function<T>> (store)
        );
        aborts_if [inferred] aborts_of<fungible_asset::deposit_sanity_check<T>> (
            store, false
        );
    }

    spec transfer<T: key>(
        sender: &signer,
        from: 0x1::object::Object<T>,
        to: 0x1::object::Object<T>,
        amount: u64
    ) {
        use 0x1::object;
        use 0x1::fungible_asset;
        pragma opaque = true, aborts_if_is_partial = true;
        modifies fungible_asset::FungibleStore[object::object_address<T>(to)];
        modifies fungible_asset::FungibleStore[object::object_address<T>(from)];
        ensures [inferred = sathard]({
            let a = ..S1 |~ result_of<withdraw<T>> (sender, from, amount);
            S1.. |~ ensures_of<deposit<T>> (to, a)
        });
    }

    spec register_derive_supply_dispatch_function(
        constructor_ref: &0x1::object::ConstructorRef,
        dispatch_function: 0x1::option::Option<0x1::function_info::FunctionInfo>
    ) {
        use 0x1::signer;
        use 0x1::object;
        use 0x1::fungible_asset;
        pragma opaque = true;
        modifies fungible_asset::DeriveSupply[
            signer::address_of(result_of<object::generate_signer>(constructor_ref))
        ];
        ensures [inferred] ensures_of<fungible_asset::register_derive_supply_dispatch_function>(
            constructor_ref, dispatch_function
        );
        aborts_if [inferred] aborts_of<fungible_asset::register_derive_supply_dispatch_function>(
            constructor_ref, dispatch_function
        );
    }

    spec register_dispatch_functions(
        constructor_ref: &0x1::object::ConstructorRef,
        withdraw_function: 0x1::option::Option<0x1::function_info::FunctionInfo>,
        deposit_function: 0x1::option::Option<0x1::function_info::FunctionInfo>,
        derived_balance_function: 0x1::option::Option<0x1::function_info::FunctionInfo>
    ) {
        use 0x1::signer;
        use 0x1::object;
        use 0x1::fungible_asset;
        pragma opaque = true, aborts_if_is_partial = true;
        modifies TransferRefStore[
            signer::address_of(result_of<object::generate_signer>(constructor_ref))
        ];
        ensures [inferred] ensures_of<fungible_asset::register_dispatch_functions>(
            constructor_ref,
            withdraw_function,
            deposit_function,
            derived_balance_function
        );
        aborts_if [inferred] aborts_of<fungible_asset::register_dispatch_functions>(
            constructor_ref,
            withdraw_function,
            deposit_function,
            derived_balance_function
        );
    }

    spec derived_balance<T: key>(store: 0x1::object::Object<T>): u64 {
        use 0x1::fungible_asset;
        pragma opaque = true, aborts_if_is_partial = true;
        aborts_if [inferred] aborts_of<fungible_asset::derived_balance_dispatch_function<T
            >> (store);
    }

    spec derived_balance_snapshot<T: key>(
        store: 0x1::object::Object<T>
    ): 0x1::aggregator_v2::AggregatorSnapshot<u64> {
        use 0x1::fungible_asset;
        pragma opaque = true, aborts_if_is_partial = true;
        aborts_if [inferred] aborts_of<fungible_asset::derived_balance_dispatch_function<T
            >> (store);
    }

    spec is_derived_balance_at_least<T: key>(
        store: 0x1::object::Object<T>, amount: u64
    ): bool {
        use 0x1::fungible_asset;
        pragma opaque = true, aborts_if_is_partial = true;
        aborts_if [inferred] aborts_of<fungible_asset::derived_balance_dispatch_function<T
            >> (store);
    }

    spec transfer_assert_minimum_deposit<T: key>(
        sender: &signer,
        from: 0x1::object::Object<T>,
        to: 0x1::object::Object<T>,
        amount: u64,
        expected: u64
    ) {
        use 0x1::fungible_asset;
        pragma opaque = true, aborts_if_is_partial = true;
        aborts_if [inferred = sathard] aborts_of<fungible_asset::balance<T>> (to);
    }
}
