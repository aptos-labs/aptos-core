spec aptos_framework::dispatchable_fungible_asset {
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
        modifies global<fungible_asset::FungibleStore>(object::object_address(store));
        modifies global<fungible_asset::ConcurrentFungibleBalance>(object::object_address(store));
    }

    spec deposit {
        modifies global<fungible_asset::FungibleStore>(object::object_address(store));
        modifies global<fungible_asset::ConcurrentFungibleBalance>(object::object_address(store));
    }
}
