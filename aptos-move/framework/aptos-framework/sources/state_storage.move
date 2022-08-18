module aptos_framework::state_storage {

    use aptos_framework::reconfiguration;
    use aptos_framework::system_addresses;
    use std::error;

    friend aptos_framework::genesis;
    friend aptos_framework::block;

    const EUSAGE_ALREADY_EXISTS: u64 = 0;
    const EEPOCH_ZERO: u64 = 1;

    struct StateStorageUsage has copy, drop, key, store {
        epoch: u64,
        items: u64,
        bytes: u64,
    }

    struct Usage_ {
        items: u64,
        bytes: u64,
    }

    public(friend) fun initialize(account: &signer) {
        system_addresses::assert_aptos_framework(account);
        assert!(
            !exists<StateStorageUsage>(@aptos_framework),
            error::already_exists(EUSAGE_ALREADY_EXISTS)
        );
        move_to(account, StateStorageUsage {
            items: 0,
            bytes: 0,
            epoch: 0,
        });
    }

    public(friend) fun on_new_block() acquires StateStorageUsage {
        let epoch = reconfiguration::current_epoch();
        let usage = borrow_global_mut<StateStorageUsage>(@aptos_framework);
        if (epoch != usage.epoch) {
            let Usage_ {
                items,
                bytes,
            } = get_state_storage_usage_only_at_epoch_beginning();

            *usage = StateStorageUsage {
                items,
                bytes,
                epoch,
            }
        }
    }

    /// Warning: the result returned is based on the base state view held by the
    /// VM for the entire block or chunk of transactions, it's only deterministic
    /// if called from the first transaction of the block because the execution layer
    /// guarantees a fresh state view then.
    native fun get_state_storage_usage_only_at_epoch_beginning(): Usage_;
}
