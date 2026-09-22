/// On-chain config for the epoch force-end watchdog.
///
/// When configured with `force_end_grace_period_secs = some(n)`, an in-progress
/// reconfiguration is finalized unconditionally (regardless of DKG state) once
///   `now >= last_reconfiguration_time + epoch_interval + n_secs`.
///
/// When `force_end_grace_period_secs = none`, the watchdog is disabled.
module aptos_framework::epoch_timeout_config {
    use std::error;
    use std::option::Option;
    use aptos_framework::config_buffer;
    use aptos_framework::system_addresses;

    friend aptos_framework::reconfiguration_with_dkg;

    /// `new_with_grace_period(0)` is disallowed: a zero grace period would cause
    /// the watchdog to fire in the same block prologue that triggers reconfig,
    /// skipping DKG entirely. Use `new_disabled()` if you mean to disable the
    /// watchdog.
    const E_GRACE_PERIOD_MUST_BE_POSITIVE: u64 = 1;

    struct EpochTimeoutConfig has copy, drop, key, store {
        force_end_grace_period_secs: Option<u64>,
    }

    /// Initialize the configuration. Used in genesis or governance.
    public fun initialize(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        if (!exists<EpochTimeoutConfig>(@aptos_framework)) {
            move_to(framework, new_disabled())
        }
    }

    /// Used by on-chain governance to update the watchdog config for the next epoch.
    public fun set_for_next_epoch(framework: &signer, new_config: EpochTimeoutConfig) {
        system_addresses::assert_aptos_framework(framework);
        config_buffer::upsert(new_config);
    }

    /// Only used in reconfigurations to apply the pending `EpochTimeoutConfig`, if there is any.
    public(friend) fun on_new_epoch(framework: &signer) acquires EpochTimeoutConfig {
        system_addresses::assert_aptos_framework(framework);
        if (config_buffer::does_exist<EpochTimeoutConfig>()) {
            let new_config = config_buffer::extract_v2<EpochTimeoutConfig>();
            if (exists<EpochTimeoutConfig>(@aptos_framework)) {
                *borrow_global_mut<EpochTimeoutConfig>(@aptos_framework) = new_config;
            } else {
                move_to(framework, new_config);
            }
        }
    }

    public fun new_disabled(): EpochTimeoutConfig {
        EpochTimeoutConfig { force_end_grace_period_secs: std::option::none() }
    }

    /// Build a watchdog config with a positive grace period (seconds). The
    /// grace period is the slack allowed *beyond* the epoch interval before
    /// the watchdog force-finalizes the reconfig. Aborts on `grace_period_secs
    /// == 0` — pass through `new_disabled()` to turn the watchdog off.
    public fun new_with_grace_period(grace_period_secs: u64): EpochTimeoutConfig {
        assert!(
            grace_period_secs > 0,
            error::invalid_argument(E_GRACE_PERIOD_MUST_BE_POSITIVE),
        );
        EpochTimeoutConfig {
            force_end_grace_period_secs: std::option::some(grace_period_secs)
        }
    }

    /// Return the configured grace period in seconds, or `none` if the watchdog is disabled
    /// (or the resource has not been initialized).
    public fun force_end_grace_period_secs(): Option<u64> acquires EpochTimeoutConfig {
        if (exists<EpochTimeoutConfig>(@aptos_framework)) {
            borrow_global<EpochTimeoutConfig>(@aptos_framework).force_end_grace_period_secs
        } else {
            std::option::none()
        }
    }

    #[test_only]
    fun initialize_for_testing(framework: &signer) {
        config_buffer::initialize(framework);
        initialize(framework);
    }

    #[test(framework = @0x1)]
    fun init_buffer_apply(framework: signer) acquires EpochTimeoutConfig {
        initialize_for_testing(&framework);
        assert!(force_end_grace_period_secs().is_none(), 1);

        set_for_next_epoch(&framework, new_with_grace_period(30));
        on_new_epoch(&framework);
        let gp = force_end_grace_period_secs();
        assert!(gp.is_some(), 2);
        assert!(*gp.borrow() == 30, 3);

        set_for_next_epoch(&framework, new_disabled());
        on_new_epoch(&framework);
        assert!(force_end_grace_period_secs().is_none(), 4);
    }

    #[test(framework = @0x1)]
    fun disabled_when_uninitialized(framework: signer) acquires EpochTimeoutConfig {
        config_buffer::initialize(&framework);
        // Note: no `initialize(&framework)` call — resource does not exist.
        assert!(!exists<EpochTimeoutConfig>(@aptos_framework), 1);
        assert!(force_end_grace_period_secs().is_none(), 2);
    }

    #[test(framework = @0x1)]
    fun idempotent_initialize(framework: signer) acquires EpochTimeoutConfig {
        initialize_for_testing(&framework);

        // Set a value, then call initialize again — the existing value must be preserved.
        set_for_next_epoch(&framework, new_with_grace_period(45));
        on_new_epoch(&framework);
        assert!(*force_end_grace_period_secs().borrow() == 45, 1);

        initialize(&framework);
        assert!(*force_end_grace_period_secs().borrow() == 45, 2);
    }

    #[test(framework = @0x1)]
    fun toggle_across_reconfigs(framework: signer) acquires EpochTimeoutConfig {
        initialize_for_testing(&framework);

        // disabled -> enabled(10)
        set_for_next_epoch(&framework, new_with_grace_period(10));
        on_new_epoch(&framework);
        assert!(*force_end_grace_period_secs().borrow() == 10, 1);

        // enabled(10) -> disabled
        set_for_next_epoch(&framework, new_disabled());
        on_new_epoch(&framework);
        assert!(force_end_grace_period_secs().is_none(), 2);

        // disabled -> enabled(99): proves we can re-enable after disable.
        set_for_next_epoch(&framework, new_with_grace_period(99));
        on_new_epoch(&framework);
        assert!(*force_end_grace_period_secs().borrow() == 99, 3);

        // An on_new_epoch with no pending change is a no-op.
        on_new_epoch(&framework);
        assert!(*force_end_grace_period_secs().borrow() == 99, 4);
    }

    #[test(framework = @0x1, attacker = @0xa11ce)]
    #[expected_failure(abort_code = 0x50003, location = aptos_framework::system_addresses)]
    fun non_framework_signer_cannot_set(framework: signer, attacker: signer) {
        initialize_for_testing(&framework);
        set_for_next_epoch(&attacker, new_with_grace_period(10));
    }

    #[test]
    #[expected_failure(abort_code = 0x10001, location = Self)]
    fun zero_grace_period_aborts() {
        let _ = new_with_grace_period(0);
    }
}
