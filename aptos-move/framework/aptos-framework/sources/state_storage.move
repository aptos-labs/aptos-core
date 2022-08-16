module aptos_framework::state_storage {

    use aptos_framework::timestamp;
    use aptos_framework::system_addresses;
    use std::error;

    friend aptos_framework::genesis;
    friend aptos_framework::block;

    const EUSAGE_ALREADY_EXISTS: u64 = 0;
    const EEPOCH_ZERO: u64 = 1;

    struct StateStorageUsage has copy, drop, key, store {
        items: u64,
        bytes: u64,
    }

    public(friend) fun initialize(account: &signer) {
        timestamp::assert_genesis();
        system_addresses::assert_aptos_framework(account);
        assert!(
            !exists<StateStorageUsage>(@aptos_framework),
            error::already_exists(EUSAGE_ALREADY_EXISTS)
        );
        move_to(account, StateStorageUsage {
            items: 0,
            bytes: 0,
        });
    }

    public(friend) fun on_epoch_begin() acquires StateStorageUsage {
        *borrow_global_mut<StateStorageUsage>(@aptos_framework)
            = native_get_state_storage_usage_only_at_epoch_beginning()
    }

    /// Warning: the result returned is based on the base state view held by the
    /// VM for the entire block or chunk of transactions, it's only deterministic
    /// if called from the first transaction of the block because the execution layer
    /// guarantees a fresh state view then.
    native fun native_get_state_storage_usage_only_at_epoch_beginning(): StateStorageUsage;
}
