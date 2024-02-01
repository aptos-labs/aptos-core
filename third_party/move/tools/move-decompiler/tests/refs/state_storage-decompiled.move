module 0x1::state_storage {
    struct GasParameter has store, key {
        usage: Usage,
    }
    
    struct StateStorageUsage has store, key {
        epoch: u64,
        usage: Usage,
    }
    
    struct Usage has copy, drop, store {
        items: u64,
        bytes: u64,
    }
    
    public(friend) fun current_items_and_bytes() : (u64, u64) acquires StateStorageUsage {
        assert!(exists<StateStorageUsage>(@0x1), 0x1::error::not_found(0));
        let v0 = borrow_global<StateStorageUsage>(@0x1);
        (v0.usage.items, v0.usage.bytes)
    }
    
    native fun get_state_storage_usage_only_at_epoch_beginning() : Usage;
    public(friend) fun initialize(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        assert!(!exists<StateStorageUsage>(@0x1), 0x1::error::already_exists(0));
        let v0 = Usage{
            items : 0, 
            bytes : 0,
        };
        let v1 = StateStorageUsage{
            epoch : 0, 
            usage : v0,
        };
        move_to<StateStorageUsage>(arg0, v1);
    }
    
    public(friend) fun on_new_block(arg0: u64) acquires StateStorageUsage {
        assert!(exists<StateStorageUsage>(@0x1), 0x1::error::not_found(0));
        let v0 = borrow_global_mut<StateStorageUsage>(@0x1);
        if (arg0 != v0.epoch) {
            v0.epoch = arg0;
            v0.usage = get_state_storage_usage_only_at_epoch_beginning();
        };
    }
    
    public(friend) fun on_reconfig() {
        abort 0
    }
    
    // decompiled from Move bytecode v6
}
