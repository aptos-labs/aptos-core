/// Maintains the hot state config for the blockchain.
///
/// The config is opaque to Move and interpreted by node software, so fields can be added without
/// a framework upgrade. Since it affects the hot state root, it only changes at epoch boundaries.
module aptos_framework::hot_state_config {
    use std::error;
    use aptos_framework::config_buffer;
    use aptos_framework::system_addresses;

    friend aptos_framework::genesis;
    friend aptos_framework::reconfiguration_with_dkg;

    struct HotStateConfig has drop, key, store {
        config: vector<u8>,
    }

    /// The provided on chain config bytes are empty or invalid
    const EINVALID_CONFIG: u64 = 1;

    /// Publishes the initial config. Only used in genesis.
    public(friend) fun initialize(framework: &signer, config: vector<u8>) {
        system_addresses::assert_aptos_framework(framework);
        assert!(config.length() > 0, error::invalid_argument(EINVALID_CONFIG));
        if (!exists<HotStateConfig>(@aptos_framework)) {
            move_to(framework, HotStateConfig { config });
        }
    }

    /// This can be called by on-chain governance to update the hot state config for the next epoch.
    /// Example usage:
    /// ```
    /// aptos_framework::hot_state_config::set_for_next_epoch(&framework_signer, some_config_bytes);
    /// aptos_framework::aptos_governance::reconfigure(&framework_signer);
    /// ```
    public fun set_for_next_epoch(framework: &signer, config: vector<u8>) {
        system_addresses::assert_aptos_framework(framework);
        assert!(config.length() > 0, error::invalid_argument(EINVALID_CONFIG));
        config_buffer::upsert(HotStateConfig { config });
    }

    /// Only used in reconfigurations to apply the pending `HotStateConfig`, if there is any.
    public(friend) fun on_new_epoch(framework: &signer) acquires HotStateConfig {
        system_addresses::assert_aptos_framework(framework);
        if (config_buffer::does_exist<HotStateConfig>()) {
            let config = config_buffer::extract_v2<HotStateConfig>();
            if (exists<HotStateConfig>(@aptos_framework)) {
                *borrow_global_mut<HotStateConfig>(@aptos_framework) = config;
            } else {
                move_to(framework, config);
            };
        }
    }

    #[test_only]
    fun config_for_testing(): vector<u8> acquires HotStateConfig {
        borrow_global<HotStateConfig>(@aptos_framework).config
    }

    #[test(framework = @aptos_framework)]
    fun test_initialize_and_apply(framework: signer) acquires HotStateConfig {
        config_buffer::initialize(&framework);

        initialize(&framework, vector[1]);
        assert!(config_for_testing() == vector[1], 1);
        // Initializing again is a no-op.
        initialize(&framework, vector[2]);
        assert!(config_for_testing() == vector[1], 2);

        set_for_next_epoch(&framework, vector[3]);
        assert!(config_for_testing() == vector[1], 3);
        on_new_epoch(&framework);
        assert!(config_for_testing() == vector[3], 4);

        // No pending change.
        on_new_epoch(&framework);
        assert!(config_for_testing() == vector[3], 5);
    }

    #[test(framework = @aptos_framework)]
    fun test_apply_without_initialize(framework: signer) acquires HotStateConfig {
        config_buffer::initialize(&framework);
        assert!(!exists<HotStateConfig>(@aptos_framework), 1);

        set_for_next_epoch(&framework, vector[1]);
        on_new_epoch(&framework);
        assert!(config_for_testing() == vector[1], 2);
    }

    #[test(framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10001, location = Self)]
    fun test_empty_config_aborts(framework: signer) {
        config_buffer::initialize(&framework);
        set_for_next_epoch(&framework, vector[]);
    }

    #[test(framework = @aptos_framework, attacker = @0xa11ce)]
    #[expected_failure(abort_code = 0x50003, location = aptos_framework::system_addresses)]
    fun test_non_framework_signer_cannot_set(framework: signer, attacker: signer) {
        config_buffer::initialize(&framework);
        set_for_next_epoch(&attacker, vector[1]);
    }
}
