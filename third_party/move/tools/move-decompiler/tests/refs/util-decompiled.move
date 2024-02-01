module 0x1::util {
    public fun address_from_bytes(arg0: vector<u8>) : address {
        from_bytes<address>(arg0)
    }
    
    native public(friend) fun from_bytes<T0>(arg0: vector<u8>) : T0;
    // decompiled from Move bytecode v6
}
