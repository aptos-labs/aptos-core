module 0x1::timestamp {
    struct CurrentTimeMicroseconds has key {
        microseconds: u64,
    }
    
    public fun now_microseconds() : u64 acquires CurrentTimeMicroseconds {
        borrow_global<CurrentTimeMicroseconds>(@0x1).microseconds
    }
    
    public fun now_seconds() : u64 acquires CurrentTimeMicroseconds {
        let v0 = now_microseconds();
        v0 / 1000000
    }
    
    public(friend) fun set_time_has_started(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = CurrentTimeMicroseconds{microseconds: 0};
        move_to<CurrentTimeMicroseconds>(arg0, v0);
    }
    
    public fun update_global_time(arg0: &signer, arg1: address, arg2: u64) acquires CurrentTimeMicroseconds {
        0x1::system_addresses::assert_vm(arg0);
        let v0 = borrow_global_mut<CurrentTimeMicroseconds>(@0x1);
        if (arg1 == @0x3001) {
            assert!(v0.microseconds == arg2, 0x1::error::invalid_argument(2));
        } else {
            assert!(v0.microseconds < arg2, 0x1::error::invalid_argument(2));
            v0.microseconds = arg2;
        };
    }
    
    // decompiled from Move bytecode v6
}
