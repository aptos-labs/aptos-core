spec aptos_framework::dispatchable_fungible_asset {
    spec module {
        pragma verify = false;
    }

    spec dispatchable_withdraw {
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
