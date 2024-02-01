module 0x1::aggregator {
    struct Aggregator has store {
        handle: address,
        key: address,
        limit: u128,
    }
    
    native public fun add(arg0: &mut Aggregator, arg1: u128);
    native public fun destroy(arg0: Aggregator);
    public fun limit(arg0: &Aggregator) : u128 {
        arg0.limit
    }
    
    native public fun read(arg0: &Aggregator) : u128;
    native public fun sub(arg0: &mut Aggregator, arg1: u128);
    // decompiled from Move bytecode v6
}
