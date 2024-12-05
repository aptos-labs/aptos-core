/// Structs and functions for on-chain randomness configurations.
module aptos_framework::randomness_config {
    use std::string;
    use std::string::String;
    use aptos_std::copyable_any;
    use aptos_std::copyable_any::Any;
    use aptos_std::fixed_point64;
    use aptos_std::fixed_point64::FixedPoint64;
    use aptos_framework::config_buffer;
    use aptos_framework::system_addresses;

    friend aptos_framework::reconfiguration_with_dkg;

    const EINVALID_CONFIG_VARIANT: u64 = 1;

    /// The configuration of the on-chain randomness feature.
    struct RandomnessConfig has copy, drop, key, store {
        /// A config variant packed as an `Any`.
        /// Currently the variant type is one of the following.
        /// - `ConfigOff`
        /// - `ConfigV1`
        variant: Any,
    }

    /// A randomness config variant indicating the feature is disabled.
    struct ConfigOff has copy, drop, store {}

    /// A randomness config variant indicating the feature is enabled.
    struct ConfigV1 has copy, drop, store {
        /// Any validator subset should not be able to reconstruct randomness if `subset_power / total_power <= secrecy_threshold`,
        secrecy_threshold: FixedPoint64,
        /// Any validator subset should be able to reconstruct randomness if `subset_power / total_power > reconstruction_threshold`.
        reconstruction_threshold: FixedPoint64,
    }

    /// A randomness config variant indicating the feature is enabled with fast path.
    struct ConfigV2 has copy, drop, store {
        /// Any validator subset should not be able to reconstruct randomness if `subset_power / total_power <= secrecy_threshold`,
        secrecy_threshold: FixedPoint64,
        /// Any validator subset should be able to reconstruct randomness if `subset_power / total_power > reconstruction_threshold`.
        reconstruction_threshold: FixedPoint64,
        /// Any validator subset should not be able to reconstruct randomness via the fast path if `subset_power / total_power <= fast_path_secrecy_threshold`,
        fast_path_secrecy_threshold: FixedPoint64,
    }

    /// Initialize the configuration. Used in genesis or governance.
    public fun initialize(framework: &signer, config: RandomnessConfig) {
        system_addresses::assert_aptos_framework(framework);
        if (!exists<RandomnessConfig>(@aptos_framework)) {
            move_to(framework, config)
        }
    }

    /// This can be called by on-chain governance to update on-chain consensus configs for the next epoch.
    public fun set_for_next_epoch(framework: &signer, new_config: RandomnessConfig) {
        system_addresses::assert_aptos_framework(framework);
        config_buffer::upsert(new_config);
    }

    /// Only used in reconfigurations to apply the pending `RandomnessConfig`, if there is any.
    public(friend) fun on_new_epoch(framework: &signer) acquires RandomnessConfig {
        system_addresses::assert_aptos_framework(framework);
        if (config_buffer::does_exist<RandomnessConfig>()) {
            let new_config = config_buffer::extract<RandomnessConfig>();
            if (exists<RandomnessConfig>(@aptos_framework)) {
                *borrow_global_mut<RandomnessConfig>(@aptos_framework) = new_config;
            } else {
                move_to(framework, new_config);
            }
        }
    }

    /// Check whether on-chain randomness main logic (e.g., `DKGManager`, `RandManager`, `BlockMetadataExt`) is enabled.
    ///
    /// NOTE: this returning true does not mean randomness will run.
    /// The feature works if and only if `consensus_config::validator_txn_enabled() && randomness_config::enabled()`.
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
    public fun new_v1(secrecy_threshold: FixedPoint64, reconstruction_threshold: FixedPoint64): RandomnessConfig {
        RandomnessConfig {
            variant: copyable_any::pack( ConfigV1 {
                secrecy_threshold,
                reconstruction_threshold
            } )
        }
    }

    /// Create a `ConfigV2` variant.
    public fun new_v2(
        secrecy_threshold: FixedPoint64,
        reconstruction_threshold: FixedPoint64,
        fast_path_secrecy_threshold: FixedPoint64,
    ): RandomnessConfig {
        RandomnessConfig {
            variant: copyable_any::pack( ConfigV2 {
                secrecy_threshold,
                reconstruction_threshold,
                fast_path_secrecy_threshold,
            } )
        }
    }

    /// Get the currently effective randomness configuration object.
    public fun current(): RandomnessConfig acquires RandomnessConfig {
        if (exists<RandomnessConfig>(@aptos_framework)) {
            *borrow_global<RandomnessConfig>(@aptos_framework)
        } else {
            new_off()
        }
    }

    /// Return the typy name, the secrecy threshold, the reconstruction threshold and the fast-path secrecy threshold.
    public(friend) fun flatten(config: &RandomnessConfig): (String, FixedPoint64, FixedPoint64, FixedPoint64) {
        let type_name = *copyable_any::type_name(&config.variant);
        let type_name_bytes = *string::bytes(&type_name);
        if (type_name_bytes == b"0x1::randomness_config::ConfigOff") {
            let zero = fixed_point64::create_from_u128(0);
            (type_name, zero, zero, zero)
        } else if (type_name_bytes == b"0x1::randomness_config::ConfigV1") {
            let v1 = copyable_any::unpack<ConfigV1>(config.variant);
            (type_name, v1.secrecy_threshold, v1.reconstruction_threshold, fixed_point64::create_from_u128(0))
        } else if (type_name_bytes == b"0x1::randomness_config::ConfigV2") {
            let v2 = copyable_any::unpack<ConfigV2>(config.variant);
            (type_name, v2.secrecy_threshold, v2.reconstruction_threshold, v2.fast_path_secrecy_threshold)
        } else {
            let zero = fixed_point64::create_from_u128(0);
            (type_name, zero, zero, zero)
        }
    }

    #[test_only]
    fun initialize_for_testing(framework: &signer) {
        config_buffer::initialize(framework);
        initialize(framework, new_off());
    }

    #[test(framework = @0x1)]
    fun init_buffer_apply(framework: signer) acquires RandomnessConfig {
        initialize_for_testing(&framework);

        // Enabling.
        let config = new_v1(
            fixed_point64::create_from_rational(1, 2),
            fixed_point64::create_from_rational(2, 3)
        );
        set_for_next_epoch(&framework, config);
        on_new_epoch(&framework);
        assert!(enabled(), 1);

        // Disabling.
        set_for_next_epoch(&framework, new_off());
        on_new_epoch(&framework);
        assert!(!enabled(), 2);
    }
}
