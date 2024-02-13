/// Maintains the consensus config for the blockchain. The config is stored in a
/// Reconfiguration, and may be updated by root.
module aptos_framework::consensus_config {
    use std::error;
    use std::vector;
    use aptos_framework::config_buffer;

    use aptos_framework::reconfiguration;
    use aptos_framework::system_addresses;

    friend aptos_framework::genesis;
    friend aptos_framework::reconfiguration_with_dkg;

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
    public fun set(account: &signer, config: vector<u8>) acquires ConsensusConfig {
        system_addresses::assert_aptos_framework(account);
        assert!(vector::length(&config) > 0, error::invalid_argument(EINVALID_CONFIG));

        let config_ref = &mut borrow_global_mut<ConsensusConfig>(@aptos_framework).config;
        *config_ref = config;

        // Need to trigger reconfiguration so validator nodes can sync on the updated configs.
        reconfiguration::reconfigure();
    }

    /// This can be called by on-chain governance to update on-chain consensus configs for the next epoch.
    ///
    /// NOTE: when it takes effects depend on feature `RECONFIGURE_WITH_DKG`.
    /// See `aptos_framework::aptos_governance::reconfigure()` for more details.
    public fun set_for_next_epoch(account: &signer, config: vector<u8>) {
        system_addresses::assert_aptos_framework(account);
        assert!(vector::length(&config) > 0, error::invalid_argument(EINVALID_CONFIG));
        std::config_buffer::upsert<ConsensusConfig>(ConsensusConfig {config});
    }

    public(friend) fun on_new_epoch() acquires ConsensusConfig {
        if (config_buffer::does_exist<ConsensusConfig>()) {
            *borrow_global_mut<ConsensusConfig>(@aptos_framework) = config_buffer::extract();
        }
    }

    public fun validator_txn_enabled(): bool acquires ConsensusConfig {
        let config_bytes = borrow_global<ConsensusConfig>(@aptos_framework).config;
        validator_txn_enabled_internal(config_bytes)
    }

    native fun validator_txn_enabled_internal(config_bytes: vector<u8>): bool;
}
