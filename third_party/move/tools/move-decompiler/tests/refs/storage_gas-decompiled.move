module 0x1::storage_gas {
    struct GasCurve has copy, drop, store {
        min_gas: u64,
        max_gas: u64,
        points: vector<Point>,
    }
    
    struct Point has copy, drop, store {
        x: u64,
        y: u64,
    }
    
    struct StorageGas has key {
        per_item_read: u64,
        per_item_create: u64,
        per_item_write: u64,
        per_byte_read: u64,
        per_byte_create: u64,
        per_byte_write: u64,
    }
    
    struct StorageGasConfig has copy, drop, key {
        item_config: UsageGasConfig,
        byte_config: UsageGasConfig,
    }
    
    struct UsageGasConfig has copy, drop, store {
        target_usage: u64,
        read_curve: GasCurve,
        create_curve: GasCurve,
        write_curve: GasCurve,
    }
    
    public fun base_8192_exponential_curve(arg0: u64, arg1: u64) : GasCurve {
        let v0 = 0x1::vector::empty<Point>();
        let v1 = &mut v0;
        0x1::vector::push_back<Point>(v1, new_point(1000, 2));
        0x1::vector::push_back<Point>(v1, new_point(2000, 6));
        0x1::vector::push_back<Point>(v1, new_point(3000, 17));
        0x1::vector::push_back<Point>(v1, new_point(4000, 44));
        0x1::vector::push_back<Point>(v1, new_point(5000, 109));
        0x1::vector::push_back<Point>(v1, new_point(6000, 271));
        0x1::vector::push_back<Point>(v1, new_point(7000, 669));
        0x1::vector::push_back<Point>(v1, new_point(8000, 1648));
        0x1::vector::push_back<Point>(v1, new_point(9000, 4061));
        0x1::vector::push_back<Point>(v1, new_point(9500, 6372));
        0x1::vector::push_back<Point>(v1, new_point(9900, 9138));
        new_gas_curve(arg0, arg1, v0)
    }
    
    fun calculate_create_gas(arg0: &UsageGasConfig, arg1: u64) : u64 {
        calculate_gas(arg0.target_usage, arg1, &arg0.create_curve)
    }
    
    fun calculate_gas(arg0: u64, arg1: u64, arg2: &GasCurve) : u64 {
        let v0 = if (arg1 > arg0) {
            arg0
        } else {
            arg1
        };
        let v1 = &arg2.points;
        let v2 = 0x1::vector::length<Point>(v1);
        let v3 = v0 * 10000 / arg0;
        let (v4, v5) = if (v2 == 0) {
            let v6 = Point{
                x : 0, 
                y : 0,
            };
            let v7 = Point{
                x : 10000, 
                y : 10000,
            };
            (&v6, &v7)
        } else {
            let (v8, v9) = if (v3 < 0x1::vector::borrow<Point>(v1, 0).x) {
                let v10 = Point{
                    x : 0, 
                    y : 0,
                };
                (&v10, 0x1::vector::borrow<Point>(v1, 0))
            } else {
                let (v11, v12) = if (0x1::vector::borrow<Point>(v1, v2 - 1).x <= v3) {
                    let v13 = Point{
                        x : 10000, 
                        y : 10000,
                    };
                    (0x1::vector::borrow<Point>(v1, v2 - 1), &v13)
                } else {
                    let v14 = v2 - 2;
                    let v15 = 0;
                    while (v15 < v14) {
                        let v16 = v14 - (v14 - v15) / 2;
                        if (v3 < 0x1::vector::borrow<Point>(v1, v16).x) {
                            v14 = v16 - 1;
                            continue
                        };
                        v15 = v16;
                    };
                    (0x1::vector::borrow<Point>(v1, v15), 0x1::vector::borrow<Point>(v1, v15 + 1))
                };
                (v11, v12)
            };
            (v8, v9)
        };
        interpolate(0, 10000, arg2.min_gas, arg2.max_gas, interpolate(v4.x, v5.x, v4.y, v5.y, v3))
    }
    
    fun calculate_read_gas(arg0: &UsageGasConfig, arg1: u64) : u64 {
        calculate_gas(arg0.target_usage, arg1, &arg0.read_curve)
    }
    
    fun calculate_write_gas(arg0: &UsageGasConfig, arg1: u64) : u64 {
        calculate_gas(arg0.target_usage, arg1, &arg0.write_curve)
    }
    
    public fun initialize(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        assert!(!exists<StorageGasConfig>(@0x1), 0x1::error::already_exists(0));
        let v0 = 1000;
        let v1 = 1000000;
        let v2 = base_8192_exponential_curve(300 * v0, 300 * v0 * 100);
        let v3 = base_8192_exponential_curve(300 * v0, 300 * v0 * 100);
        let v4 = UsageGasConfig{
            target_usage : 2 * v0 * v1, 
            read_curve   : v2, 
            create_curve : v3, 
            write_curve  : base_8192_exponential_curve(300 * v0, 300 * v0 * 100),
        };
        let v5 = base_8192_exponential_curve(5 * v0, 5 * v0 * 100);
        let v6 = base_8192_exponential_curve(5 * v0, 5 * v0 * 100);
        let v7 = UsageGasConfig{
            target_usage : 1 * v1 * v1, 
            read_curve   : base_8192_exponential_curve(300, 30000), 
            create_curve : v5, 
            write_curve  : v6,
        };
        let v8 = StorageGasConfig{
            item_config : v4, 
            byte_config : v7,
        };
        move_to<StorageGasConfig>(arg0, v8);
        assert!(!exists<StorageGas>(@0x1), 0x1::error::already_exists(1));
        let v9 = 300 * v0;
        let v10 = 300 * v0;
        let v11 = StorageGas{
            per_item_read   : v9, 
            per_item_create : 5 * v1, 
            per_item_write  : v10, 
            per_byte_read   : 300, 
            per_byte_create : 5 * v0, 
            per_byte_write  : 5 * v0,
        };
        move_to<StorageGas>(arg0, v11);
    }
    
    fun interpolate(arg0: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64) : u64 {
        arg2 + (arg4 - arg0) * (arg3 - arg2) / (arg1 - arg0)
    }
    
    public fun new_gas_curve(arg0: u64, arg1: u64, arg2: vector<Point>) : GasCurve {
        assert!(arg1 >= arg0, 0x1::error::invalid_argument(2));
        assert!(arg1 <= 18446744073709551615 / 10000, 0x1::error::invalid_argument(2));
        validate_points(&arg2);
        GasCurve{
            min_gas : arg0, 
            max_gas : arg1, 
            points  : arg2,
        }
    }
    
    public fun new_point(arg0: u64, arg1: u64) : Point {
        assert!(arg0 <= 10000 && arg1 <= 10000, 0x1::error::invalid_argument(6));
        Point{
            x : arg0, 
            y : arg1,
        }
    }
    
    public fun new_storage_gas_config(arg0: UsageGasConfig, arg1: UsageGasConfig) : StorageGasConfig {
        StorageGasConfig{
            item_config : arg0, 
            byte_config : arg1,
        }
    }
    
    public fun new_usage_gas_config(arg0: u64, arg1: GasCurve, arg2: GasCurve, arg3: GasCurve) : UsageGasConfig {
        assert!(arg0 > 0, 0x1::error::invalid_argument(3));
        assert!(arg0 <= 18446744073709551615 / 10000, 0x1::error::invalid_argument(4));
        UsageGasConfig{
            target_usage : arg0, 
            read_curve   : arg1, 
            create_curve : arg2, 
            write_curve  : arg3,
        }
    }
    
    public(friend) fun on_reconfig() acquires StorageGas, StorageGasConfig {
        assert!(exists<StorageGasConfig>(@0x1), 0x1::error::not_found(0));
        assert!(exists<StorageGas>(@0x1), 0x1::error::not_found(1));
        let (v0, v1) = 0x1::state_storage::current_items_and_bytes();
        let v2 = borrow_global<StorageGasConfig>(@0x1);
        let v3 = borrow_global_mut<StorageGas>(@0x1);
        v3.per_item_read = calculate_read_gas(&v2.item_config, v0);
        v3.per_item_create = calculate_create_gas(&v2.item_config, v0);
        v3.per_item_write = calculate_write_gas(&v2.item_config, v0);
        v3.per_byte_read = calculate_read_gas(&v2.byte_config, v1);
        v3.per_byte_create = calculate_create_gas(&v2.byte_config, v1);
        v3.per_byte_write = calculate_write_gas(&v2.byte_config, v1);
    }
    
    public(friend) fun set_config(arg0: &signer, arg1: StorageGasConfig) acquires StorageGasConfig {
        0x1::system_addresses::assert_aptos_framework(arg0);
        *borrow_global_mut<StorageGasConfig>(@0x1) = arg1;
    }
    
    fun validate_points(arg0: &vector<Point>) {
        let v0 = 0x1::vector::length<Point>(arg0);
        let v1 = 0;
        while (v1 <= v0) {
            let v2 = if (v1 == 0) {
                let v3 = Point{
                    x : 0, 
                    y : 0,
                };
                &v3
            } else {
                0x1::vector::borrow<Point>(arg0, v1 - 1)
            };
            let v4 = if (v1 == v0) {
                let v5 = Point{
                    x : 10000, 
                    y : 10000,
                };
                &v5
            } else {
                0x1::vector::borrow<Point>(arg0, v1)
            };
            assert!(v2.x < v4.x && v2.y <= v4.y, 0x1::error::invalid_argument(5));
            v1 = v1 + 1;
        };
    }
    
    // decompiled from Move bytecode v6
}
