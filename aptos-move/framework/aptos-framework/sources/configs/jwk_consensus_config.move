module aptos_framework::jwk_consensus_config {
    use aptos_framework::config_buffer;
    use aptos_framework::system_addresses;

    friend aptos_framework::reconfiguration_with_dkg;

    struct JWKConsensusConfig has drop, key, store {
        bytes: vector<u8>,
    }

    public fun initialize(framework: &signer, bytes: vector<u8>) {
        move_to(framework, JWKConsensusConfig { bytes })
    }

    public fun set_for_next_epoch(framework: &signer, bytes: vector<u8>) {
        system_addresses::assert_aptos_framework(framework);
        let flag = JWKConsensusConfig { bytes };
        config_buffer::upsert(flag);

    }

    public(friend) fun on_new_epoch() acquires JWKConsensusConfig {
        if (config_buffer::does_exist<JWKConsensusConfig>()) {
            let new_config = config_buffer::extract<JWKConsensusConfig>();
            borrow_global_mut<JWKConsensusConfig>(@aptos_framework).bytes = new_config.bytes;
        }
    }
}
