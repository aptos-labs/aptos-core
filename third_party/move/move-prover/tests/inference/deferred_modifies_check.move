module 0x42::deferred_modifies_check {
    struct R has key {
        value: u64,
    }

    public fun initialize(account: &signer) {
        move_to(account, R { value: 0 });
    }

    public fun write_value(addr: address, value: u64) acquires R {
        borrow_global_mut<R>(addr).value = value;
    }

    public fun update_from_caller(addr: address, value: u64) acquires R {
        write_value(addr, value);
    }

    spec update_from_caller {
        modifies global<R>(addr);
    }
}
