module ExperimentalFramework::ExperimentalConsensusConfig {
    use Std::Capability;
    use CoreFramework::ConsensusConfig;

    struct ExperimentalConsensusConfig has drop {}

    public fun initialize(account: &signer) {
        ConsensusConfig::initialize<ExperimentalConsensusConfig>(account);
        Capability::create<ExperimentalConsensusConfig>(account, &ExperimentalConsensusConfig {});
    }

    public fun set(account: &signer, config: vector<u8>) {
        ConsensusConfig::set(
            config, &Capability::acquire(account, &ExperimentalConsensusConfig {})
        );
    }
}
