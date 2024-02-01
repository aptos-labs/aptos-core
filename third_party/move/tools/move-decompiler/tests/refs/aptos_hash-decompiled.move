module 0x1::aptos_hash {
    public fun blake2b_256(arg0: vector<u8>) : vector<u8> {
        if (!0x1::features::blake2b_256_enabled()) {
            abort 0x1::error::invalid_state(1)
        };
        blake2b_256_internal(arg0)
    }
    
    native fun blake2b_256_internal(arg0: vector<u8>) : vector<u8>;
    native public fun keccak256(arg0: vector<u8>) : vector<u8>;
    public fun ripemd160(arg0: vector<u8>) : vector<u8> {
        if (!0x1::features::sha_512_and_ripemd_160_enabled()) {
            abort 0x1::error::invalid_state(1)
        };
        ripemd160_internal(arg0)
    }
    
    native fun ripemd160_internal(arg0: vector<u8>) : vector<u8>;
    public fun sha2_512(arg0: vector<u8>) : vector<u8> {
        if (!0x1::features::sha_512_and_ripemd_160_enabled()) {
            abort 0x1::error::invalid_state(1)
        };
        sha2_512_internal(arg0)
    }
    
    native fun sha2_512_internal(arg0: vector<u8>) : vector<u8>;
    public fun sha3_512(arg0: vector<u8>) : vector<u8> {
        if (!0x1::features::sha_512_and_ripemd_160_enabled()) {
            abort 0x1::error::invalid_state(1)
        };
        sha3_512_internal(arg0)
    }
    
    native fun sha3_512_internal(arg0: vector<u8>) : vector<u8>;
    native public fun sip_hash(arg0: vector<u8>) : u64;
    public fun sip_hash_from_value<T0>(arg0: &T0) : u64 {
        sip_hash(0x1::bcs::to_bytes<T0>(arg0))
    }
    
    // decompiled from Move bytecode v6
}
