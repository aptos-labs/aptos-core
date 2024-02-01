module 0x1::smart_table {
    struct Entry<T0, T1> has copy, drop, store {
        hash: u64,
        key: T0,
        value: T1,
    }
    
    struct SmartTable<T0, T1> has store {
        buckets: 0x1::table_with_length::TableWithLength<u64, vector<Entry<T0, T1>>>,
        num_buckets: u64,
        level: u8,
        size: u64,
        split_load_threshold: u8,
        target_bucket_size: u64,
    }
    
    public fun contains<T0: drop, T1>(arg0: &SmartTable<T0, T1>, arg1: T0) : bool {
        let v0 = 0x1::aptos_hash::sip_hash_from_value<T0>(&arg1);
        let v1 = bucket_index(arg0.level, arg0.num_buckets, v0);
        let v2 = 0x1::table_with_length::borrow<u64, vector<Entry<T0, T1>>>(&arg0.buckets, v1);
        let v3 = false;
        let v4 = 0;
        while (v4 < 0x1::vector::length<Entry<T0, T1>>(v2)) {
            let v5 = 0x1::vector::borrow<Entry<T0, T1>>(v2, v4);
            let v6 = v5.hash == v0 && &v5.key == &arg1;
            v3 = v6;
            if (v6) {
                break
            };
            v4 = v4 + 1;
        };
        v3
    }
    
    public fun add_all<T0, T1>(arg0: &mut SmartTable<T0, T1>, arg1: vector<T0>, arg2: vector<T1>) {
        let v0 = arg2;
        let v1 = arg1;
        0x1::vector::reverse<T0>(&mut v1);
        0x1::vector::reverse<T1>(&mut v0);
        let v2 = v0;
        let v3 = v1;
        let v4 = 0x1::vector::length<T0>(&v3);
        let v5 = v4;
        assert!(v4 == 0x1::vector::length<T1>(&v2), 131074);
        while (v5 > 0) {
            add<T0, T1>(arg0, 0x1::vector::pop_back<T0>(&mut v3), 0x1::vector::pop_back<T1>(&mut v2));
            v5 = v5 - 1;
        };
        0x1::vector::destroy_empty<T0>(v3);
        0x1::vector::destroy_empty<T1>(v2);
    }
    
    public fun new<T0: copy + drop + store, T1: store>() : SmartTable<T0, T1> {
        new_with_config<T0, T1>(0, 0, 0)
    }
    
    public fun add<T0, T1>(arg0: &mut SmartTable<T0, T1>, arg1: T0, arg2: T1) {
        let v0 = 0x1::aptos_hash::sip_hash_from_value<T0>(&arg1);
        let v1 = bucket_index(arg0.level, arg0.num_buckets, v0);
        let v2 = 0x1::table_with_length::borrow_mut<u64, vector<Entry<T0, T1>>>(&mut arg0.buckets, v1);
        assert!(0x1::vector::length<Entry<T0, T1>>(v2) <= 10000, 0x1::error::permission_denied(7));
        let v3 = v2;
        let v4 = true;
        let v5 = 0;
        while (v5 < 0x1::vector::length<Entry<T0, T1>>(v3)) {
            let v6 = &0x1::vector::borrow<Entry<T0, T1>>(v3, v5).key != &arg1;
            v4 = v6;
            if (!v6) {
                break
            };
            v5 = v5 + 1;
        };
        assert!(v4, 0x1::error::invalid_argument(4));
        let v7 = Entry<T0, T1>{
            hash  : v0, 
            key   : arg1, 
            value : arg2,
        };
        if (arg0.target_bucket_size == 0) {
            let v8 = 0x1::math64::max(1024 / 0x1::math64::max(0x1::type_info::size_of_val<Entry<T0, T1>>(&v7), 1), 1);
            arg0.target_bucket_size = v8;
        };
        0x1::vector::push_back<Entry<T0, T1>>(v2, v7);
        arg0.size = arg0.size + 1;
        if (load_factor<T0, T1>(arg0) >= (arg0.split_load_threshold as u64)) {
            split_one_bucket<T0, T1>(arg0);
        };
    }
    
    public fun borrow<T0: drop, T1>(arg0: &SmartTable<T0, T1>, arg1: T0) : &T1 {
        let v0 = bucket_index(arg0.level, arg0.num_buckets, 0x1::aptos_hash::sip_hash_from_value<T0>(&arg1));
        let v1 = 0x1::table_with_length::borrow<u64, vector<Entry<T0, T1>>>(&arg0.buckets, v0);
        let v2 = 0;
        while (v2 < 0x1::vector::length<Entry<T0, T1>>(v1)) {
            let v3 = 0x1::vector::borrow<Entry<T0, T1>>(v1, v2);
            if (&v3.key == &arg1) {
                return &v3.value
            };
            v2 = v2 + 1;
        };
        abort 0x1::error::invalid_argument(1)
    }
    
    public fun borrow_mut<T0: drop, T1>(arg0: &mut SmartTable<T0, T1>, arg1: T0) : &mut T1 {
        let v0 = bucket_index(arg0.level, arg0.num_buckets, 0x1::aptos_hash::sip_hash_from_value<T0>(&arg1));
        let v1 = 0x1::table_with_length::borrow_mut<u64, vector<Entry<T0, T1>>>(&mut arg0.buckets, v0);
        let v2 = 0;
        while (v2 < 0x1::vector::length<Entry<T0, T1>>(v1)) {
            let v3 = 0x1::vector::borrow_mut<Entry<T0, T1>>(v1, v2);
            if (&v3.key == &arg1) {
                return &mut v3.value
            };
            v2 = v2 + 1;
        };
        abort 0x1::error::invalid_argument(1)
    }
    
    public fun destroy_empty<T0, T1>(arg0: SmartTable<T0, T1>) {
        assert!(arg0.size == 0, 0x1::error::invalid_argument(3));
        let v0 = 0;
        while (v0 < arg0.num_buckets) {
            let v1 = 0x1::table_with_length::remove<u64, vector<Entry<T0, T1>>>(&mut arg0.buckets, v0);
            0x1::vector::destroy_empty<Entry<T0, T1>>(v1);
            v0 = v0 + 1;
        };
        let SmartTable {
            buckets              : v2,
            num_buckets          : _,
            level                : _,
            size                 : _,
            split_load_threshold : _,
            target_bucket_size   : _,
        } = arg0;
        0x1::table_with_length::destroy_empty<u64, vector<Entry<T0, T1>>>(v2);
    }
    
    public fun remove<T0: copy + drop, T1>(arg0: &mut SmartTable<T0, T1>, arg1: T0) : T1 {
        let v0 = bucket_index(arg0.level, arg0.num_buckets, 0x1::aptos_hash::sip_hash_from_value<T0>(&arg1));
        let v1 = 0x1::table_with_length::borrow_mut<u64, vector<Entry<T0, T1>>>(&mut arg0.buckets, v0);
        let v2 = 0;
        while (v2 < 0x1::vector::length<Entry<T0, T1>>(v1)) {
            if (&0x1::vector::borrow<Entry<T0, T1>>(v1, v2).key == &arg1) {
                let Entry {
                    hash  : _,
                    key   : _,
                    value : v5,
                } = 0x1::vector::swap_remove<Entry<T0, T1>>(v1, v2);
                arg0.size = arg0.size - 1;
                return v5
            };
            v2 = v2 + 1;
        };
        abort 0x1::error::invalid_argument(1)
    }
    
    public fun length<T0, T1>(arg0: &SmartTable<T0, T1>) : u64 {
        arg0.size
    }
    
    public fun borrow_buckets<T0, T1>(arg0: &SmartTable<T0, T1>) : &0x1::table_with_length::TableWithLength<u64, vector<Entry<T0, T1>>> {
        &arg0.buckets
    }
    
    public fun borrow_buckets_mut<T0, T1>(arg0: &mut SmartTable<T0, T1>) : &mut 0x1::table_with_length::TableWithLength<u64, vector<Entry<T0, T1>>> {
        &mut arg0.buckets
    }
    
    public fun borrow_kv<T0, T1>(arg0: &Entry<T0, T1>) : (&T0, &T1) {
        (&arg0.key, &arg0.value)
    }
    
    public fun borrow_kv_mut<T0, T1>(arg0: &mut Entry<T0, T1>) : (&mut T0, &mut T1) {
        (&mut arg0.key, &mut arg0.value)
    }
    
    public fun borrow_mut_with_default<T0: copy + drop, T1: drop>(arg0: &mut SmartTable<T0, T1>, arg1: T0, arg2: T1) : &mut T1 {
        if (!contains<T0, T1>(arg0, arg1)) {
            add<T0, T1>(arg0, arg1, arg2);
        };
        borrow_mut<T0, T1>(arg0, arg1)
    }
    
    public fun borrow_with_default<T0: copy + drop, T1>(arg0: &SmartTable<T0, T1>, arg1: T0, arg2: &T1) : &T1 {
        if (!contains<T0, T1>(arg0, arg1)) {
            arg2
        } else {
            borrow<T0, T1>(arg0, arg1)
        }
    }
    
    fun bucket_index(arg0: u8, arg1: u64, arg2: u64) : u64 {
        let v0 = arg2 % (1 << arg0 + 1);
        if (v0 < arg1) {
            v0
        } else {
            v0 % (1 << arg0)
        }
    }
    
    public fun clear<T0: drop, T1: drop>(arg0: &mut SmartTable<T0, T1>) {
        let v0 = 0x1::table_with_length::borrow_mut<u64, vector<Entry<T0, T1>>>(&mut arg0.buckets, 0);
        *v0 = 0x1::vector::empty<Entry<T0, T1>>();
        let v1 = 1;
        while (v1 < arg0.num_buckets) {
            0x1::table_with_length::remove<u64, vector<Entry<T0, T1>>>(&mut arg0.buckets, v1);
            v1 = v1 + 1;
        };
        arg0.num_buckets = 1;
        arg0.level = 0;
        arg0.size = 0;
    }
    
    public fun destroy<T0: drop, T1: drop>(arg0: SmartTable<T0, T1>) {
        clear<T0, T1>(&mut arg0);
        destroy_empty<T0, T1>(arg0);
    }
    
    public fun load_factor<T0, T1>(arg0: &SmartTable<T0, T1>) : u64 {
        arg0.size * 100 / arg0.num_buckets / arg0.target_bucket_size
    }
    
    public fun new_with_config<T0: copy + drop + store, T1: store>(arg0: u64, arg1: u8, arg2: u64) : SmartTable<T0, T1> {
        assert!(arg1 <= 100, 0x1::error::invalid_argument(5));
        let v0 = 0x1::table_with_length::new<u64, vector<Entry<T0, T1>>>();
        0x1::table_with_length::add<u64, vector<Entry<T0, T1>>>(&mut v0, 0, 0x1::vector::empty<Entry<T0, T1>>());
        let v1 = if (arg1 == 0) {
            75
        } else {
            arg1
        };
        let v2 = SmartTable<T0, T1>{
            buckets              : v0, 
            num_buckets          : 1, 
            level                : 0, 
            size                 : 0, 
            split_load_threshold : v1, 
            target_bucket_size   : arg2,
        };
        if (arg0 == 0) {
            arg0 = 2;
        };
        while (arg0 > 1) {
            arg0 = arg0 - 1;
            split_one_bucket<T0, T1>(&mut v2);
        };
        v2
    }
    
    public fun num_buckets<T0, T1>(arg0: &SmartTable<T0, T1>) : u64 {
        arg0.num_buckets
    }
    
    fun split_one_bucket<T0, T1>(arg0: &mut SmartTable<T0, T1>) {
        let v0 = arg0.num_buckets;
        let v1 = v0 ^ 1 << arg0.level;
        arg0.num_buckets = v0 + 1;
        if (v1 + 1 == 1 << arg0.level) {
            arg0.level = arg0.level + 1;
        };
        let v2 = 0x1::table_with_length::borrow_mut<u64, vector<Entry<T0, T1>>>(&mut arg0.buckets, v1);
        let v3 = 0;
        let v4 = 0x1::vector::length<Entry<T0, T1>>(v2);
        while (v3 < v4) {
            if (!(bucket_index(arg0.level, arg0.num_buckets, 0x1::vector::borrow<Entry<T0, T1>>(v2, v3).hash) != v0)) {
                break
            };
            v3 = v3 + 1;
        };
        let v5 = v3;
        v3 = v3 + 1;
        while (v3 < v4) {
            if (bucket_index(arg0.level, arg0.num_buckets, 0x1::vector::borrow<Entry<T0, T1>>(v2, v3).hash) != v0) {
                0x1::vector::swap<Entry<T0, T1>>(v2, v5, v3);
                v5 = v5 + 1;
            };
            v3 = v3 + 1;
        };
        let v6 = 0x1::vector::trim_reverse<Entry<T0, T1>>(v2, v5);
        0x1::table_with_length::add<u64, vector<Entry<T0, T1>>>(&mut arg0.buckets, v0, v6);
    }
    
    public fun to_simple_map<T0: copy + drop + store, T1: copy + store>(arg0: &SmartTable<T0, T1>) : 0x1::simple_map::SimpleMap<T0, T1> {
        let v0 = 0;
        let v1 = 0x1::simple_map::new<T0, T1>();
        while (v0 < arg0.num_buckets) {
            let v2 = 0x1::table_with_length::borrow<u64, vector<Entry<T0, T1>>>(&arg0.buckets, v0);
            let v3 = 0x1::vector::empty<T0>();
            let v4 = 0x1::vector::empty<T1>();
            let v5 = 0;
            while (v5 < 0x1::vector::length<Entry<T0, T1>>(v2)) {
                let v6 = 0x1::vector::borrow<Entry<T0, T1>>(v2, v5);
                0x1::vector::push_back<T0>(&mut v3, v6.key);
                0x1::vector::push_back<T1>(&mut v4, v6.value);
                v5 = v5 + 1;
            };
            0x1::simple_map::add_all<T0, T1>(&mut v1, v3, v4);
            v0 = v0 + 1;
        };
        v1
    }
    
    public fun update_split_load_threshold<T0, T1>(arg0: &mut SmartTable<T0, T1>, arg1: u8) {
        assert!(arg1 <= 100 && arg1 > 0, 0x1::error::invalid_argument(5));
        arg0.split_load_threshold = arg1;
    }
    
    public fun update_target_bucket_size<T0, T1>(arg0: &mut SmartTable<T0, T1>, arg1: u64) {
        assert!(arg1 > 0, 0x1::error::invalid_argument(6));
        arg0.target_bucket_size = arg1;
    }
    
    public fun upsert<T0: copy + drop, T1: drop>(arg0: &mut SmartTable<T0, T1>, arg1: T0, arg2: T1) {
        if (!contains<T0, T1>(arg0, arg1)) {
            add<T0, T1>(arg0, arg1, arg2);
        } else {
            *borrow_mut<T0, T1>(arg0, arg1) = arg2;
        };
    }
    
    // decompiled from Move bytecode v6
}
