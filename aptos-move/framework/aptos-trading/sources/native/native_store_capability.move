module aptos_trading::native_store_capability {
    use aptos_framework::big_ordered_map::{BigOrderedMap, Self};

    const ENOT_DEPLOYER: u64 = 1;
    const EAUTHORIZED_ACCOUNT: u64 = 2;

    struct None has drop, copy, store {}

    enum AuthorizedAccounts has key {
        V1 {
            accounts: BigOrderedMap<address, None>,
        }
    }

    enum NativeStoreCapability has key {
        V1 { account: address },
    }

    fun init_module(owner: &signer) {
        assert!(owner.address_of() == @aptos_trading, ENOT_DEPLOYER);
        move_to(owner, AuthorizedAccounts::V1 { accounts: big_ordered_map::new() });
    }

    public fun get_capability(authorized: &signer): NativeStoreCapability {
        let AuthorizedAccounts { accounts } = &AuthorizedAccounts[@aptos_trading];
        assert!(accounts.contains(authorized.address_of()), EAUTHORIZED_ACCOUNT);
        NativeStoreCapability::V1 { account: authorized.address_of() }
    }
}
