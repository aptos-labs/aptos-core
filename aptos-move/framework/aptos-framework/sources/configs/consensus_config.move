/// Maintains the consensus config for the blockchain. The config is stored in a
/// Reconfiguration, and may be updated by root.
module aptos_framework::consensus_config {
    use aptos_framework::reconfiguration;
    use aptos_framework::timestamp;
    use aptos_framework::system_addresses;

    friend aptos_framework::genesis;

    struct ConsensusConfig has key {
        config: vector<u8>,
    }

    /// Publishes the ConsensusConfig config.
    public(friend) fun initialize(account: &signer, config: vector<u8>) {
        timestamp::assert_genesis();
        system_addresses::assert_aptos_framework(account);

        move_to(account, ConsensusConfig { config });
    }

    /// Update the config.
    public fun set(account: &signer, config: vector<u8>) acquires ConsensusConfig {
        system_addresses::assert_aptos_framework(account);
        let config_ref = &mut borrow_global_mut<ConsensusConfig>(@aptos_framework).config;
        *config_ref = config;
        reconfiguration::reconfigure();
    }
}
