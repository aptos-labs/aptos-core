module ExperimentalFramework::ExperimentalConsensusConfig {
    use Std::Capability;
    use CoreFramework::DiemConsensusConfig;

    struct ExperimentalConsensusConfig has drop {}

    public fun initialize(account: &signer) {
        DiemConsensusConfig::initialize<ExperimentalConsensusConfig>(account);
        Capability::create<ExperimentalConsensusConfig>(account, &ExperimentalConsensusConfig {});
    }

    public fun set(account: &signer, config: vector<u8>) {
        DiemConsensusConfig::set(
            config, &Capability::acquire(account, &ExperimentalConsensusConfig {})
        );
    }
}
