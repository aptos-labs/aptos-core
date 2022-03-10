/// Maintains the consensus config for the blockchain. The config is stored in a
/// Reconfiguration, and may be updated by root.
module CoreFramework::ConsensusConfig {
    use Std::Capability::Cap;
    use Std::Errors;
    use Std::Vector;
    use CoreFramework::Reconfiguration;
    use CoreFramework::Timestamp;
    use CoreFramework::SystemAddresses;

    /// Error with chain marker
    const ECHAIN_MARKER: u64 = 0;
    /// Error with config
    const ECONFIG: u64 = 1;

    /// Marker to be stored under @CoreResources during genesis
    struct ConsensusConfigChainMarker<phantom T> has key {}

    struct ConsensusConfig has key {
        config: vector<u8>,
    }

    /// Publishes the ConsensusConfig config.
    public fun initialize<T>(account: &signer) {
        Timestamp::assert_genesis();
        SystemAddresses::assert_core_resource(account);
        assert!(
            !exists<ConsensusConfigChainMarker<T>>(@CoreResources),
            Errors::already_published(ECHAIN_MARKER)
        );

        assert!(
            !exists<ConsensusConfig>(@CoreResources),
            Errors::already_published(ECONFIG)
        );
        move_to(account, ConsensusConfigChainMarker<T>{});
        move_to(account, ConsensusConfig { config: Vector::empty() });
    }

    /// Update the config.
    public fun set<T>(config: vector<u8>, _cap: &Cap<T>) acquires ConsensusConfig {
        assert!(exists<ConsensusConfigChainMarker<T>>(@CoreResources), Errors::not_published(ECHAIN_MARKER));
        let config_ref = &mut borrow_global_mut<ConsensusConfig>(@CoreResources).config;
        *config_ref = config;
        Reconfiguration::reconfigure();
    }
}
