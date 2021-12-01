/// Maintains the consensus config for the Diem blockchain. The config is stored in a
/// DiemConfig, and may be updated by Diem root.
module ExperimentalFramework::DiemConsensusConfig {
    use CoreFramework::SystemAddresses;
    use ExperimentalFramework::DiemConfig;
    use Std::Vector;

    struct DiemConsensusConfig has copy, drop, store {
        config: vector<u8>,
    }

    /// Publishes the DiemConsensusConfig config.
    public fun initialize(dr_account: &signer) {
        SystemAddresses::assert_core_resource(dr_account);
        DiemConfig::publish_new_config(dr_account, DiemConsensusConfig { config: Vector::empty() });
    }

    /// Allows Diem root to update the config.
    public fun set(dr_account: &signer, config: vector<u8>) {
        SystemAddresses::assert_core_resource(dr_account);

        DiemConfig::set(
            dr_account,
            DiemConsensusConfig { config }
        );
    }
}
