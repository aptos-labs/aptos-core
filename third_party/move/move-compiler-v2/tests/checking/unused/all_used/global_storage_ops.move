// Test that structs used in global storage operations are tracked.
module 0x42::m {
    struct MyResource has key, drop { value: u64 }

    public fun test_exists(addr: address): bool {
        exists<MyResource>(addr)
    }

    public fun test_borrow_global(addr: address): u64 acquires MyResource {
        borrow_global<MyResource>(addr).value
    }

    public fun test_borrow_global_mut(addr: address) acquires MyResource {
        borrow_global_mut<MyResource>(addr).value = 100;
    }

    public fun test_move_from(addr: address): MyResource acquires MyResource {
        move_from<MyResource>(addr)
    }

    public fun test_move_to(account: &signer) {
        move_to(account, MyResource { value: 42 });
    }
}
