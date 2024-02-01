module 0x1::gas_schedule {
    struct GasEntry has copy, drop, store {
        key: 0x1::string::String,
        val: u64,
    }
    
    struct GasSchedule has copy, drop, key {
        entries: vector<GasEntry>,
    }
    
    struct GasScheduleV2 has copy, drop, key {
        feature_version: u64,
        entries: vector<GasEntry>,
    }
    
    public(friend) fun initialize(arg0: &signer, arg1: vector<u8>) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        assert!(!0x1::vector::is_empty<u8>(&arg1), 0x1::error::invalid_argument(1));
        move_to<GasScheduleV2>(arg0, 0x1::util::from_bytes<GasScheduleV2>(arg1));
    }
    
    public fun set_gas_schedule(arg0: &signer, arg1: vector<u8>) acquires GasSchedule, GasScheduleV2 {
        0x1::system_addresses::assert_aptos_framework(arg0);
        assert!(!0x1::vector::is_empty<u8>(&arg1), 0x1::error::invalid_argument(1));
        if (exists<GasScheduleV2>(@0x1)) {
            let v0 = borrow_global_mut<GasScheduleV2>(@0x1);
            let v1 = 0x1::util::from_bytes<GasScheduleV2>(arg1);
            assert!(v1.feature_version >= v0.feature_version, 0x1::error::invalid_argument(2));
            *v0 = v1;
        } else {
            if (exists<GasSchedule>(@0x1)) {
                move_from<GasSchedule>(@0x1);
            };
            move_to<GasScheduleV2>(arg0, 0x1::util::from_bytes<GasScheduleV2>(arg1));
        };
        0x1::reconfiguration::reconfigure();
    }
    
    public fun set_storage_gas_config(arg0: &signer, arg1: 0x1::storage_gas::StorageGasConfig) {
        0x1::storage_gas::set_config(arg0, arg1);
        0x1::reconfiguration::reconfigure();
    }
    
    // decompiled from Move bytecode v6
}
