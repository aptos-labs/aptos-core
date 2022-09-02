module aptos_framework::storage_gas {

    use aptos_framework::system_addresses;
    use std::error;
    use aptos_framework::state_storage;
    use std::vector;

    friend aptos_framework::gas_schedule;
    friend aptos_framework::genesis;
    friend aptos_framework::reconfiguration;

    const ESTORAGE_GAS_CONFIG: u64 = 0;
    const ESTORAGE_GAS: u64 = 1;
    const EINVALID_GAS_RANGE: u64 = 2;
    const EZERO_TARGET_UTILIZATION: u64 = 3;
    const EINVALID_CURVE_LENGTH: u64 = 4;
    const EINVALID_CURVE_START_POINT: u64 = 5;
    const EINVALID_CURVE_END_POINT: u64 = 6;
    const EINVALID_MONOTONICALLY_NON_DECREASING_CURVE: u64 = 7;

    const BASIS_POINT_BASE: u64 = 10000;

    /// This updates at reconfig and guarantees not to change elsewhere, safe
    /// for gas calculation.
    ///
    /// Specifically, it is updated based on the usage at the begining of the
    /// current epoch when end it by executing a reconfig transaction. The gas
    /// schedule derived from these parameter will be for gas calculation of
    /// the entire next epoch.
    /// -- The data is one epoch stale than ideal, but VM doesn't need to worry
    /// about reloading gas parameters after the first txn of an epoch.
    struct StorageGas has key {
        per_item_read: u64,
        per_item_create: u64,
        per_item_write: u64,
        per_byte_read: u64,
        per_byte_create: u64,
        per_byte_write: u64,
    }

    // x and y are basis points.
    struct Point has copy, drop, store {
        x: u64,
        y: u64
    }

    /// P(x) = min_price + (base ^ (utilization / target_utilization) - 1) / (base - 1) * (max_price - min_price)
    // initialize the base to be 32, which means:
    //   When DB is at 50% target utilization,the price increases roughly 15% of (max_price - min_price) on top of min_price.
    //   More data points:
    //     10% -> 1%
    //     20% -> 3%
    //     30% -> 6%
    //     40% -> 10%
    //     50% -> 15%
    //     60% -> 23%
    //     70% -> 33%
    //     80% -> 48%
    //     90% -> 70%
    //     95% -> 84%
    //     99% -> 96%
    //    100% -> 100%
    struct UsageGasConfig has copy, drop, store {
        min_gas: u64,
        max_gas: u64,
        target_utilization: u64,
        read_curve: vector<Point>,
        create_curve: vector<Point>,
        write_curve: vector<Point>,
    }

    struct StorageGasConfig has copy, drop, key {
        item_config: UsageGasConfig,
        byte_config: UsageGasConfig,
    }

    public fun new_curve_point(x: u64, y: u64): Point {
        Point {x, y}
    }

    public fun new_usage_gas_config(min_gas: u64, max_gas: u64, target_utilization: u64, read_curve: vector<Point>, create_curve: vector<Point>, write_curve: vector<Point>): UsageGasConfig {
        UsageGasConfig {
            min_gas,
            max_gas,
            target_utilization,
            read_curve,
            create_curve,
            write_curve,
        }
    }

    public fun new_storage_gas_config(item_config: UsageGasConfig, byte_config: UsageGasConfig): StorageGasConfig {
        StorageGasConfig {
            item_config,
            byte_config
        }
    }

    public(friend) fun set_config(aptos_framework: &signer, config: StorageGasConfig) acquires StorageGasConfig {
        system_addresses::assert_aptos_framework(aptos_framework);
        validate_usage_config(&config.item_config);
        validate_usage_config(&config.item_config);
        *borrow_global_mut<StorageGasConfig>(@aptos_framework) = config;
    }

    public(friend) fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(
            !exists<StorageGasConfig>(@aptos_framework),
            error::already_exists(ESTORAGE_GAS_CONFIG)
        );
        let standard_curve: vector<Point> = vector[
        Point {x: 0, y: 0},
        Point {x: 1000, y: 100},
        Point {x: 2000, y: 300},
        Point {x: 3000, y: 600},
        Point {x: 4000, y: 1000},
        Point {x: 5000, y: 1500},
        Point {x: 6000, y: 2300},
        Point {x: 7000, y: 3300},
        Point {x: 8000, y: 4800},
        Point {x: 9000, y: 7000},
        Point {x: 9500, y: 8400},
        Point {x: 9900, y: 9600},
        Point {x: BASIS_POINT_BASE, y: BASIS_POINT_BASE},
        ];

        let item_config = UsageGasConfig {
            min_gas: 100,
            max_gas: 100000,
            target_utilization: 1000000000,  // 1 billion
            read_curve: copy standard_curve,
            create_curve: copy standard_curve,
            write_curve: copy standard_curve,
        };
        let byte_config = UsageGasConfig {
            min_gas: 1,
            max_gas: 1000,
            target_utilization: 250000000000, // 250 GB
            read_curve: copy standard_curve,
            create_curve: copy standard_curve,
            write_curve: copy standard_curve,
        };
        validate_usage_config(&item_config);
        validate_usage_config(&item_config);
        move_to(aptos_framework, StorageGasConfig {
            item_config,
            byte_config,
        });

        assert!(
            !exists<StorageGas>(@aptos_framework),
            error::already_exists(ESTORAGE_GAS)
        );
        move_to(aptos_framework, StorageGas {
            per_item_read: 0,
            per_item_create: 0,
            per_item_write: 0,
            per_byte_read: 0,
            per_byte_create: 0,
            per_byte_write: 0,
        });
    }

    fun validate_usage_config(config: &UsageGasConfig) {
        assert!(config.max_gas >= config.min_gas, error::invalid_argument(EINVALID_GAS_RANGE));
        assert!(config.target_utilization > 0, error::invalid_argument(EZERO_TARGET_UTILIZATION));
        validate_curve(&config.read_curve);
        validate_curve(&config.create_curve);
        validate_curve(&config.write_curve);
    }

    fun validate_curve(curve: &vector<Point>) {
        let len = vector::length(curve);
        assert!(len >= 2, error::invalid_argument(EINVALID_CURVE_LENGTH));
        assert!(vector::borrow(curve, 0).x == 0, error::invalid_argument(EINVALID_CURVE_START_POINT));
        assert!(vector::borrow(curve, len - 1).x == BASIS_POINT_BASE, error::invalid_argument(EINVALID_CURVE_END_POINT));
        let i = 0;
        while (i < len - 1) {
            let cur = vector::borrow(curve, i);
            let next = vector::borrow(curve, i + 1);
            assert!(cur.x < next.x && cur.y <= next.y, error::invalid_argument(EINVALID_MONOTONICALLY_NON_DECREASING_CURVE));
            i = i + 1;
        }
    }

    fun calculate_gas(max_usage: u64, current_usage: u64, curve: &vector<Point>, min_gas:u64, max_gas:u64): u64 {
        let capped_current_usage = if (current_usage > max_usage) max_usage else current_usage;
        let num_points = vector::length(curve);
        let current_usage_bps = capped_current_usage * BASIS_POINT_BASE / max_usage;
        let (i, j) = (0, num_points - 1);
        while (i < j) {
            let mid = j - (j - i) / 2;
            if (current_usage_bps < vector::borrow(curve, mid).x) {
                j = mid - 1;
            } else {
                i = mid;
            };
        };
        let start = vector::borrow(curve, i);
        if (i == num_points - 1) {
            min_gas + (max_gas - min_gas) * start.y / BASIS_POINT_BASE
        } else {
            let end = vector::borrow(curve, i + 1);
            min_gas + (max_gas - min_gas) * (start.y + (current_usage_bps - start.x) * (end.y - start.y) / (end.x - start.x)) / BASIS_POINT_BASE
        }
    }

    fun calculate_read_gas(config: &UsageGasConfig, usage: u64): u64 {
        calculate_gas(config.target_utilization, usage, &config.read_curve, config.min_gas, config.max_gas)
    }

    fun calculate_create_gas(config: &UsageGasConfig, usage: u64): u64 {
        calculate_gas(config.target_utilization, usage, &config.create_curve, config.min_gas, config.max_gas)
    }

    fun calculate_write_gas(config: &UsageGasConfig, usage: u64): u64 {
        calculate_gas(config.target_utilization, usage, &config.write_curve, config.min_gas, config.max_gas)
    }

    public(friend) fun on_reconfig() acquires StorageGas, StorageGasConfig {
        assert!(
            exists<StorageGasConfig>(@aptos_framework),
            error::not_found(ESTORAGE_GAS_CONFIG)
        );
        assert!(
            exists<StorageGas>(@aptos_framework),
            error::not_found(ESTORAGE_GAS)
        );
        let (items, bytes) = state_storage::current_items_and_bytes();
        let gas_config= borrow_global<StorageGasConfig>(@aptos_framework);
        let gas= borrow_global_mut<StorageGas>(@aptos_framework);
        gas.per_item_read = calculate_read_gas(&gas_config.item_config, items);
        gas.per_item_create = calculate_create_gas(&gas_config.item_config, items);
        gas.per_item_write = calculate_write_gas(&gas_config.item_config, items);
        gas.per_byte_read = calculate_read_gas(&gas_config.byte_config, bytes);
        gas.per_byte_create = calculate_create_gas(&gas_config.byte_config, bytes);
        gas.per_byte_write = calculate_write_gas(&gas_config.byte_config, bytes);
    }

    #[test(framework = @aptos_framework)]
    fun test_initialize_and_reconfig(framework: signer) acquires StorageGas, StorageGasConfig {
        state_storage::initialize(&framework);
        initialize(&framework);
        on_reconfig();
        let gas_parameter = borrow_global<StorageGas>(@aptos_framework);
        assert!(gas_parameter.per_item_read == 100, 0);
        assert!(gas_parameter.per_item_create == 100, 0);
        assert!(gas_parameter.per_item_write == 100, 0);
        assert!(gas_parameter.per_byte_read == 1, 0);
        assert!(gas_parameter.per_byte_create == 1, 0);
        assert!(gas_parameter.per_byte_write == 1, 0);
    }
}
