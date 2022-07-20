/// Maintains the consensus config for the blockchain. The config is stored in a
/// Reconfiguration, and may be updated by root.
module aptos_framework::consensus_config {
    use std::error;
    use std::vector;
    use aptos_framework::reconfiguration;
    use aptos_framework::timestamp;
    use aptos_framework::system_addresses;

    /// Error with config
    const ECONFIG: u64 = 0;

    struct ConsensusConfig has key {
        config: vector<u8>,
    }

    /// Publishes the ConsensusConfig config.
    public fun initialize(account: &signer) {
        timestamp::assert_genesis();
        system_addresses::assert_aptos_framework(account);

        assert!(
            !exists<ConsensusConfig>(@aptos_framework),
            error::already_exists(ECONFIG)
        );
        move_to(account, ConsensusConfig { config: vector::empty() });
    }

    /// Update the config.
    public fun set(account: &signer, config: vector<u8>) acquires ConsensusConfig {
        system_addresses::assert_aptos_framework(account);
        let config_ref = &mut borrow_global_mut<ConsensusConfig>(@aptos_framework).config;
        *config_ref = config;
        reconfiguration::reconfigure();
    }
}
