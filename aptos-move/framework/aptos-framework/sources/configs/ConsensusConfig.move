/// Maintains the consensus config for the blockchain. The config is stored in a
/// Reconfiguration, and may be updated by root.
module AptosFramework::ConsensusConfig {
    use Std::Errors;
    use Std::Vector;
    use AptosFramework::Reconfiguration;
    use AptosFramework::Timestamp;
    use AptosFramework::SystemAddresses;

    /// Error with config
    const ECONFIG: u64 = 0;

    struct ConsensusConfig has key {
        config: vector<u8>,
    }

    /// Publishes the ConsensusConfig config.
    public fun initialize(account: &signer) {
        Timestamp::assert_genesis();
        SystemAddresses::assert_core_resource(account);

        assert!(
            !exists<ConsensusConfig>(@CoreResources),
            Errors::already_published(ECONFIG)
        );
        move_to(account, ConsensusConfig { config: Vector::empty() });
    }

    /// Update the config.
    public fun set(account: &signer, config: vector<u8>) acquires ConsensusConfig {
        SystemAddresses::assert_core_resource(account);
        let config_ref = &mut borrow_global_mut<ConsensusConfig>(@CoreResources).config;
        *config_ref = config;
        Reconfiguration::reconfigure();
    }
}
