module aptos_framework::state_storage {

    use aptos_framework::system_addresses;
    use std::error;

    friend aptos_framework::block;
    friend aptos_framework::genesis;
    friend aptos_framework::reconfiguration;

    const ESTATE_STORAGE_USAGE: u64 = 0;
    const ESTORAGE_GAS_PARAMETER_INPUT: u64 = 1;
    const ESTORAGE_GAS_PARAMETER: u64 = 2;
    const EEPOCH_ZERO: u64 = 3;

    const STATE_STORAGE_TARGET_ITEMS: u64 = 1000000000;   // 1 billion
    const STATE_STORAGE_TARGET_BYTES: u64 = 250000000000;   // 250 GB
    const SCALE_FACTOR: u64 = 10000;

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
    /// Specifically, it is updated based on the usage at the begining of the
    /// current epoch when end it by executing a reconfig transaction. The gas
    /// schedule derived from these parameter will be for gas calculation of
    /// the entire next epoch.
    /// -- The data is one epoch stale than ideal, but VM doesn't need to worry
    /// about reloading gas parameters after the first txn of an epoch.
    struct StorageGasParameter has key, store {
        per_item: u64,
        per_byte: u64,
    }

    public(friend) fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(
            !exists<StateStorageUsage>(@aptos_framework),
            error::already_exists(ESTATE_STORAGE_USAGE)
        );
        assert!(
            !exists<StorageGasParameter>(@aptos_framework),
            error::already_exists(ESTORAGE_GAS_PARAMETER)
        );
        move_to(aptos_framework, StateStorageUsage {
            epoch: 0,
            usage: Usage {
                items: 0,
                bytes: 0,
            }
        });
        move_to(aptos_framework, StorageGasParameter {
            per_byte: 0,
            per_item: 0,
        });
    }

    /// The storage gas base follows a 1/4 arc curve and scale it to [0, SCALE_FACTOR].
    fun calculate_gas(target: u64, current: u64): u64 {
        let r = (target as u128);
        let x = if (current < target) {(current as u128)} else {r};
        let y = (isqrt(r * r - x * x) as u64);
        (target - y) * SCALE_FACTOR / target
    }

    // Find the greatest number x that x * 2 <= n.
    fun isqrt(n: u128): u128{
        let x = n;
        let c = 0;
        let d = 1u128 << 126;
        while (d > n) {
            d = d >> 2;
        };
        while (d != 0) {
            if (x >= c + d) {
                x = x - (c + d);
                c = (c >> 1) + d;
            }
            else {
                c = c >> 1;
            };
            d = d >> 2;
        };
        c
    }

    public(friend) fun on_new_block(epoch: u64) acquires StateStorageUsage {
        let usage = borrow_global_mut<StateStorageUsage>(@aptos_framework);
        if (epoch != usage.epoch) {
            usage.epoch = epoch;
            usage.usage = get_state_storage_usage_only_at_epoch_beginning();
        }
    }

    public(friend) fun on_reconfig() acquires StateStorageUsage, StorageGasParameter {
        assert!(
            exists<StateStorageUsage>(@aptos_framework),
            error::not_found(ESTATE_STORAGE_USAGE)
        );
        assert!(
            exists<StorageGasParameter>(@aptos_framework),
            error::not_found(ESTORAGE_GAS_PARAMETER)
        );
        let usage = borrow_global<StateStorageUsage>(@aptos_framework);
        let gas_parameter = borrow_global_mut<StorageGasParameter>(@aptos_framework);
        gas_parameter.per_byte = calculate_gas(STATE_STORAGE_TARGET_BYTES, usage.usage.bytes);
        gas_parameter.per_item = calculate_gas(STATE_STORAGE_TARGET_ITEMS, usage.usage.items);
    }

    /// Warning: the result returned is based on the base state view held by the
    /// VM for the entire block or chunk of transactions, it's only deterministic
    /// if called from the first transaction of the block because the execution layer
    /// guarantees a fresh state view then.
    native fun get_state_storage_usage_only_at_epoch_beginning(): Usage;

    #[test(framework = @aptos_framework)]
    fun test_initialize_and_reconfig(framework: signer) acquires StateStorageUsage, StorageGasParameter {
        initialize(&framework);
        on_reconfig();
        let gas_parameter = borrow_global<StorageGasParameter>(@aptos_framework);
        assert!(gas_parameter.per_item == 0, 0);
        assert!(gas_parameter.per_byte == 0, 0);
    }
}
