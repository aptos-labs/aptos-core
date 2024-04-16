module supra_framework::state_storage {

    use supra_framework::system_addresses;
    use std::error;

    friend supra_framework::block;
    friend supra_framework::genesis;
    friend supra_framework::storage_gas;

    const ESTATE_STORAGE_USAGE: u64 = 0;

    struct Usage has copy, drop, store {
        items: u64,
        bytes: u64,
    }

    /// This is updated at the beginning of each epoch, reflecting the storage
    /// usage after the last txn of the previous epoch is committed.
    struct StateStorageUsage has key, store {
        epoch: u64,
        usage: Usage,
    }

    public(friend) fun initialize(supra_framework: &signer) {
        system_addresses::assert_supra_framework(supra_framework);
        assert!(
            !exists<StateStorageUsage>(@supra_framework),
            error::already_exists(ESTATE_STORAGE_USAGE)
        );
        move_to(supra_framework, StateStorageUsage {
            epoch: 0,
            usage: Usage {
                items: 0,
                bytes: 0,
            }
        });
    }

    public(friend) fun on_new_block(epoch: u64) acquires StateStorageUsage {
        assert!(
            exists<StateStorageUsage>(@supra_framework),
            error::not_found(ESTATE_STORAGE_USAGE)
        );
        let usage = borrow_global_mut<StateStorageUsage>(@supra_framework);
        if (epoch != usage.epoch) {
            usage.epoch = epoch;
            usage.usage = get_state_storage_usage_only_at_epoch_beginning();
        }
    }

    public(friend) fun current_items_and_bytes(): (u64, u64) acquires StateStorageUsage {
        assert!(
            exists<StateStorageUsage>(@supra_framework),
            error::not_found(ESTATE_STORAGE_USAGE)
        );
        let usage = borrow_global<StateStorageUsage>(@supra_framework);
        (usage.usage.items, usage.usage.bytes)
    }

    /// Warning: the result returned is based on the base state view held by the
    /// VM for the entire block or chunk of transactions, it's only deterministic
    /// if called from the first transaction of the block because the execution layer
    /// guarantees a fresh state view then.
    native fun get_state_storage_usage_only_at_epoch_beginning(): Usage;

    #[test_only]
    public fun set_for_test(epoch: u64, items: u64, bytes: u64) acquires StateStorageUsage {
        assert!(
            exists<StateStorageUsage>(@supra_framework),
            error::not_found(ESTATE_STORAGE_USAGE)
        );
        let usage = borrow_global_mut<StateStorageUsage>(@supra_framework);
        usage.epoch = epoch;
        usage.usage = Usage {
            items,
            bytes
        };
    }

    // ======================== deprecated ============================
    friend supra_framework::reconfiguration;

    struct GasParameter has key, store {
        usage: Usage,
    }

    public(friend) fun on_reconfig() {
        abort 0
    }
}
