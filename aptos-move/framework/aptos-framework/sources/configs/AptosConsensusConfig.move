module AptosFramework::AptosConsensusConfig {
    use Std::Capability;
    use AptosFramework::ConsensusConfig;
    use AptosFramework::Marker;

    public fun initialize(account: &signer) {
        ConsensusConfig::initialize<Marker::ChainMarker>(account);
    }

    public fun set(account: &signer, config: vector<u8>) {
        ConsensusConfig::set(
            config, &Capability::acquire(account, &Marker::get())
        );
    }
}
