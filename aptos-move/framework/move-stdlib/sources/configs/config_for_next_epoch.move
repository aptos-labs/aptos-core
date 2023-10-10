/// This wrapper helps store an on-chain config for the next epoch.
module std::config_for_next_epoch {

    use std::option;
    use std::option::Option;

    const ESYSTEM_SIGNER_NEEDED: u64 = 1;
    const ERESOURCE_BUSY: u64 = 2;

    struct ForNextEpoch<T> has drop, key {
        payload: Option<T>,
    }

    struct UpdateLock has drop, key {
        locked: bool,
    }

    public fun updates_enabled(): bool acquires UpdateLock {
        borrow_global<UpdateLock>(@std).locked
    }

    public fun disable_updates(account: &signer) acquires UpdateLock {
        abort_unless_system_account(account);
        borrow_global_mut<UpdateLock>(@std).locked = true;
    }

    public fun enable_updates(account: &signer) acquires UpdateLock {
        abort_unless_system_account(account);
        borrow_global_mut<UpdateLock>(@std).locked = false;
    }

    public fun does_exist<T: store>(): bool acquires ForNextEpoch {
        exists<ForNextEpoch<T>>(@std) && option::is_some(&borrow_global<ForNextEpoch<T>>(@std).payload)
    }

    public fun upsert<T: drop + store>(std: &signer, config: T) acquires ForNextEpoch {
        abort_unless_system_account(std);
        abort_if_updates_disabled();
        borrow_global_mut<ForNextEpoch<T>>(@std).payload = option::some(config);
    }

    public fun extract<T: store>(account: &signer): T acquires ForNextEpoch {
        abort_unless_system_account(account);
        abort_if_updates_disabled();
        option::extract(&mut borrow_global_mut<ForNextEpoch<T>>(@std).payload)
    }

    fun abort_unless_system_account(std: &signer) {
        let addr = std::signer::address_of(std);
        assert!(addr == @std || addr == @vm, std::error::permission_denied(ESYSTEM_SIGNER_NEEDED));
    }

    fun abort_if_updates_disabled() {
        assert!(!exists<UpdateLock>(@std), std::error::invalid_state(ERESOURCE_BUSY));
    }
}
