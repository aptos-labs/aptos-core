spec velor_framework::dispatchable_fungible_asset {
    use velor_framework::permissioned_signer;
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
        modifies global<permissioned_signer::PermissionStorage>(permissioned_signer::spec_permission_address(owner));
        modifies global<fungible_asset::FungibleStore>(object::object_address(store));
        modifies global<fungible_asset::ConcurrentFungibleBalance>(object::object_address(store));
    }

    spec deposit {
        modifies global<fungible_asset::FungibleStore>(object::object_address(store));
        modifies global<fungible_asset::ConcurrentFungibleBalance>(object::object_address(store));
    }
}
