module 0x1::chain_id {
    struct ChainId has key {
        id: u8,
    }
    
    public fun get() : u8 acquires ChainId {
        borrow_global<ChainId>(@0x1).id
    }
    
    public(friend) fun initialize(arg0: &signer, arg1: u8) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = ChainId{id: arg1};
        move_to<ChainId>(arg0, v0);
    }
    
    // decompiled from Move bytecode v6
}
