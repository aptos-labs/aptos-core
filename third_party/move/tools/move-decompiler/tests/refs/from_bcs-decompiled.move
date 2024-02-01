module 0x1::from_bcs {
    native public(friend) fun from_bytes<T0>(arg0: vector<u8>) : T0;
    public fun to_address(arg0: vector<u8>) : address {
        from_bytes<address>(arg0)
    }
    
    public fun to_bool(arg0: vector<u8>) : bool {
        from_bytes<bool>(arg0)
    }
    
    public fun to_bytes(arg0: vector<u8>) : vector<u8> {
        from_bytes<vector<u8>>(arg0)
    }
    
    public fun to_string(arg0: vector<u8>) : 0x1::string::String {
        let v0 = from_bytes<0x1::string::String>(arg0);
        assert!(0x1::string::internal_check_utf8(0x1::string::bytes(&v0)), 1);
        v0
    }
    
    public fun to_u128(arg0: vector<u8>) : u128 {
        from_bytes<u128>(arg0)
    }
    
    public fun to_u16(arg0: vector<u8>) : u16 {
        from_bytes<u16>(arg0)
    }
    
    public fun to_u256(arg0: vector<u8>) : u256 {
        from_bytes<u256>(arg0)
    }
    
    public fun to_u32(arg0: vector<u8>) : u32 {
        from_bytes<u32>(arg0)
    }
    
    public fun to_u64(arg0: vector<u8>) : u64 {
        from_bytes<u64>(arg0)
    }
    
    public fun to_u8(arg0: vector<u8>) : u8 {
        from_bytes<u8>(arg0)
    }
    
    // decompiled from Move bytecode v6
}
