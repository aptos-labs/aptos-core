/// Maintains the execution config for the blockchain. The config is stored in a
/// Reconfiguration, and may be updated by root.
module supra_framework::execution_config {
    use std::error;
    use std::vector;

    use supra_framework::reconfiguration;
    use supra_framework::system_addresses;

    friend supra_framework::genesis;

    struct ExecutionConfig has key {
        config: vector<u8>,
    }

    /// The provided on chain config bytes are empty or invalid
    const EINVALID_CONFIG: u64 = 1;

    /// This can be called by on-chain governance to update on-chain execution configs.
    public fun set(account: &signer, config: vector<u8>) acquires ExecutionConfig {
        system_addresses::assert_supra_framework(account);
        assert!(vector::length(&config) > 0, error::invalid_argument(EINVALID_CONFIG));

        if (exists<ExecutionConfig>(@supra_framework)) {
            let config_ref = &mut borrow_global_mut<ExecutionConfig>(@supra_framework).config;
            *config_ref = config;
        } else {
            move_to(account, ExecutionConfig { config });
        };
        // Need to trigger reconfiguration so validator nodes can sync on the updated configs.
        reconfiguration::reconfigure();
    }
}
