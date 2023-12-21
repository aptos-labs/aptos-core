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
module std::config_for_next_epoch {

    use std::signer::address_of;

    const ESTD_SIGNER_NEEDED: u64 = 1;
    const ERESOURCE_BUSY: u64 = 2;

    /// `ForNextEpoch<T>` under account 0x1 holds the config payload for the next epoch, where `T` can be `ConsnsusConfig`, `Features`, etc.
    struct ForNextEpoch<T> has drop, key {
        payload: T,
    }

    /// This flag exists under account 0x1 if and only if any on-chain config change for the next epoch should be rejected.
    struct UpsertLocked has copy, drop, key {}

    public fun upserts_enabled(): bool {
        !exists<UpsertLocked>(@std)
    }

    /// Disable on-chain config updates. Called by the system when a reconfiguration with DKG starts.
    public fun disable_upserts(account: &signer) {
        abort_unless_std(account);
        if (!exists<UpsertLocked>(@std)) {
            move_to(account, UpsertLocked {})
        }
    }

    /// Enable on-chain config updates. Called by the system when a reconfiguration with DKG finishes.
    public fun enable_upserts(account: &signer) acquires UpsertLocked {
        abort_unless_std(account);
        if (!exists<UpsertLocked>(@std)) {
            move_from<UpsertLocked>(address_of(account));
        }
    }

    /// Check whether there is a pending config payload for `T`.
    public fun does_exist<T: store>(): bool {
        exists<ForNextEpoch<T>>(@std)
    }

    /// Upsert an on-chain config to the buffer for the next epoch.
    ///
    /// Typically used in `X::set_for_next_epoch()` where X is an on-chaon config.
    public fun upsert<T: drop + store>(account: &signer, config: T) acquires ForNextEpoch {
        abort_unless_std(account);
        assert!(upserts_enabled(), std::error::invalid_state(ERESOURCE_BUSY));
        if (exists<ForNextEpoch<T>>(@std)) {
            move_from<ForNextEpoch<T>>(@std);
        };
        move_to(account, ForNextEpoch { payload: config });
    }

    /// Take the buffered config `T` out (buffer cleared). Abort if the buffer is empty.
    /// Should only be used at the end of a reconfiguration.
    ///
    /// Typically used in `X::on_new_epoch()` where X is an on-chaon config.
    public fun extract<T: store>(account: &signer): T acquires ForNextEpoch {
        abort_unless_std(account);
        let ForNextEpoch<T> { payload } = move_from<ForNextEpoch<T>>(@std);
        payload
    }

    fun abort_unless_std(account: &signer) {
        let addr = std::signer::address_of(account);
        assert!(addr == @std, std::error::permission_denied(ESTD_SIGNER_NEEDED));
    }
}
