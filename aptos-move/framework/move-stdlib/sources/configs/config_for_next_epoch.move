/// This wrapper helps store an on-chain config for the next epoch.
module std::config_for_next_epoch {

    use std::signer::address_of;

    const ESTD_OR_VM_SIGNER_NEEDED: u64 = 1;
    const ESTD_SIGNER_NEEDED: u64 = 2;
    const EVM_SIGNER_NEEDED: u64 = 3;
    const ERESOURCE_BUSY: u64 = 4;

    /// `0x1::ForNextEpoch<T>` holds the config payload for the next epoch, where `T` can be `ConsnsusConfig`, `Features`, etc.
    struct ForNextEpoch<T> has drop, key {
        payload: T,
    }

    /// We need to temporarily reject on-chain config changes during DKG.
    /// `0x0::UpdateLock` or `0x1::UpdateLock`, whichever has the higher `seq_num`, represents whether we should reject.
    struct UpdateLock has copy, drop, key {
        seq_num: u64,
        locked: bool,
    }

    public fun updates_enabled(): bool acquires UpdateLock {
        !latest_lock_state().locked
    }

    /// Disable on-chain config updates. Only needed when a reconfiguration with DKG starts.
    public fun disable_updates(account: &signer) acquires UpdateLock {
        update_lock_state(account, true);
    }

    /// Enable on-chain config updates. Only needed when a reconfiguration with DKG finishes.
    public fun enable_updates(account: &signer) acquires UpdateLock {
        update_lock_state(account, false);
    }

    /// Check whether there is a pending config payload for `T`.
    public fun does_exist<T: store>(): bool {
        exists<ForNextEpoch<T>>(@std)
    }

    /// Save an on-chain config to be used in the next epoch.
    /// Typically followed by a `aptos_framework::reconfigure::start_reconfigure_with_dkg()` to make it effective as soon as possible.
    public fun upsert<T: drop + store>(std: &signer, config: T) acquires ForNextEpoch, UpdateLock {
        abort_unless_std(std);
        abort_unless_updates_enabled();
        if (exists<ForNextEpoch<T>>(@std)) {
            move_from<ForNextEpoch<T>>(@std);
        };
        move_to(std, ForNextEpoch { payload: config });
    }

    /// Extract the config payload. Should be called at the end of a reconfiguration with DKG.
    /// It is assumed that the caller has checked existence using `does_exist()`.
    public fun extract<T: store>(account: &signer): T acquires ForNextEpoch, UpdateLock {
        abort_unless_vm_or_std(account);
        abort_unless_updates_enabled();
        let ForNextEpoch<T> { payload } = move_from<ForNextEpoch<T>>(@std);
        payload
    }

    fun lock_state(addr: address): UpdateLock acquires UpdateLock {
        if (exists<UpdateLock>(addr)) {
            *borrow_global<UpdateLock>(addr)
        } else {
            UpdateLock {
                seq_num: 0,
                locked: false,
            }
        }
    }

    fun latest_lock_state(): UpdateLock acquires UpdateLock {
        let state_0 = lock_state(@vm);
        let state_1 = lock_state(@std);
        if (state_0.seq_num > state_1.seq_num) {
            state_0
        } else {
            state_1
        }
    }

    fun update_lock_state(account: &signer, locked: bool) acquires UpdateLock {
        abort_unless_vm_or_std(account);

        let latest_lock_state = latest_lock_state();

        if (exists<UpdateLock>(address_of(account))) {
            move_from<UpdateLock>(address_of(account));
        };

        let new_state = UpdateLock {
            seq_num: latest_lock_state.seq_num + 1,
            locked,
        };
        move_to(account, new_state);
    }

    fun abort_unless_vm_or_std(std: &signer) {
        let addr = std::signer::address_of(std);
        assert!(addr == @std || addr == @vm, std::error::permission_denied(ESTD_OR_VM_SIGNER_NEEDED));
    }

    fun abort_unless_std(std: &signer) {
        let addr = std::signer::address_of(std);
        assert!(addr == @std, std::error::permission_denied(ESTD_SIGNER_NEEDED));
    }

    fun abort_unless_updates_enabled() acquires UpdateLock {
        assert!(!latest_lock_state().locked, std::error::invalid_state(ERESOURCE_BUSY));
    }
}
