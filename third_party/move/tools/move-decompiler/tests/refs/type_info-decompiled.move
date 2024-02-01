module 0x1::type_info {
    struct TypeInfo has copy, drop, store {
        account_address: address,
        module_name: vector<u8>,
        struct_name: vector<u8>,
    }
    
    public fun account_address(arg0: &TypeInfo) : address {
        arg0.account_address
    }
    
    public fun chain_id() : u8 {
        if (!0x1::features::aptos_stdlib_chain_id_enabled()) {
            abort 0x1::error::invalid_state(1)
        };
        chain_id_internal()
    }
    
    native fun chain_id_internal() : u8;
    public fun module_name(arg0: &TypeInfo) : vector<u8> {
        arg0.module_name
    }
    
    public fun size_of_val<T0>(arg0: &T0) : u64 {
        let v0 = 0x1::bcs::to_bytes<T0>(arg0);
        0x1::vector::length<u8>(&v0)
    }
    
    public fun struct_name(arg0: &TypeInfo) : vector<u8> {
        arg0.struct_name
    }
    
    native public fun type_name<T0>() : 0x1::string::String;
    native public fun type_of<T0>() : TypeInfo;
    // decompiled from Move bytecode v6
}
