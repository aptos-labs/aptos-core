module 0x1::consensus_config {
    struct ConsensusConfig has key {
        config: vector<u8>,
    }
    
    public(friend) fun initialize(arg0: &signer, arg1: vector<u8>) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        assert!(0x1::vector::length<u8>(&arg1) > 0, 0x1::error::invalid_argument(1));
        let v0 = ConsensusConfig{config: arg1};
        move_to<ConsensusConfig>(arg0, v0);
    }
    
    public fun set(arg0: &signer, arg1: vector<u8>) acquires ConsensusConfig {
        0x1::system_addresses::assert_aptos_framework(arg0);
        assert!(0x1::vector::length<u8>(&arg1) > 0, 0x1::error::invalid_argument(1));
        borrow_global_mut<ConsensusConfig>(@0x1).config = arg1;
        0x1::reconfiguration::reconfigure();
    }
    
    // decompiled from Move bytecode v6
}
