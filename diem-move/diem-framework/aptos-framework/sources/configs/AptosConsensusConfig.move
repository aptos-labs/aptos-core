module AptosFramework::AptosConsensusConfig {
    use Std::Capability;
    use CoreFramework::DiemConsensusConfig;
    use AptosFramework::Marker;

    public fun initialize(account: &signer) {
        DiemConsensusConfig::initialize<Marker::ChainMarker>(account);
    }

    public fun set(account: &signer, config: vector<u8>) {
        DiemConsensusConfig::set(
            config, &Capability::acquire(account, &Marker::get())
        );
    }
}
