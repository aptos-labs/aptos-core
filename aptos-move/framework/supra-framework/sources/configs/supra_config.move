/// Maintains protocol configuation settings specific to Supra. The config is stored in a
/// Reconfiguration, and may be updated by root.
module supra_framework::supra_config {
    use std::error;
    use std::vector;
    use supra_framework::config_buffer;
    use supra_framework::system_addresses;

    friend supra_framework::genesis;
    friend supra_framework::reconfiguration_with_dkg;

    struct SupraConfig has drop, key, store {
        config: vector<u8>,
    }

    /// The provided on chain config bytes are empty or invalid
    const EINVALID_CONFIG: u64 = 1;

    /// Publishes the SupraConfig config.
    public(friend) fun initialize(supra_framework: &signer, config: vector<u8>) {
        system_addresses::assert_supra_framework(supra_framework);
        assert!(vector::length(&config) > 0, error::invalid_argument(EINVALID_CONFIG));
        move_to(supra_framework, SupraConfig { config });
    }

    /// This can be called by on-chain governance to update on-chain configs for the next epoch.
    /// Example usage:
    /// ```
    /// supra_framework::supra_config::set_for_next_epoch(&framework_signer, some_config_bytes);
    /// supra_framework::supra_governance::reconfigure(&framework_signer);
    /// ```
    public fun set_for_next_epoch(account: &signer, config: vector<u8>) {
        system_addresses::assert_supra_framework(account);
        assert!(vector::length(&config) > 0, error::invalid_argument(EINVALID_CONFIG));
        std::config_buffer::upsert<SupraConfig>(SupraConfig {config});
    }

    /// Only used in reconfigurations to apply the pending `SupraConfig`, if there is any.
    public(friend) fun on_new_epoch(framework: &signer) acquires SupraConfig {
        system_addresses::assert_supra_framework(framework);
        if (config_buffer::does_exist<SupraConfig>()) {
            let new_config = config_buffer::extract<SupraConfig>();
            if (exists<SupraConfig>(@supra_framework)) {
                *borrow_global_mut<SupraConfig>(@supra_framework) = new_config;
            } else {
                move_to(framework, new_config);
            };
        }
    }
}
