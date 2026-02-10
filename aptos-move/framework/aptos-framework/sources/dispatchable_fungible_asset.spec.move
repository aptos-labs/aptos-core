spec aptos_framework::dispatchable_fungible_asset {
    use aptos_framework::permissioned_signer;
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
        modifies global<fungible_asset::FungibleStore>(store.object_address());
        modifies global<fungible_asset::ConcurrentFungibleBalance>(store.object_address());
    }

    spec deposit {
        modifies global<fungible_asset::FungibleStore>(store.object_address());
        modifies global<fungible_asset::ConcurrentFungibleBalance>(store.object_address());
    }
}
