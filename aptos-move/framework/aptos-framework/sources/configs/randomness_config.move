module aptos_framework::randomness_config {
    use aptos_framework::config_buffer;
    use aptos_framework::system_addresses;

    friend aptos_framework::reconfiguration_with_dkg;

    struct RandomnessConfig has drop, key, store {
        bytes: vector<u8>,
    }

    public fun initialize(framework: &signer, bytes: vector<u8>) {
        move_to(framework, RandomnessConfig { bytes })
    }

    public fun set_for_next_epoch(framework: &signer, bytes: vector<u8>) {
        system_addresses::assert_aptos_framework(framework);
        let flag = RandomnessConfig { bytes };
        config_buffer::upsert(flag);
    }

    public(friend) fun on_new_epoch() acquires RandomnessConfig {
        if (config_buffer::does_exist<RandomnessConfig>()) {
            let new_config = config_buffer::extract<RandomnessConfig>();
            borrow_global_mut<RandomnessConfig>(@aptos_framework).bytes = new_config.bytes;
        }
    }

    public fun enabled(): bool acquires RandomnessConfig {
        if (exists<RandomnessConfig>(@aptos_framework)) {
            let config_bytes = borrow_global<RandomnessConfig>(@aptos_framework).bytes;
            enabled_internal(config_bytes)
        } else {
            false
        }
    }

    native fun enabled_internal(config_bytes: vector<u8>): bool;
}
