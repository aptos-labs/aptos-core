module supra_framework::evm_genesis_config {
    use std::error;
    use std::vector;
    use supra_framework::config_buffer;
    use supra_framework::event;

    use supra_framework::system_addresses;

    friend supra_framework::genesis;
    friend supra_framework::reconfiguration_with_dkg;

    /// The struct stores the on-chain EVM genesis configuration.
    // Note: Serialized bytes of `OnChainEvmGenesisConfig` in rust layer.
    struct EvmGenesisConfig has drop, key, store {
        config: vector<u8>
    }

    #[event]
    /// Event to signal EVM genesis config has been initialized or updated.
    struct EvmGenesisEvent has drop, store {}

    /// The provided on chain config bytes are empty or invalid
    const EINVALID_CONFIG: u64 = 1;

    /// Publishes the EvmGenesisConfig config.
    public(friend) fun initialize(
        supra_framework: &signer, config: vector<u8>
    ) {
        system_addresses::assert_supra_framework(supra_framework);
        assert!(!vector::is_empty(&config), error::invalid_argument(EINVALID_CONFIG));
        move_to(supra_framework, EvmGenesisConfig { config });
        event::emit(EvmGenesisEvent {});
    }

    /// This can be called by on-chain governance to update on-chain evm configs for the next epoch.
    /// Example usage:
    /// ```
    /// supra_framework::evm_genesis_config::set_for_next_epoch(&framework_signer, some_config_bytes);
    /// supra_framework::supra_governance::reconfigure(&framework_signer);
    /// ```
    public fun set_for_next_epoch(account: &signer, config: vector<u8>) {
        system_addresses::assert_supra_framework(account);
        assert!(!vector::is_empty(&config), error::invalid_argument(EINVALID_CONFIG));
        std::config_buffer::upsert<EvmGenesisConfig>(EvmGenesisConfig { config });
    }

    /// Only used in reconfigurations to apply the pending `EvmGenesisConfig` in buffer, if there is any.
    /// If supra_framework has a EvmGenesisConfig, then update the new config to supra_framework.
    /// Otherwise, move the new config to supra_framework.
    public(friend) fun on_new_epoch(framework: &signer) {
        system_addresses::assert_supra_framework(framework);
        if (config_buffer::does_exist<EvmGenesisConfig>()) {
            let new_config = config_buffer::extract<EvmGenesisConfig>();
            if (!exists<EvmGenesisConfig>(@supra_framework)) {
                move_to(framework, new_config);
                event::emit(EvmGenesisEvent {});
            };
        }
    }
}
