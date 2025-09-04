/// This demonstrates how to use a resource group within a single module
/// See resource_groups_primary::secondary for cross module and multiple resources
module resource_groups_primary::primary {
    use std::signer;

    #[resource_group(scope = global)]
    struct ResourceGroupContainer { }

    #[resource_group_member(group = resource_groups_primary::primary::ResourceGroupContainer)]
    struct Primary has drop, key {
        value: u64,
    }

    public entry fun init(account: &signer, value: u64) {
        move_to(account, Primary { value });
    }

    public entry fun set_value(account: &signer, value: u64) acquires Primary {
        let primary = borrow_global_mut<Primary>(signer::address_of(account));
        primary.value = value;
    }

    public fun read(account: address): u64 acquires Primary {
        borrow_global<Primary>(account).value
    }

    public entry fun remove(account: &signer) acquires Primary {
        move_from<Primary>(signer::address_of(account));
    }

    public fun exists_at(account: address): bool {
        exists<Primary>(account)
    }

    fun init_module(owner: &signer) {
        move_to(owner, Primary { value: 3 });
    }

    #[test(account = @0x3)]
    fun test_multiple(account: &signer) acquires Primary {
        // Do it once to verify normal flow
        test_primary(account);

        // Do it again to verify it can be recreated
        test_primary(account);
    }

    public fun test_primary(account: &signer) acquires Primary {
        let addr = signer::address_of(account);
        assert!(!exists_at(addr), 0);
        init(account, 5);
        assert!(read(addr) == 5, 1);
        set_value(account, 12);
        assert!(read(addr) == 12, 1);
        remove(account);
        assert!(!exists_at(addr), 0);
    }
}
