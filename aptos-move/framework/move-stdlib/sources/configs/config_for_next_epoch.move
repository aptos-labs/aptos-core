/// This wrapper helps store an on-chain config for the next epoch.
module std::config_for_next_epoch {

    use std::signer::address_of;

    const ESTD_OR_VM_SIGNER_NEEDED: u64 = 1;
    const ESTD_SIGNER_NEEDED: u64 = 2;
    const EVM_SIGNER_NEEDED: u64 = 3;
    const ERESOURCE_BUSY: u64 = 4;
    const EPERMISSION_DENIED: u64 = 5;

    /// `0x1::ForNextEpoch<T>` holds the config payload for the next epoch, where `T` can be `ConsnsusConfig`, `Features`, etc.
    struct ForNextEpoch<T> has drop, key {
        payload: T,
    }

    /// We need to temporarily reject on-chain config changes during DKG.
    /// `0x0::UpdateLock` or `0x1::UpdateLock`, whichever has the higher `seq_num`, represents whether we should reject.
    struct UpsertLock has copy, drop, key {
        seq_num: u64,
        locked: bool,
    }

    /// We need to allow extraction of pending configs ONLY when we are at the end of a reconfiguration.
    struct ExtractPermit has copy, drop, key {}

    public fun extracts_enabled(): bool {
        exists<ExtractPermit>(@vm) || exists<ExtractPermit>(@std)
    }

    public fun enable_extracts(account: &signer) {
        move_to(account, ExtractPermit {});
    }

    public fun disable_extracts(account: &signer) acquires ExtractPermit {
        move_from<ExtractPermit>(address_of(account));
    }

    public fun upserts_enabled(): bool acquires UpsertLock {
        !latest_upsert_lock_state().locked
    }

    /// Disable on-chain config updates. Only needed when a reconfiguration with DKG starts.
    public fun disable_upserts(account: &signer) acquires UpsertLock {
        set_upsert_lock_state(account, true);
    }

    /// Enable on-chain config updates. Only needed when a reconfiguration with DKG finishes.
    public fun enable_upserts(account: &signer) acquires UpsertLock {
        set_upsert_lock_state(account, false);
    }

    /// Check whether there is a pending config payload for `T`.
    public fun does_exist<T: store>(): bool {
        exists<ForNextEpoch<T>>(@std)
    }

    /// Return a copy of the buffered on-chain config. Abort if the buffer is empty.
    public fun copied<T: copy + store>(): T acquires ForNextEpoch {
        borrow_global<ForNextEpoch<T>>(@std).payload
    }

    /// Save an on-chain config to the buffer for the next epoch.
    /// If the buffer is not empty, put in the new one and discard the old one.
    /// Typically followed by a `aptos_framework::reconfigure::start_reconfigure_with_dkg()` to make it effective as soon as possible.
    public fun upsert<T: drop + store>(std: &signer, config: T) acquires ForNextEpoch, UpsertLock {
        assert!(address_of(std) == @std, std::error::permission_denied(ESTD_SIGNER_NEEDED));
        assert!(!latest_upsert_lock_state().locked, std::error::invalid_state(ERESOURCE_BUSY));
        if (exists<ForNextEpoch<T>>(@std)) {
            move_from<ForNextEpoch<T>>(@std);
        };
        move_to(std, ForNextEpoch { payload: config });
    }

    /// Take the buffered config `T` out (buffer cleared). Abort if the buffer is empty.
    /// Should only be used at the end of a reconfiguration.
    ///
    /// NOTE: The caller has to ensure updates are enabled using `enable_updates()`.
    public fun extract<T: store>(): T acquires ForNextEpoch {
        assert!(!extracts_enabled(), std::error::invalid_state(EPERMISSION_DENIED));
        let ForNextEpoch<T> { payload } = move_from<ForNextEpoch<T>>(@std);
        payload
    }

    fun upsert_lock_state(addr: address): UpsertLock acquires UpsertLock {
        if (exists<UpsertLock>(addr)) {
            *borrow_global<UpsertLock>(addr)
        } else {
            UpsertLock {
                seq_num: 0,
                locked: false,
            }
        }
    }

    fun latest_upsert_lock_state(): UpsertLock acquires UpsertLock {
        let state_0 = upsert_lock_state(@vm);
        let state_1 = upsert_lock_state(@std);
        if (state_0.seq_num > state_1.seq_num) {
            state_0
        } else {
            state_1
        }
    }

    fun set_upsert_lock_state(account: &signer, locked: bool) acquires UpsertLock {
        abort_unless_vm_or_std(account);

        let latest_state = latest_upsert_lock_state();

        if (exists<UpsertLock>(address_of(account))) {
            move_from<UpsertLock>(address_of(account));
        };

        let new_state = UpsertLock {
            seq_num: latest_state.seq_num + 1,
            locked,
        };
        move_to(account, new_state);
    }

    fun abort_unless_vm_or_std(account: &signer) {
        let addr = std::signer::address_of(account);
        assert!(addr == @std || addr == @vm, std::error::permission_denied(ESTD_OR_VM_SIGNER_NEEDED));
    }
}
