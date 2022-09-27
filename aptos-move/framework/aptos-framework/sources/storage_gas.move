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
    const EZERO_TARGET_USAGE: u64 = 3;
    const ETARGET_USAGE_TOO_BIG: u64 = 4;
    const EINVALID_MONOTONICALLY_NON_DECREASING_CURVE: u64 = 5;
    const EINVALID_POINT_RANGE: u64 = 6;

    const BASIS_POINT_DENOMINATION: u64 = 10000;

    const MAX_U64: u64 = 18446744073709551615;

    /// This updates at reconfig and guarantees not to change elsewhere, safe
    /// for gas calculation.
    ///
    /// Specifically, it is updated by executing a reconfig transaction based
    /// on the usage at the begining of the current epoch. The gas schedule
    /// derived from these parameter will be for gas calculation of the entire
    /// next epoch.
    /// -- The data is one epoch older than ideal, but VM doesn't need to worry
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

    struct UsageGasConfig has copy, drop, store {
        target_usage: u64,
        read_curve: GasCurve,
        create_curve: GasCurve,
        write_curve: GasCurve,
    }

    /// The curve assumes there are two points (0, 0) and (10000, 10000) on both ends. Moreover, points must also
    /// satisfy the following rules:
    /// 1. the x values must be strictly increasing and between (0, 10000);
    /// 2. the y values must be non-decreasing and between (0, 10000);
    /// So the curve will be identified as point (0, 0) and (10000, 10000) interpolated with the points. The y value
    /// between two points will be calculated by neighboring points as if there is a linear line connecting these two
    /// points.
    struct GasCurve has copy, drop, store {
        min_gas: u64,
        max_gas: u64,
        points: vector<Point>,
    }

    /// P(x) = min_price + (base ^ (utilization / target_usage) - 1) / (base - 1) * (max_price - min_price)
    // Provide a default exponential curve with the base to be 8192, which means:
    // When DB is at 50% target usage,the price increases roughly 1.09% of (max_price - min_price).
    // Detailed data points:
    // 10% -> 0.02%
    // 20% -> 0.06%
    // 30% -> 0.17%
    // 40% -> 0.44%
    // 50% -> 1.09%
    // 60% -> 2.71%
    // 70% -> 6.69%
    // 80% -> 16.48%
    // 90% -> 40.61%
    // 95% -> 63.72%
    // 99% -> 91.38%
    public fun base_8192_exponential_curve(min_gas: u64, max_gas: u64): GasCurve {
        new_gas_curve(min_gas, max_gas,
            vector[
                new_point(1000, 2),
                new_point(2000, 6),
                new_point(3000, 17),
                new_point(4000, 44),
                new_point(5000, 109),
                new_point(6000, 271),
                new_point(7000, 669),
                new_point(8000, 1648),
                new_point(9000, 4061),
                new_point(9500, 6372),
                new_point(9900, 9138),
            ]
        )
    }

    struct StorageGasConfig has copy, drop, key {
        item_config: UsageGasConfig,
        byte_config: UsageGasConfig,
    }

    public fun new_point(x: u64, y: u64): Point {
        assert!(
            x <= BASIS_POINT_DENOMINATION && y <= BASIS_POINT_DENOMINATION,
            error::invalid_argument(EINVALID_POINT_RANGE)
        );
        Point { x, y }
    }

    public fun new_gas_curve(min_gas: u64, max_gas: u64, points: vector<Point>): GasCurve {
        assert!(max_gas >= min_gas, error::invalid_argument(EINVALID_GAS_RANGE));
        assert!(max_gas <= MAX_U64 / BASIS_POINT_DENOMINATION, error::invalid_argument(EINVALID_GAS_RANGE));
        validate_points(&points);
        GasCurve {
            min_gas,
            max_gas,
            points
        }
    }

    public fun new_usage_gas_config(target_usage: u64, read_curve: GasCurve, create_curve: GasCurve, write_curve: GasCurve): UsageGasConfig {
        assert!(target_usage > 0, error::invalid_argument(EZERO_TARGET_USAGE));
        assert!(target_usage <= MAX_U64 / BASIS_POINT_DENOMINATION, error::invalid_argument(ETARGET_USAGE_TOO_BIG));
        UsageGasConfig {
            target_usage,
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
        *borrow_global_mut<StorageGasConfig>(@aptos_framework) = config;
    }

    public fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(
            !exists<StorageGasConfig>(@aptos_framework),
            error::already_exists(ESTORAGE_GAS_CONFIG)
        );

        let item_config = UsageGasConfig {
            target_usage: 1000000000, // 1 billion
            read_curve: base_8192_exponential_curve(80000, 80000 * 100),
            create_curve: base_8192_exponential_curve(2000000, 2000000 * 100),
            write_curve: base_8192_exponential_curve(400000, 400000 * 100),
        };
        let byte_config = UsageGasConfig {
            target_usage: 500000000000, // 500 GB
            read_curve: base_8192_exponential_curve(40, 40 * 100),
            create_curve: base_8192_exponential_curve(1000, 1000 * 100),
            write_curve: base_8192_exponential_curve(200, 200 * 100),
        };
        move_to(aptos_framework, StorageGasConfig {
            item_config,
            byte_config,
        });

        assert!(
            !exists<StorageGas>(@aptos_framework),
            error::already_exists(ESTORAGE_GAS)
        );
        move_to(aptos_framework, StorageGas {
            per_item_read: 8000,
            per_item_create: 1280000,
            per_item_write: 160000,
            per_byte_read: 1000,
            per_byte_create: 10000,
            per_byte_write: 10000,
        });
    }

    fun validate_points(points: &vector<Point>) {
        let len = vector::length(points);
        spec {
            assume len < MAX_U64;
        };
        let i = 0;
        while ({
            spec {
                invariant forall j in 0..i: {
                    let cur = if (j == 0) { Point { x: 0, y: 0 } } else { points[j - 1] };
                    let next = if (j == len) { Point { x: BASIS_POINT_DENOMINATION, y: BASIS_POINT_DENOMINATION } } else { points[j] };
                    cur.x < next.x && cur.y <= next.y
                };
            };
            i <= len
        }) {
            let cur = if (i == 0) { &Point { x: 0, y: 0 } } else { vector::borrow(points, i - 1) };
            let next = if (i == len) { &Point { x: BASIS_POINT_DENOMINATION, y: BASIS_POINT_DENOMINATION } } else { vector::borrow(points, i) };
            assert!(cur.x < next.x && cur.y <= next.y, error::invalid_argument(EINVALID_MONOTONICALLY_NON_DECREASING_CURVE));
            i = i + 1;
        }
    }

    fun calculate_gas(max_usage: u64, current_usage: u64, curve: &GasCurve): u64 {
        let capped_current_usage = if (current_usage > max_usage) max_usage else current_usage;
        let points = &curve.points;
        let num_points = vector::length(points);
        let current_usage_bps = capped_current_usage * BASIS_POINT_DENOMINATION / max_usage;

        // Check the corner case that current_usage_bps drops before the first point.
        let (left, right) = if (num_points == 0) {
            (&Point { x: 0, y: 0 }, &Point { x: BASIS_POINT_DENOMINATION, y: BASIS_POINT_DENOMINATION })
        } else if (current_usage_bps < vector::borrow(points, 0).x) {
            (&Point { x: 0, y: 0 }, vector::borrow(points, 0))
        } else if (vector::borrow(points, num_points - 1).x <= current_usage_bps) {
            (vector::borrow(points, num_points - 1), &Point { x: BASIS_POINT_DENOMINATION, y: BASIS_POINT_DENOMINATION })
        } else {
            let (i, j) = (0, num_points - 2);
            while ({
                spec {
                    invariant i <= j;
                    invariant j < num_points - 1;
                    invariant points[i].x <= current_usage_bps;
                    invariant current_usage_bps < points[j + 1].x;
                };
                i < j
            }) {
                let mid = j - (j - i) / 2;
                if (current_usage_bps < vector::borrow(points, mid).x) {
                    spec {
                        // j is strictly decreasing.
                        assert mid - 1 < j;
                    };
                    j = mid - 1;
                } else {
                    spec {
                        // i is strictly increasing.
                        assert i < mid;
                    };
                    i = mid;
                };
            };
            (vector::borrow(points, i), vector::borrow(points, i + 1))
        };
        let y_interpolated = interpolate(left.x, right.x, left.y, right.y, current_usage_bps);
        interpolate(0, BASIS_POINT_DENOMINATION, curve.min_gas, curve.max_gas, y_interpolated)
    }

    // Interpolates y for x on the line between (x0, y0) and (x1, y1).
    fun interpolate(x0: u64, x1: u64, y0: u64, y1: u64, x: u64): u64 {
        y0 + (x - x0) * (y1 - y0) / (x1 - x0)
    }

    fun calculate_read_gas(config: &UsageGasConfig, usage: u64): u64 {
        calculate_gas(config.target_usage, usage, &config.read_curve)
    }

    fun calculate_create_gas(config: &UsageGasConfig, usage: u64): u64 {
        calculate_gas(config.target_usage, usage, &config.create_curve)
    }

    fun calculate_write_gas(config: &UsageGasConfig, usage: u64): u64 {
        calculate_gas(config.target_usage, usage, &config.write_curve)
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
        let gas_config = borrow_global<StorageGasConfig>(@aptos_framework);
        let gas = borrow_global_mut<StorageGas>(@aptos_framework);
        gas.per_item_read = calculate_read_gas(&gas_config.item_config, items);
        gas.per_item_create = calculate_create_gas(&gas_config.item_config, items);
        gas.per_item_write = calculate_write_gas(&gas_config.item_config, items);
        gas.per_byte_read = calculate_read_gas(&gas_config.byte_config, bytes);
        gas.per_byte_create = calculate_create_gas(&gas_config.byte_config, bytes);
        gas.per_byte_write = calculate_write_gas(&gas_config.byte_config, bytes);
    }

    // TODO: reactivate this test after fixing assertions
    //#[test(framework = @aptos_framework)]
    #[test_only]
    fun test_initialize_and_reconfig(framework: signer) acquires StorageGas, StorageGasConfig {
        state_storage::initialize(&framework);
        initialize(&framework);
        on_reconfig();
        let gas_parameter = borrow_global<StorageGas>(@aptos_framework);
        assert!(gas_parameter.per_item_read == 10, 0);
        assert!(gas_parameter.per_item_create == 10, 0);
        assert!(gas_parameter.per_item_write == 10, 0);
        assert!(gas_parameter.per_byte_read == 1, 0);
        assert!(gas_parameter.per_byte_create == 1, 0);
        assert!(gas_parameter.per_byte_write == 1, 0);
    }

    #[test]
    fun test_curve() {
        let constant_curve = new_gas_curve(5, 5, vector[]);
        let linear_curve = new_gas_curve(1, 1000, vector[]);
        let standard_curve = base_8192_exponential_curve(1, 1000);
        let target = BASIS_POINT_DENOMINATION / 2;
        while (target < 2 * BASIS_POINT_DENOMINATION) {
            let i = 0;
            let old_standard_curve_gas = 1;
            while (i <= target + 7) {
                assert!(calculate_gas(target, i, &constant_curve) == 5, 0);
                assert!(calculate_gas(target, i, &linear_curve) == (if (i < target) { 1 + 999 * (i * BASIS_POINT_DENOMINATION / target) / BASIS_POINT_DENOMINATION } else { 1000 }), 0);
                let new_standard_curve_gas = calculate_gas(target, i, &standard_curve);
                assert!(new_standard_curve_gas >= old_standard_curve_gas, 0);
                old_standard_curve_gas = new_standard_curve_gas;
                i = i + 3;
            };
            assert!(old_standard_curve_gas == 1000, 0);
            target = target + BASIS_POINT_DENOMINATION;
        }
    }

    #[test(framework = @aptos_framework)]
    fun test_set_storage_gas_config(framework: signer) acquires StorageGas, StorageGasConfig {
        state_storage::initialize(&framework);
        initialize(&framework);
        let item_curve = new_gas_curve(1000, 2000,
            vector[new_point(3000, 0), new_point(5000, 5000), new_point(8000, 5000)]
        );
        let byte_curve = new_gas_curve(0, 1000, vector::singleton<Point>(new_point(5000, 3000)));
        let item_usage_config = new_usage_gas_config(100, copy item_curve, copy item_curve, copy item_curve);
        let byte_usage_config = new_usage_gas_config(2000, copy byte_curve, copy byte_curve, copy byte_curve);
        let storage_gas_config = new_storage_gas_config(item_usage_config, byte_usage_config);
        set_config(&framework, storage_gas_config);
        {
            state_storage::set_for_test(0, 20, 100);
            on_reconfig();
            let gas_parameter = borrow_global<StorageGas>(@aptos_framework);
            assert!(gas_parameter.per_item_read == 1000, 0);
            assert!(gas_parameter.per_byte_read == 30, 0);
        };
        {
            state_storage::set_for_test(0, 40, 800);
            on_reconfig();
            let gas_parameter = borrow_global<StorageGas>(@aptos_framework);
            assert!(gas_parameter.per_item_create == 1250, 0);
            assert!(gas_parameter.per_byte_create == 240, 0);
        };
        {
            state_storage::set_for_test(0, 60, 1200);
            on_reconfig();
            let gas_parameter = borrow_global<StorageGas>(@aptos_framework);
            assert!(gas_parameter.per_item_write == 1500, 0);
            assert!(gas_parameter.per_byte_write == 440, 0);
        };
        {
            state_storage::set_for_test(0, 90, 1800);
            on_reconfig();
            let gas_parameter = borrow_global<StorageGas>(@aptos_framework);
            assert!(gas_parameter.per_item_create == 1750, 0);
            assert!(gas_parameter.per_byte_create == 860, 0);
        };
        {
            // usage overflow case
            state_storage::set_for_test(0, 110, 2200);
            on_reconfig();
            let gas_parameter = borrow_global<StorageGas>(@aptos_framework);
            assert!(gas_parameter.per_item_read == 2000, 0);
            assert!(gas_parameter.per_byte_read == 1000, 0);
        };
    }
}
