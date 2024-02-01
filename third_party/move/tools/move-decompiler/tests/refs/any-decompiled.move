module 0x1::any {
    struct Any has drop, store {
        type_name: 0x1::string::String,
        data: vector<u8>,
    }
    
    public fun type_name(arg0: &Any) : &0x1::string::String {
        &arg0.type_name
    }
    
    public fun pack<T0: drop + store>(arg0: T0) : Any {
        Any{
            type_name : 0x1::type_info::type_name<T0>(), 
            data      : 0x1::bcs::to_bytes<T0>(&arg0),
        }
    }
    
    public fun unpack<T0>(arg0: Any) : T0 {
        assert!(0x1::type_info::type_name<T0>() == arg0.type_name, 0x1::error::invalid_argument(1));
        0x1::from_bcs::from_bytes<T0>(arg0.data)
    }
    
    // decompiled from Move bytecode v6
}
