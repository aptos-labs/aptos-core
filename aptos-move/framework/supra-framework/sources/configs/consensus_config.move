/// Maintains the consensus config for the blockchain. The config is stored in a
/// Reconfiguration, and may be updated by root.
module supra_framework::consensus_config {
    use std::error;
    use std::vector;

    use supra_framework::reconfiguration;
    use supra_framework::system_addresses;

    friend supra_framework::genesis;

    struct ConsensusConfig has key {
        config: vector<u8>,
    }

    /// The provided on chain config bytes are empty or invalid
    const EINVALID_CONFIG: u64 = 1;

    /// Publishes the ConsensusConfig config.
    public(friend) fun initialize(supra_framework: &signer, config: vector<u8>) {
        system_addresses::assert_supra_framework(supra_framework);
        assert!(vector::length(&config) > 0, error::invalid_argument(EINVALID_CONFIG));
        move_to(supra_framework, ConsensusConfig { config });
    }

    /// This can be called by on-chain governance to update on-chain consensus configs.
    public fun set(account: &signer, config: vector<u8>) acquires ConsensusConfig {
        system_addresses::assert_supra_framework(account);
        assert!(vector::length(&config) > 0, error::invalid_argument(EINVALID_CONFIG));

        let config_ref = &mut borrow_global_mut<ConsensusConfig>(@supra_framework).config;
        *config_ref = config;

        // Need to trigger reconfiguration so validator nodes can sync on the updated configs.
        reconfiguration::reconfigure();
    }
}
