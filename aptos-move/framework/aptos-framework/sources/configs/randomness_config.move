/// Structs and functions for on-chain randomness configurations.
module aptos_framework::randomness_config {
    use std::string;
    use aptos_std::copyable_any;
    use aptos_std::copyable_any::Any;
    use aptos_framework::config_buffer;
    use aptos_framework::system_addresses;

    friend aptos_framework::reconfiguration_with_dkg;

    const EINVALID_CONFIG_VARIANT: u64 = 1;

    /// The configuration of the on-chain randomness feature.
    struct RandomnessConfig has drop, key, store {
        /// A config variant packed as an `Any`.
        /// Currently the variant type is one of the following.
        /// - `ConfigOn`
        /// - `ConfigOff`
        variant: Any,
    }

    /// A randomness config variant indicating the feature is disabled.
    struct ConfigOff has copy, drop, store {}

    /// A randomness config variant indicating the feature is enabled.
    struct ConfigV1 has copy, drop, store {}

    /// Initialize the configuration. Used in genesis or governance.
    public fun initialize(framework: &signer, config: RandomnessConfig) {
        system_addresses::assert_aptos_framework(framework);
        move_to(framework, config)
    }

    /// This can be called by on-chain governance to update on-chain consensus configs for the next epoch.
    public fun set_for_next_epoch(framework: &signer, new_config: RandomnessConfig) {
        system_addresses::assert_aptos_framework(framework);
        config_buffer::upsert(new_config);
    }

    public(friend) fun on_new_epoch() acquires RandomnessConfig {
        if (config_buffer::does_exist<RandomnessConfig>()) {
            let new_config = config_buffer::extract<RandomnessConfig>();
            borrow_global_mut<RandomnessConfig>(@aptos_framework).variant = new_config.variant;
        }
    }

    /// Check whether on-chain randomness main logic (e.g., `DKGManager`, `RandManager`, `BlockMetadataExt`) is enabled.
    ///
    /// NOTE: the main logic is not the only dependency.
    /// On-chain randomness works if and only if `consensus_config::validator_txn_enabled() && randomness_config::enabled()`.
    public fun enabled(): bool acquires RandomnessConfig {
        if (exists<RandomnessConfig>(@aptos_framework)) {
            let config = borrow_global<RandomnessConfig>(@aptos_framework);
            let variant_type_name = *string::bytes(copyable_any::type_name(&config.variant));
            variant_type_name != b"0x1::randomness_config::ConfigOff"
        } else {
            false
        }
    }

    /// Create a `ConfigOff` variant.
    public fun new_off(): RandomnessConfig {
        RandomnessConfig {
            variant: copyable_any::pack( ConfigOff {} )
        }
    }

    /// Create a `ConfigV1` variant.
    public fun new_v1(): RandomnessConfig {
        RandomnessConfig {
            variant: copyable_any::pack( ConfigV1 {} )
        }
    }

    #[test_only]
    fun initialize_for_testing(framework: &signer) {
        config_buffer::initialize(framework);
        initialize(framework, new_off());
    }

    #[test(framework = @0x1)]
    fun basic(framework: signer) acquires RandomnessConfig {
        initialize_for_testing(&framework);

        // Enabling.
        let config_1 = new_v1();
        set_for_next_epoch(&framework, config_1);
        on_new_epoch();
        assert!(enabled(), 1);

        // Disabling.
        let config_2 = new_off();
        set_for_next_epoch(&framework, config_2);
        on_new_epoch();
        assert!(!enabled(), 2);
    }
}
