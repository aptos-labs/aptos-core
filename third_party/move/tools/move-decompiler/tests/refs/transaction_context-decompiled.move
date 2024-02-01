module 0x1::transaction_context {
    struct AUID has drop, store {
        unique_address: address,
    }
    
    public fun auid_address(arg0: &AUID) : address {
        arg0.unique_address
    }
    
    public fun generate_auid() : AUID {
        AUID{unique_address: generate_unique_address()}
    }
    
    public fun generate_auid_address() : address {
        generate_unique_address()
    }
    
    native fun generate_unique_address() : address;
    native public fun get_script_hash() : vector<u8>;
    public fun get_transaction_hash() : vector<u8> {
        get_txn_hash()
    }
    
    native fun get_txn_hash() : vector<u8>;
    // decompiled from Move bytecode v6
}
