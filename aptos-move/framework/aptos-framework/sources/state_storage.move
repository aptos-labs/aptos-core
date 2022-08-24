module aptos_framework::state_storage {

    use aptos_framework::system_addresses;
    use std::error;

    friend aptos_framework::block;
    friend aptos_framework::genesis;
    friend aptos_framework::reconfiguration;

    const ESTATE_STORAGE_USAGE: u64 = 0;
    const EGAS_PARAMETER: u64 = 1;
    const EEPOCH_ZERO: u64 = 2;

    struct Usage has copy, drop, store {
        items: u64,
        bytes: u64,
    }

    /// This is updated at the begining of each opoch, reflecting the storage
    /// usage after the last txn of the previous epoch is committed.
    struct StateStorageUsage has key, store {
        epoch: u64,
        usage: Usage,
    }

    /// This updates at reconfig and guarantees not to change elsewhere, safe
    /// for gas calculation.
    ///
    /// Specifically, it copies the usage at the begining of the concluding
    /// epoch for gas calculation of the entire next epoch. -- The data is one
    /// epoch older than ideal, but the Vm doesn't need to worry about reloading
    /// gas parameters after the first txn of an epoch.
    struct GasParameter has key, store {
        usage: Usage,
    }

    public(friend) fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(
            !exists<StateStorageUsage>(@aptos_framework),
            error::already_exists(ESTATE_STORAGE_USAGE)
        );
        assert!(
            !exists<GasParameter>(@aptos_framework),
            error::already_exists(EGAS_PARAMETER)
        );
        move_to(aptos_framework, StateStorageUsage {
            epoch: 0,
            usage: Usage {
                items: 0,
                bytes: 0,
            }
        });
        move_to(aptos_framework, GasParameter {
            usage: Usage {
                items: 0,
                bytes: 0,
            }
        });
    }

    public(friend) fun on_new_block(epoch: u64) acquires StateStorageUsage {
        let usage = borrow_global_mut<StateStorageUsage>(@aptos_framework);
        if (epoch != usage.epoch) {
            usage.epoch = epoch;
            usage.usage = get_state_storage_usage_only_at_epoch_beginning();
        }
    }

    public(friend) fun on_reconfig() acquires StateStorageUsage, GasParameter {
        let gas_parameter = borrow_global_mut<GasParameter>(@aptos_framework);
        let usage = borrow_global<StateStorageUsage>(@aptos_framework);
        gas_parameter.usage = usage.usage;
    }

    /// Warning: the result returned is based on the base state view held by the
    /// VM for the entire block or chunk of transactions, it's only deterministic
    /// if called from the first transaction of the block because the execution layer
    /// guarantees a fresh state view then.
    native fun get_state_storage_usage_only_at_epoch_beginning(): Usage;
}
