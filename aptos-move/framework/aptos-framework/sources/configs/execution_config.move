/// Maintains the execution config for the blockchain. The config is stored in a
/// Reconfiguration, and may be updated by root.
module aptos_framework::execution_config {
    use std::config_for_next_epoch;
    use std::error;
    use std::vector;
    use aptos_framework::reconfiguration;

    use aptos_framework::system_addresses;

    friend aptos_framework::genesis;
    friend aptos_framework::reconfiguration_v2;

    struct ExecutionConfig has drop, key, store {
        config: vector<u8>,
    }

    /// The provided on chain config bytes are empty or invalid
    const EINVALID_CONFIG: u64 = 1;

    /// This can be called by on-chain governance to update on-chain execution configs.
    public fun set(account: &signer, config: vector<u8>) acquires ExecutionConfig {
        system_addresses::assert_aptos_framework(account);
        assert!(vector::length(&config) > 0, error::invalid_argument(EINVALID_CONFIG));

        if (exists<ExecutionConfig>(@aptos_framework)) {
            let config_ref = &mut borrow_global_mut<ExecutionConfig>(@aptos_framework).config;
            *config_ref = config;
        } else {
            move_to(account, ExecutionConfig { config });
        };
        // Need to trigger reconfiguration so validator nodes can sync on the updated configs.
        reconfiguration::reconfigure();
    }

    /// This can be called by on-chain governance to update on-chain execution configs.
    public fun set_for_next_epoch(account: &signer, config: vector<u8>) {
        system_addresses::assert_aptos_framework(account);
        assert!(vector::length(&config) > 0, error::invalid_argument(EINVALID_CONFIG));
        config_for_next_epoch::upsert(account, ExecutionConfig { config });
    }

    /// Only used in reconfiguration with DKG.
    public(friend) fun on_new_epoch(account: &signer) acquires ExecutionConfig {
        if (config_for_next_epoch::does_exist<ExecutionConfig>()) {
            let config = config_for_next_epoch::extract<ExecutionConfig>(account);
            *borrow_global_mut<ExecutionConfig>(@aptos_framework) = config;
        }
    }
}
