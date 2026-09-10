spec aptos_framework::dispatchable_fungible_asset {
    /// The balance a store reports through its dispatch hook, if any: the hook
    /// is user code, so the summary is uninterpreted and only its stability is
    /// promised.
    spec fun spec_derived_balance_at(store_addr: address): u64;

    spec derived_balance<T: key>(store: Object<T>): u64 {
        pragma opaque;
        pragma aborts_if_is_partial;
        ensures result == spec_derived_balance_at(object::object_address(store));
    }

    spec module {
        pragma verify = false;
    }

    spec dispatchable_withdraw {
        pragma opaque;
    }

    // Opaque, mirroring the natives they replace.

    spec dispatch_withdraw_hook {
        pragma opaque;
    }

    spec dispatch_deposit_hook {
        pragma opaque;
    }

    spec dispatch_derived_balance_hook {
        pragma opaque;
    }

    spec dispatch_derived_supply_hook {
        pragma opaque;
    }

    spec dispatchable_deposit {
        pragma opaque;
    }

    spec dispatchable_derived_balance{
        pragma opaque;
    }

    spec dispatchable_derived_supply{
        pragma opaque;
    }

    spec withdraw {
        modifies global<fungible_asset::FungibleStore>(aptos_framework::object::object_address(store));
        modifies global<fungible_asset::ConcurrentFungibleBalance>(aptos_framework::object::object_address(store));
    }

    spec deposit {
        modifies global<fungible_asset::FungibleStore>(aptos_framework::object::object_address(store));
        modifies global<fungible_asset::ConcurrentFungibleBalance>(aptos_framework::object::object_address(store));
    }
}
