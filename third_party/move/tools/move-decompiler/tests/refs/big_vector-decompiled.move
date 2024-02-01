module 0x1::big_vector {
    struct BigVector<T0> has store {
        buckets: 0x1::table_with_length::TableWithLength<u64, vector<T0>>,
        end_index: u64,
        bucket_size: u64,
    }
    
    public fun contains<T0>(arg0: &BigVector<T0>, arg1: &T0) : bool {
        if (is_empty<T0>(arg0)) {
            return false
        };
        let (v0, _) = index_of<T0>(arg0, arg1);
        v0
    }
    
    public fun index_of<T0>(arg0: &BigVector<T0>, arg1: &T0) : (bool, u64) {
        let v0 = 0;
        while (v0 < 0x1::table_with_length::length<u64, vector<T0>>(&arg0.buckets)) {
            let (v1, v2) = 0x1::vector::index_of<T0>(0x1::table_with_length::borrow<u64, vector<T0>>(&arg0.buckets, v0), arg1);
            if (v1) {
                return (true, v0 * arg0.bucket_size + v2)
            };
            v0 = v0 + 1;
        };
        (false, 0)
    }
    
    public fun borrow<T0>(arg0: &BigVector<T0>, arg1: u64) : &T0 {
        assert!(arg1 < length<T0>(arg0), 0x1::error::invalid_argument(1));
        let v0 = 0x1::table_with_length::borrow<u64, vector<T0>>(&arg0.buckets, arg1 / arg0.bucket_size);
        0x1::vector::borrow<T0>(v0, arg1 % arg0.bucket_size)
    }
    
    public fun borrow_mut<T0>(arg0: &mut BigVector<T0>, arg1: u64) : &mut T0 {
        assert!(arg1 < length<T0>(arg0), 0x1::error::invalid_argument(1));
        let v0 = 0x1::table_with_length::borrow_mut<u64, vector<T0>>(&mut arg0.buckets, arg1 / arg0.bucket_size);
        0x1::vector::borrow_mut<T0>(v0, arg1 % arg0.bucket_size)
    }
    
    public fun destroy_empty<T0>(arg0: BigVector<T0>) {
        assert!(is_empty<T0>(&arg0), 0x1::error::invalid_argument(2));
        let BigVector {
            buckets     : v0,
            end_index   : _,
            bucket_size : _,
        } = arg0;
        0x1::table_with_length::destroy_empty<u64, vector<T0>>(v0);
    }
    
    public fun length<T0>(arg0: &BigVector<T0>) : u64 {
        arg0.end_index
    }
    
    public fun remove<T0>(arg0: &mut BigVector<T0>, arg1: u64) : T0 {
        assert!(arg1 < length<T0>(arg0), 0x1::error::invalid_argument(1));
        let v0 = 0x1::table_with_length::length<u64, vector<T0>>(&arg0.buckets);
        let v1 = arg1 / arg0.bucket_size + 1;
        let v2 = v1;
        let v3 = 0x1::table_with_length::borrow_mut<u64, vector<T0>>(&mut arg0.buckets, v1 - 1);
        arg0.end_index = arg0.end_index - 1;
        while (v2 < v0) {
            let v4 = &mut arg0.buckets;
            let v5 = 0x1::vector::remove<T0>(0x1::table_with_length::borrow_mut<u64, vector<T0>>(v4, v2), 0);
            let v6 = 0x1::table_with_length::borrow_mut<u64, vector<T0>>(&mut arg0.buckets, v2 - 1);
            0x1::vector::push_back<T0>(v6, v5);
            v2 = v2 + 1;
        };
        let v7 = 0x1::table_with_length::borrow_mut<u64, vector<T0>>(&mut arg0.buckets, v0 - 1);
        if (0x1::vector::is_empty<T0>(v7)) {
            let v8 = 0x1::table_with_length::remove<u64, vector<T0>>(&mut arg0.buckets, v0 - 1);
            0x1::vector::destroy_empty<T0>(v8);
        };
        0x1::vector::remove<T0>(v3, arg1 % arg0.bucket_size)
    }
    
    public fun append<T0: store>(arg0: &mut BigVector<T0>, arg1: BigVector<T0>) {
        let v0 = length<T0>(&arg1);
        let v1 = 0;
        while (v1 < v0 / 2) {
            push_back<T0>(arg0, swap_remove<T0>(&mut arg1, v1));
            v1 = v1 + 1;
        };
        while (v1 < v0) {
            push_back<T0>(arg0, pop_back<T0>(&mut arg1));
            v1 = v1 + 1;
        };
        destroy_empty<T0>(arg1);
    }
    
    public(friend) fun empty<T0: store>(arg0: u64) : BigVector<T0> {
        assert!(arg0 > 0, 0x1::error::invalid_argument(4));
        let v0 = 0x1::table_with_length::new<u64, vector<T0>>();
        BigVector<T0>{
            buckets     : v0, 
            end_index   : 0, 
            bucket_size : arg0,
        }
    }
    
    public fun is_empty<T0>(arg0: &BigVector<T0>) : bool {
        length<T0>(arg0) == 0
    }
    
    public fun pop_back<T0>(arg0: &mut BigVector<T0>) : T0 {
        assert!(!is_empty<T0>(arg0), 0x1::error::invalid_state(3));
        let v0 = 0x1::table_with_length::length<u64, vector<T0>>(&arg0.buckets);
        let v1 = 0x1::table_with_length::borrow_mut<u64, vector<T0>>(&mut arg0.buckets, v0 - 1);
        if (0x1::vector::is_empty<T0>(v1)) {
            let v2 = 0x1::table_with_length::remove<u64, vector<T0>>(&mut arg0.buckets, v0 - 1);
            0x1::vector::destroy_empty<T0>(v2);
        };
        arg0.end_index = arg0.end_index - 1;
        0x1::vector::pop_back<T0>(v1)
    }
    
    public fun push_back<T0: store>(arg0: &mut BigVector<T0>, arg1: T0) {
        let v0 = 0x1::table_with_length::length<u64, vector<T0>>(&arg0.buckets);
        if (arg0.end_index == v0 * arg0.bucket_size) {
            0x1::table_with_length::add<u64, vector<T0>>(&mut arg0.buckets, v0, 0x1::vector::empty<T0>());
            let v1 = 0x1::table_with_length::borrow_mut<u64, vector<T0>>(&mut arg0.buckets, v0);
            0x1::vector::push_back<T0>(v1, arg1);
        } else {
            let v2 = 0x1::table_with_length::borrow_mut<u64, vector<T0>>(&mut arg0.buckets, v0 - 1);
            0x1::vector::push_back<T0>(v2, arg1);
        };
        arg0.end_index = arg0.end_index + 1;
    }
    
    public fun reverse<T0>(arg0: &mut BigVector<T0>) {
        let v0 = 0x1::vector::empty<vector<T0>>();
        let v1 = 0x1::vector::empty<T0>();
        let v2 = 0x1::table_with_length::length<u64, vector<T0>>(&arg0.buckets);
        let v3 = v2;
        while (v3 > 0) {
            let v4 = 0x1::table_with_length::remove<u64, vector<T0>>(&mut arg0.buckets, v3 - 1);
            let v5 = 0x1::vector::length<T0>(&v4);
            while (v5 > 0) {
                0x1::vector::push_back<T0>(&mut v1, 0x1::vector::pop_back<T0>(&mut v4));
                if (0x1::vector::length<T0>(&v1) == arg0.bucket_size) {
                    0x1::vector::push_back<vector<T0>>(&mut v0, v1);
                    v1 = 0x1::vector::empty<T0>();
                };
                v5 = v5 - 1;
            };
            0x1::vector::destroy_empty<T0>(v4);
            v3 = v3 - 1;
        };
        if (0x1::vector::length<T0>(&v1) > 0) {
            0x1::vector::push_back<vector<T0>>(&mut v0, v1);
        } else {
            0x1::vector::destroy_empty<T0>(v1);
        };
        0x1::vector::reverse<vector<T0>>(&mut v0);
        let v6 = 0;
        assert!(0x1::table_with_length::length<u64, vector<T0>>(&arg0.buckets) == 0, 0);
        while (v6 < v2) {
            let v7 = 0x1::vector::pop_back<vector<T0>>(&mut v0);
            0x1::table_with_length::add<u64, vector<T0>>(&mut arg0.buckets, v6, v7);
            v6 = v6 + 1;
        };
        0x1::vector::destroy_empty<vector<T0>>(v0);
    }
    
    public fun swap<T0>(arg0: &mut BigVector<T0>, arg1: u64, arg2: u64) {
        assert!(arg1 < length<T0>(arg0) && arg2 < length<T0>(arg0), 0x1::error::invalid_argument(1));
        let v0 = arg1 / arg0.bucket_size;
        let v1 = arg2 / arg0.bucket_size;
        let v2 = arg1 % arg0.bucket_size;
        let v3 = arg2 % arg0.bucket_size;
        if (v0 == v1) {
            let v4 = 0x1::table_with_length::borrow_mut<u64, vector<T0>>(&mut arg0.buckets, v0);
            0x1::vector::swap<T0>(v4, v2, v3);
            return
        };
        let v5 = 0x1::table_with_length::remove<u64, vector<T0>>(&mut arg0.buckets, v0);
        let v6 = 0x1::table_with_length::remove<u64, vector<T0>>(&mut arg0.buckets, v1);
        0x1::vector::push_back<T0>(&mut v5, 0x1::vector::swap_remove<T0>(&mut v6, v3));
        0x1::vector::push_back<T0>(&mut v6, 0x1::vector::swap_remove<T0>(&mut v5, v2));
        0x1::vector::swap<T0>(&mut v5, v2, 0x1::vector::length<T0>(&v5) - 1);
        0x1::vector::swap<T0>(&mut v6, v3, 0x1::vector::length<T0>(&v6) - 1);
        0x1::table_with_length::add<u64, vector<T0>>(&mut arg0.buckets, v0, v5);
        0x1::table_with_length::add<u64, vector<T0>>(&mut arg0.buckets, v1, v6);
    }
    
    public fun swap_remove<T0>(arg0: &mut BigVector<T0>, arg1: u64) : T0 {
        assert!(arg1 < length<T0>(arg0), 0x1::error::invalid_argument(1));
        if (arg0.end_index == arg1) {
            return pop_back<T0>(arg0)
        };
        let v0 = 0x1::table_with_length::borrow_mut<u64, vector<T0>>(&mut arg0.buckets, arg1 / arg0.bucket_size);
        0x1::vector::push_back<T0>(v0, pop_back<T0>(arg0));
        0x1::vector::swap<T0>(v0, arg1 % arg0.bucket_size, 0x1::vector::length<T0>(v0) - 1);
        0x1::vector::swap_remove<T0>(v0, arg1 % arg0.bucket_size)
    }
    
    public fun destroy<T0: drop>(arg0: BigVector<T0>) {
        let BigVector {
            buckets     : v0,
            end_index   : v1,
            bucket_size : _,
        } = arg0;
        let v3 = v1;
        let v4 = v0;
        let v5 = 0;
        while (v3 > 0) {
            let v6 = 0x1::table_with_length::remove<u64, vector<T0>>(&mut v4, v5);
            v3 = v3 - 0x1::vector::length<T0>(&v6);
            v5 = v5 + 1;
        };
        0x1::table_with_length::destroy_empty<u64, vector<T0>>(v4);
    }
    
    public(friend) fun singleton<T0: store>(arg0: T0, arg1: u64) : BigVector<T0> {
        let v0 = empty<T0>(arg1);
        push_back<T0>(&mut v0, arg0);
        v0
    }
    
    public fun to_vector<T0: copy>(arg0: &BigVector<T0>) : vector<T0> {
        let v0 = 0x1::vector::empty<T0>();
        let v1 = 0;
        while (v1 < 0x1::table_with_length::length<u64, vector<T0>>(&arg0.buckets)) {
            let v2 = *0x1::table_with_length::borrow<u64, vector<T0>>(&arg0.buckets, v1);
            0x1::vector::append<T0>(&mut v0, v2);
            v1 = v1 + 1;
        };
        v0
    }
    
    // decompiled from Move bytecode v6
}
