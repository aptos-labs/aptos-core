/// This demonstrates how to use a resource group accross modules
module resource_groups_secondary::secondary {
    use std::signer;
    use resource_groups_primary::primary;

    #[resource_group_member(group = resource_groups_primary::primary::ResourceGroupContainer)]
    struct Secondary has drop, key {
        value: u32,
    }

    public entry fun init(account: &signer, value: u32) {
        move_to(account, Secondary { value });
    }

    public entry fun set_value(account: &signer, value: u32) acquires Secondary {
        let primary = borrow_global_mut<Secondary>(signer::address_of(account));
        primary.value = value;
    }

    public fun read(account: address): u32 acquires Secondary {
        borrow_global<Secondary>(account).value
    }

    public entry fun remove(account: &signer) acquires Secondary {
        move_from<Secondary>(signer::address_of(account));
    }

    public fun exists_at(account: address): bool {
        exists<Secondary>(account)
    }

    // This boiler plate function exists just so that primary is loaded with secondary
    // We'll need to explore how to load resource_group_containers without necessarily
    // having loaded their module via the traditional module graph.
    public fun primary_exists(account: address): bool {
        primary::exists_at(account)
    }

    #[test(account = @0x3)]
    fun test_multiple(account: &signer) acquires Secondary {
        let addr = signer::address_of(account);
        assert!(!exists_at(addr), 0);
        init(account, 7);
        assert!(read(addr) == 7, 1);
        set_value(account, 13);
        assert!(read(addr) == 13, 1);

        // Verify that primary can be added and removed without affecting secondary
        primary::test_primary(account);
        assert!(read(addr) == 13, 1);

        remove(account);
        assert!(!exists_at(addr), 0);

        // Verify that primary can be re-added after secondary has been removed
        primary::test_primary(account);
    }
}
