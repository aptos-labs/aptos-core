module 0x1::version {
    struct SetVersionCapability has key {
        dummy_field: bool,
    }
    
    struct Version has key {
        major: u64,
    }
    
    public(friend) fun initialize(arg0: &signer, arg1: u64) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = Version{major: arg1};
        move_to<Version>(arg0, v0);
        let v1 = SetVersionCapability{dummy_field: false};
        move_to<SetVersionCapability>(arg0, v1);
    }
    
    fun initialize_for_test(arg0: &signer) {
        0x1::system_addresses::assert_core_resource(arg0);
        let v0 = SetVersionCapability{dummy_field: false};
        move_to<SetVersionCapability>(arg0, v0);
    }
    
    public entry fun set_version(arg0: &signer, arg1: u64) acquires Version {
        let v0 = exists<SetVersionCapability>(0x1::signer::address_of(arg0));
        assert!(v0, 0x1::error::permission_denied(2));
        assert!(borrow_global<Version>(@0x1).major < arg1, 0x1::error::invalid_argument(1));
        borrow_global_mut<Version>(@0x1).major = arg1;
        0x1::reconfiguration::reconfigure();
    }
    
    // decompiled from Move bytecode v6
}
