/// Maintains the consensus config for the blockchain. The config is stored in a
/// Reconfiguration, and may be updated by root.
module aptos_framework::consensus_config {
    use std::config_for_next_epoch;
    use std::error;
    use std::vector;
    use aptos_framework::system_addresses;

    friend aptos_framework::genesis;
    friend aptos_framework::reconfiguration;

    struct ConsensusConfig has drop, key, store {
        config: vector<u8>,
    }

    /// The provided on chain config bytes are empty or invalid
    const EINVALID_CONFIG: u64 = 1;

    /// Publishes the ConsensusConfig config.
    public(friend) fun initialize(aptos_framework: &signer, config: vector<u8>) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(vector::length(&config) > 0, error::invalid_argument(EINVALID_CONFIG));
        move_to(aptos_framework, ConsensusConfig { config });
    }

    /// This can be called by on-chain governance to update on-chain consensus configs.
    public fun set(account: &signer, config: vector<u8>) {
        system_addresses::assert_aptos_framework(account);
        assert!(vector::length(&config) > 0, error::invalid_argument(EINVALID_CONFIG));
        std::config_for_next_epoch::upsert<ConsensusConfig>(account, ConsensusConfig {config});
    }

    public(friend) fun on_new_epoch() acquires ConsensusConfig {
        if (config_for_next_epoch::does_exist<ConsensusConfig>()) {
            *borrow_global_mut<ConsensusConfig>(@aptos_framework) = std::config_for_next_epoch::extract();
        }
    }
}
