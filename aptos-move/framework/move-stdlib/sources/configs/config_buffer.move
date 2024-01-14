/// This wrapper helps store an on-chain config for the next epoch.
///
/// Once reconfigure with DKG is introduced, every on-chain config `C` should do the following.
/// - Support async update when DKG is enabled. This is typically done by 3 steps below.
///   - Implement `C::set_for_next_epoch()` using `upsert()` function in this module.
///   - Implement `C::on_new_epoch()` using `extract()` function in this module.
///   - Update `0x1::reconfiguration_with_dkg::finish()` to call `C::on_new_epoch()`.
/// - Support sychronous update when DKG is disabled.
///   This is typically done by implementing `C::set()` to update the config resource directly.
///
/// NOTE: on-chain config `0x1::state::ValidatorSet` implemented its own buffer.
module std::config_buffer {

    use std::signer;

    /// Config buffer operations failed with permission denied.
    const ESTD_SIGNER_NEEDED: u64 = 1;

    /// `ConfigBuffer<T>` under account 0x1 holds the payload of on-chain config `T` for the next epoch.
    /// Examples of `T`: `ConsensusConfig`, `Features`.
    struct ConfigBuffer<T> has drop, key {
        payload: T,
    }

    /// This flag exists under account 0x1 if and only if any validator set change for the next epoch should be rejected.
    struct ValidatorSetChangeLocked has copy, drop, key {}

    /// Return whether validator set changes are disabled (because of ongoing DKG).
    public fun validator_set_changes_disabled(): bool {
        exists<ValidatorSetChangeLocked>(@std)
    }

    /// When a DKG starts, call this to disable validator set changes.
    public fun disable_validator_set_changes(account: &signer) {
        abort_unless_std(account);
        if (!exists<ValidatorSetChangeLocked>(@std)) {
            move_to(account, ValidatorSetChangeLocked {})
        }
    }

    /// When a DKG finishes, call this to re-enable validator set changes.
    public fun enable_validator_set_changes(account: &signer) acquires ValidatorSetChangeLocked {
        abort_unless_std(account);
        if (!exists<ValidatorSetChangeLocked>(@std)) {
            move_from<ValidatorSetChangeLocked>(signer::address_of(account));
        }
    }

    /// Check whether there is a pending config payload for `T`.
    public fun does_exist<T: store>(): bool {
        exists<ConfigBuffer<T>>(@std)
    }

    /// Upsert an on-chain config to the buffer for the next epoch.
    ///
    /// Typically used in `X::set_for_next_epoch()` where X is an on-chain config.
    public fun upsert<T: drop + store>(account: &signer, config: T) acquires ConfigBuffer {
        abort_unless_std(account);
        if (exists<ConfigBuffer<T>>(@std)) {
            move_from<ConfigBuffer<T>>(@std);
        };
        move_to(account, ConfigBuffer { payload: config });
    }

    spec upsert {
        pragma opaque;
        pragma aborts_if_is_partial;
        modifies global<ConfigBuffer<T>>(@std);
    }

    /// Take the buffered config `T` out (buffer cleared). Abort if the buffer is empty.
    /// Should only be used at the end of a reconfiguration.
    ///
    /// Typically used in `X::on_new_epoch()` where X is an on-chaon config.
    public fun extract<T: store>(account: &signer): T acquires ConfigBuffer {
        abort_unless_std(account);
        let ConfigBuffer<T> { payload } = move_from<ConfigBuffer<T>>(@std);
        payload
    }

    spec extract {
        pragma opaque;
        pragma aborts_if_is_partial;
        modifies global<ConfigBuffer<T>>(@std);
    }

    fun abort_unless_std(account: &signer) {
        let addr = std::signer::address_of(account);
        assert!(addr == @std, std::error::permission_denied(ESTD_SIGNER_NEEDED));
    }

    #[test_only]
    struct DummyConfig has drop, store {
        data: u64,
    }

    #[test(fx = @std)]
    fun test_config_buffer_basic(fx: &signer) acquires ConfigBuffer {
        // Initially nothing in the buffer.
        assert!(!does_exist<DummyConfig>(), 1);

        // Insert should work.
        upsert(fx, DummyConfig { data: 888 });
        assert!(does_exist<DummyConfig>(), 1);

        // Update and extract should work.
        upsert(fx, DummyConfig { data: 999 });
        assert!(does_exist<DummyConfig>(), 1);
        let config = extract<DummyConfig>(fx);
        assert!(config == DummyConfig { data: 999 }, 1);
        assert!(!does_exist<DummyConfig>(), 1);
    }

    #[test(malice = @0x1234)]
    #[expected_failure(abort_code = 0x050001)]
    fun upsert_as_non_std_should_abort(malice: &signer) acquires ConfigBuffer {
        upsert(malice, DummyConfig { data: 888 });
    }

    #[test(malice = @0x1234)]
    #[expected_failure(abort_code = 0x050001)]
    fun extract_as_non_std_should_abort(malice: &signer) acquires ConfigBuffer {
        let _ = extract<DummyConfig>(malice);
    }

    #[test(malice = @0x1234)]
    #[expected_failure(abort_code = 0x050001)]
    fun disable_validator_set_change_as_non_std_should_abort(malice: &signer) {
        disable_validator_set_changes(malice);
    }

    #[test(malice = @0x1234)]
    #[expected_failure(abort_code = 0x050001)]
    fun enable_validator_set_change_as_non_std_should_abort(malice: &signer) acquires ValidatorSetChangeLocked {
        enable_validator_set_changes(malice);
    }
}
