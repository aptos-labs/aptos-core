/// Structs and functions for on-chain chunky DKG configurations.
module aptos_framework::chunky_dkg_config {
    use std::option;
    use std::option::Option;
    use aptos_std::copyable_any;
    use aptos_std::copyable_any::Any;
    use aptos_std::fixed_point64::FixedPoint64;
    use aptos_framework::config_buffer;
    use aptos_framework::system_addresses;

    friend aptos_framework::reconfiguration_with_dkg;

    /// The configuration of the on-chain chunky DKG feature.
    struct ChunkyDKGConfig has copy, drop, key, store {
        /// A config variant packed as an `Any`.
        /// Currently the variant type is one of the following.
        /// - `ConfigOff`
        /// - `ConfigV1`
        /// - `ConfigShadowV1`
        variant: Any
    }

    /// A chunky DKG config variant indicating the feature is disabled.
    struct ConfigOff has copy, drop, store {}

    /// A chunky DKG config variant indicating the feature is enabled.
    struct ConfigV1 has copy, drop, store {
        /// Any validator subset should not be able to reconstruct randomness if `subset_power / total_power <= secrecy_threshold`,
        secrecy_threshold: FixedPoint64,
        /// Any validator subset should be able to reconstruct randomness if `subset_power / total_power > reconstruction_threshold`.
        reconstruction_threshold: FixedPoint64
    }

    /// A chunky DKG config variant for shadow mode: chunky DKG runs alongside regular DKG,
    /// but epoch change is forced after `grace_period_secs` if chunky DKG hasn't completed.
    struct ConfigShadowV1 has copy, drop, store {
        secrecy_threshold: FixedPoint64,
        reconstruction_threshold: FixedPoint64,
        grace_period_secs: u64
    }

    /// Initialize the configuration. Used in genesis or governance.
    public fun initialize(framework: &signer, config: ChunkyDKGConfig) {
        system_addresses::assert_aptos_framework(framework);
        if (!exists<ChunkyDKGConfig>(@aptos_framework)) {
            move_to(framework, config)
        }
    }

    /// This can be called by on-chain governance to update on-chain consensus configs for the next epoch.
    public fun set_for_next_epoch(
        framework: &signer, new_config: ChunkyDKGConfig
    ) {
        system_addresses::assert_aptos_framework(framework);
        config_buffer::upsert(new_config);
    }

    /// Only used in reconfigurations to apply the pending `ChunkyDKGConfig`, if there is any.
    public(friend) fun on_new_epoch(framework: &signer) acquires ChunkyDKGConfig {
        system_addresses::assert_aptos_framework(framework);
        if (config_buffer::does_exist<ChunkyDKGConfig>()) {
            let new_config = config_buffer::extract_v2<ChunkyDKGConfig>();
            if (exists<ChunkyDKGConfig>(@aptos_framework)) {
                *borrow_global_mut<ChunkyDKGConfig>(@aptos_framework) = new_config;
            } else {
                move_to(framework, new_config);
            }
        }
    }

    /// Check whether on-chain chunky DKG main logic is enabled.
    ///
    /// NOTE: this returning true does not mean chunky DKG will run.
    /// The feature works if and only if `consensus_config::validator_txn_enabled() && chunky_dkg_config::enabled()`.
    public fun enabled(): bool acquires ChunkyDKGConfig {
        if (exists<ChunkyDKGConfig>(@aptos_framework)) {
            let config = borrow_global<ChunkyDKGConfig>(@aptos_framework);
            let variant_type_name = *config.variant.type_name().bytes();
            variant_type_name != b"0x1::chunky_dkg_config::ConfigOff"
        } else { false }
    }

    /// Create a `ConfigOff` variant.
    public fun new_off(): ChunkyDKGConfig {
        ChunkyDKGConfig {
            variant: copyable_any::pack(ConfigOff {})
        }
    }

    /// Create a `ConfigV1` variant.
    public fun new_v1(
        secrecy_threshold: FixedPoint64, reconstruction_threshold: FixedPoint64
    ): ChunkyDKGConfig {
        ChunkyDKGConfig {
            variant: copyable_any::pack(
                ConfigV1 { secrecy_threshold, reconstruction_threshold }
            )
        }
    }

    /// Create a `ConfigShadowV1` variant for shadow mode.
    public fun new_shadow_v1(
        secrecy_threshold: FixedPoint64,
        reconstruction_threshold: FixedPoint64,
        grace_period_secs: u64
    ): ChunkyDKGConfig {
        ChunkyDKGConfig {
            variant: copyable_any::pack(
                ConfigShadowV1 { secrecy_threshold, reconstruction_threshold, grace_period_secs }
            )
        }
    }

    /// Return the grace period in seconds if configured (i.e. shadow mode), or none otherwise.
    public fun grace_period_secs(): Option<u64> acquires ChunkyDKGConfig {
        if (exists<ChunkyDKGConfig>(@aptos_framework)) {
            let config = borrow_global<ChunkyDKGConfig>(@aptos_framework);
            let variant_type_name = *config.variant.type_name().bytes();
            if (variant_type_name == b"0x1::chunky_dkg_config::ConfigShadowV1") {
                let shadow_v1 = copyable_any::unpack<ConfigShadowV1>(config.variant);
                return std::option::some(shadow_v1.grace_period_secs)
            }
        };
        option::none()
    }

    /// Get the currently effective chunky DKG configuration object.
    public fun current(): ChunkyDKGConfig acquires ChunkyDKGConfig {
        if (exists<ChunkyDKGConfig>(@aptos_framework)) {
            *borrow_global<ChunkyDKGConfig>(@aptos_framework)
        } else {
            new_off()
        }
    }

    #[test_only]
    use aptos_std::fixed_point64;

    #[test_only]
    fun initialize_for_testing(framework: &signer) {
        config_buffer::initialize(framework);
        initialize(framework, new_off());
    }

    #[test(framework = @0x1)]
    fun init_buffer_apply(framework: signer) acquires ChunkyDKGConfig {
        initialize_for_testing(&framework);

        // Enabling.
        let config =
            new_v1(
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

    #[test(framework = @0x1)]
    fun shadow_v1_config(framework: signer) acquires ChunkyDKGConfig {
        initialize_for_testing(&framework);

        // Shadow mode is enabled but has a grace period.
        let config =
            new_shadow_v1(
                fixed_point64::create_from_rational(1, 2),
                fixed_point64::create_from_rational(2, 3),
                30
            );
        set_for_next_epoch(&framework, config);
        on_new_epoch(&framework);
        assert!(enabled(), 1);
        let gp = grace_period_secs();
        assert!(gp.is_some(), 2);
        assert!(*gp.borrow() == 30, 3);

        // ConfigV1 has no grace period.
        let config =
            new_v1(
                fixed_point64::create_from_rational(1, 2),
                fixed_point64::create_from_rational(2, 3)
            );
        set_for_next_epoch(&framework, config);
        on_new_epoch(&framework);
        assert!(enabled(), 4);
        assert!(grace_period_secs().is_none(), 5);

        // ConfigOff has no grace period and is not enabled.
        set_for_next_epoch(&framework, new_off());
        on_new_epoch(&framework);
        assert!(!enabled(), 6);
        assert!(grace_period_secs().is_none(), 7);
    }
}
