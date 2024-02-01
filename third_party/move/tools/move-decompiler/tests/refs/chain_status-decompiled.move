module 0x1::chain_status {
    struct GenesisEndMarker has key {
        dummy_field: bool,
    }
    
    public fun assert_genesis() {
        assert!(is_genesis(), 0x1::error::invalid_state(1));
    }
    
    public fun assert_operating() {
        assert!(is_operating(), 0x1::error::invalid_state(1));
    }
    
    public fun is_genesis() : bool {
        !exists<GenesisEndMarker>(@0x1)
    }
    
    public fun is_operating() : bool {
        exists<GenesisEndMarker>(@0x1)
    }
    
    public(friend) fun set_genesis_end(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = GenesisEndMarker{dummy_field: false};
        move_to<GenesisEndMarker>(arg0, v0);
    }
    
    // decompiled from Move bytecode v6
}
