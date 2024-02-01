module 0x1::smart_vector {
    struct SmartVector<T0> has store {
        inline_vec: vector<T0>,
        big_vec: 0x1::option::Option<0x1::big_vector::BigVector<T0>>,
        inline_capacity: 0x1::option::Option<u64>,
        bucket_size: 0x1::option::Option<u64>,
    }
    
    public fun contains<T0>(arg0: &SmartVector<T0>, arg1: &T0) : bool {
        if (is_empty<T0>(arg0)) {
            return false
        };
        let (v0, _) = index_of<T0>(arg0, arg1);
        v0
    }
    
    public fun index_of<T0>(arg0: &SmartVector<T0>, arg1: &T0) : (bool, u64) {
        let (v0, v1) = 0x1::vector::index_of<T0>(&arg0.inline_vec, arg1);
        if (v0) {
            (true, v1)
        } else {
            let (v4, v5) = if (0x1::option::is_some<0x1::big_vector::BigVector<T0>>(&arg0.big_vec)) {
                let v6 = 0x1::option::borrow<0x1::big_vector::BigVector<T0>>(&arg0.big_vec);
                let (v7, v8) = 0x1::big_vector::index_of<T0>(v6, arg1);
                (v7, v8 + 0x1::vector::length<T0>(&arg0.inline_vec))
            } else {
                (false, 0)
            };
            (v4, v5)
        }
    }
    
    public fun borrow<T0>(arg0: &SmartVector<T0>, arg1: u64) : &T0 {
        assert!(arg1 < length<T0>(arg0), 0x1::error::invalid_argument(1));
        let v0 = 0x1::vector::length<T0>(&arg0.inline_vec);
        if (arg1 < v0) {
            0x1::vector::borrow<T0>(&arg0.inline_vec, arg1)
        } else {
            0x1::big_vector::borrow<T0>(0x1::option::borrow<0x1::big_vector::BigVector<T0>>(&arg0.big_vec), arg1 - v0)
        }
    }
    
    public fun borrow_mut<T0>(arg0: &mut SmartVector<T0>, arg1: u64) : &mut T0 {
        assert!(arg1 < length<T0>(arg0), 0x1::error::invalid_argument(1));
        let v0 = 0x1::vector::length<T0>(&arg0.inline_vec);
        if (arg1 < v0) {
            0x1::vector::borrow_mut<T0>(&mut arg0.inline_vec, arg1)
        } else {
            0x1::big_vector::borrow_mut<T0>(0x1::option::borrow_mut<0x1::big_vector::BigVector<T0>>(&mut arg0.big_vec), arg1 - v0)
        }
    }
    
    public fun destroy<T0: drop>(arg0: SmartVector<T0>) {
        clear<T0>(&mut arg0);
        destroy_empty<T0>(arg0);
    }
    
    public fun destroy_empty<T0>(arg0: SmartVector<T0>) {
        assert!(is_empty<T0>(&arg0), 0x1::error::invalid_argument(2));
        let SmartVector {
            inline_vec      : v0,
            big_vec         : v1,
            inline_capacity : _,
            bucket_size     : _,
        } = arg0;
        0x1::vector::destroy_empty<T0>(v0);
        0x1::option::destroy_none<0x1::big_vector::BigVector<T0>>(v1);
    }
    
    public fun empty<T0: store>() : SmartVector<T0> {
        let v0 = 0x1::vector::empty<T0>();
        let v1 = 0x1::option::none<0x1::big_vector::BigVector<T0>>();
        let v2 = 0x1::option::none<u64>();
        let v3 = 0x1::option::none<u64>();
        SmartVector<T0>{
            inline_vec      : v0, 
            big_vec         : v1, 
            inline_capacity : v2, 
            bucket_size     : v3,
        }
    }
    
    public fun is_empty<T0>(arg0: &SmartVector<T0>) : bool {
        length<T0>(arg0) == 0
    }
    
    public fun length<T0>(arg0: &SmartVector<T0>) : u64 {
        let v0 = if (0x1::option::is_none<0x1::big_vector::BigVector<T0>>(&arg0.big_vec)) {
            0
        } else {
            0x1::big_vector::length<T0>(0x1::option::borrow<0x1::big_vector::BigVector<T0>>(&arg0.big_vec))
        };
        0x1::vector::length<T0>(&arg0.inline_vec) + v0
    }
    
    public fun pop_back<T0>(arg0: &mut SmartVector<T0>) : T0 {
        assert!(!is_empty<T0>(arg0), 0x1::error::invalid_state(3));
        let v0 = &mut arg0.big_vec;
        if (0x1::option::is_some<0x1::big_vector::BigVector<T0>>(v0)) {
            let v2 = 0x1::option::extract<0x1::big_vector::BigVector<T0>>(v0);
            if (0x1::big_vector::is_empty<T0>(&v2)) {
                0x1::big_vector::destroy_empty<T0>(v2);
            } else {
                0x1::option::fill<0x1::big_vector::BigVector<T0>>(v0, v2);
            };
            0x1::big_vector::pop_back<T0>(&mut v2)
        } else {
            0x1::vector::pop_back<T0>(&mut arg0.inline_vec)
        }
    }
    
    public fun push_back<T0: store>(arg0: &mut SmartVector<T0>, arg1: T0) {
        let v0 = length<T0>(arg0);
        let v1 = 0x1::vector::length<T0>(&arg0.inline_vec);
        if (v0 == v1) {
            let v2 = if (0x1::option::is_some<u64>(&arg0.inline_capacity)) {
                if (v0 < *0x1::option::borrow<u64>(&arg0.inline_capacity)) {
                    0x1::vector::push_back<T0>(&mut arg0.inline_vec, arg1);
                    return
                };
                *0x1::option::borrow<u64>(&arg0.bucket_size)
            } else {
                let v3 = 0x1::type_info::size_of_val<T0>(&arg1);
                if (v3 * (v1 + 1) < 150) {
                    0x1::vector::push_back<T0>(&mut arg0.inline_vec, arg1);
                    return
                };
                0x1::math64::max(1024 / 0x1::math64::max((0x1::type_info::size_of_val<vector<T0>>(&arg0.inline_vec) + v3) / (v1 + 1), 1), 1)
            };
            0x1::option::fill<0x1::big_vector::BigVector<T0>>(&mut arg0.big_vec, 0x1::big_vector::empty<T0>(v2));
        };
        let v4 = 0x1::option::borrow_mut<0x1::big_vector::BigVector<T0>>(&mut arg0.big_vec);
        0x1::big_vector::push_back<T0>(v4, arg1);
    }
    
    public fun remove<T0>(arg0: &mut SmartVector<T0>, arg1: u64) : T0 {
        assert!(arg1 < length<T0>(arg0), 0x1::error::invalid_argument(1));
        let v0 = 0x1::vector::length<T0>(&arg0.inline_vec);
        if (arg1 < v0) {
            0x1::vector::remove<T0>(&mut arg0.inline_vec, arg1)
        } else {
            let v2 = &mut arg0.big_vec;
            let v3 = 0x1::option::extract<0x1::big_vector::BigVector<T0>>(v2);
            if (0x1::big_vector::is_empty<T0>(&v3)) {
                0x1::big_vector::destroy_empty<T0>(v3);
            } else {
                0x1::option::fill<0x1::big_vector::BigVector<T0>>(v2, v3);
            };
            0x1::big_vector::remove<T0>(&mut v3, arg1 - v0)
        }
    }
    
    public fun reverse<T0: store>(arg0: &mut SmartVector<T0>) {
        let v0 = 0;
        let v1 = 0x1::vector::empty<T0>();
        while (v0 < 0x1::vector::length<T0>(&arg0.inline_vec)) {
            0x1::vector::push_back<T0>(&mut v1, pop_back<T0>(arg0));
            v0 = v0 + 1;
        };
        0x1::vector::reverse<T0>(&mut v1);
        if (0x1::option::is_some<0x1::big_vector::BigVector<T0>>(&arg0.big_vec)) {
            let v2 = 0x1::option::borrow_mut<0x1::big_vector::BigVector<T0>>(&mut arg0.big_vec);
            0x1::big_vector::reverse<T0>(v2);
        };
        let v3 = 0x1::vector::empty<T0>();
        while (!0x1::vector::is_empty<T0>(&mut arg0.inline_vec)) {
            0x1::vector::push_back<T0>(&mut v3, 0x1::vector::pop_back<T0>(&mut arg0.inline_vec));
        };
        0x1::vector::reverse<T0>(&mut v3);
        while (!0x1::vector::is_empty<T0>(&mut v1)) {
            0x1::vector::push_back<T0>(&mut arg0.inline_vec, 0x1::vector::pop_back<T0>(&mut v1));
        };
        0x1::vector::destroy_empty<T0>(v1);
        while (!0x1::vector::is_empty<T0>(&mut v3)) {
            push_back<T0>(arg0, 0x1::vector::pop_back<T0>(&mut v3));
        };
        0x1::vector::destroy_empty<T0>(v3);
    }
    
    public fun swap<T0: store>(arg0: &mut SmartVector<T0>, arg1: u64, arg2: u64) {
        if (arg1 > arg2) {
            swap<T0>(arg0, arg2, arg1);
            return
        };
        let v0 = length<T0>(arg0);
        assert!(arg2 < v0, 0x1::error::invalid_argument(1));
        let v1 = 0x1::vector::length<T0>(&arg0.inline_vec);
        if (arg1 >= v1) {
            let v2 = 0x1::option::borrow_mut<0x1::big_vector::BigVector<T0>>(&mut arg0.big_vec);
            0x1::big_vector::swap<T0>(v2, arg1 - v1, arg2 - v1);
        } else {
            if (arg2 < v1) {
                0x1::vector::swap<T0>(&mut arg0.inline_vec, arg1, arg2);
            } else {
                let v3 = 0x1::option::borrow_mut<0x1::big_vector::BigVector<T0>>(&mut arg0.big_vec);
                let v4 = &mut arg0.inline_vec;
                0x1::vector::push_back<T0>(v4, 0x1::big_vector::swap_remove<T0>(v3, arg2 - v1));
                0x1::vector::swap<T0>(v4, arg1, v1 - 1);
                0x1::big_vector::push_back<T0>(v3, 0x1::vector::swap_remove<T0>(v4, arg1));
                0x1::big_vector::swap<T0>(v3, arg2 - v1, v0 - v1 - 1);
            };
        };
    }
    
    public fun swap_remove<T0>(arg0: &mut SmartVector<T0>, arg1: u64) : T0 {
        let v0 = length<T0>(arg0);
        assert!(arg1 < v0, 0x1::error::invalid_argument(1));
        let v1 = 0x1::vector::length<T0>(&arg0.inline_vec);
        let v2 = &mut arg0.big_vec;
        let v3 = &mut arg0.inline_vec;
        if (arg1 >= v1) {
            let v5 = 0x1::option::extract<0x1::big_vector::BigVector<T0>>(v2);
            if (0x1::big_vector::is_empty<T0>(&v5)) {
                0x1::big_vector::destroy_empty<T0>(v5);
            } else {
                0x1::option::fill<0x1::big_vector::BigVector<T0>>(v2, v5);
            };
            0x1::big_vector::swap_remove<T0>(&mut v5, arg1 - v1)
        } else {
            if (v1 < v0) {
                let v6 = 0x1::option::extract<0x1::big_vector::BigVector<T0>>(v2);
                if (0x1::big_vector::is_empty<T0>(&v6)) {
                    0x1::big_vector::destroy_empty<T0>(v6);
                } else {
                    0x1::option::fill<0x1::big_vector::BigVector<T0>>(v2, v6);
                };
                0x1::vector::push_back<T0>(v3, 0x1::big_vector::pop_back<T0>(&mut v6));
            };
            0x1::vector::swap_remove<T0>(v3, arg1)
        }
    }
    
    public fun to_vector<T0: copy + store>(arg0: &SmartVector<T0>) : vector<T0> {
        let v0 = arg0.inline_vec;
        if (0x1::option::is_some<0x1::big_vector::BigVector<T0>>(&arg0.big_vec)) {
            let v1 = 0x1::big_vector::to_vector<T0>(0x1::option::borrow<0x1::big_vector::BigVector<T0>>(&arg0.big_vec));
            0x1::vector::append<T0>(&mut v0, v1);
        };
        v0
    }
    
    public fun append<T0: store>(arg0: &mut SmartVector<T0>, arg1: SmartVector<T0>) {
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
    
    public fun add_all<T0: store>(arg0: &mut SmartVector<T0>, arg1: vector<T0>) {
        let v0 = arg1;
        0x1::vector::reverse<T0>(&mut v0);
        let v1 = v0;
        let v2 = 0x1::vector::length<T0>(&v1);
        while (v2 > 0) {
            push_back<T0>(arg0, 0x1::vector::pop_back<T0>(&mut v1));
            v2 = v2 - 1;
        };
        0x1::vector::destroy_empty<T0>(v1);
    }
    
    public fun clear<T0: drop>(arg0: &mut SmartVector<T0>) {
        arg0.inline_vec = 0x1::vector::empty<T0>();
        if (0x1::option::is_some<0x1::big_vector::BigVector<T0>>(&arg0.big_vec)) {
            let v0 = 0x1::option::extract<0x1::big_vector::BigVector<T0>>(&mut arg0.big_vec);
            0x1::big_vector::destroy<T0>(v0);
        };
    }
    
    public fun empty_with_config<T0: store>(arg0: u64, arg1: u64) : SmartVector<T0> {
        assert!(arg1 > 0, 0x1::error::invalid_argument(4));
        let v0 = 0x1::vector::empty<T0>();
        let v1 = 0x1::option::none<0x1::big_vector::BigVector<T0>>();
        let v2 = 0x1::option::some<u64>(arg0);
        let v3 = 0x1::option::some<u64>(arg1);
        SmartVector<T0>{
            inline_vec      : v0, 
            big_vec         : v1, 
            inline_capacity : v2, 
            bucket_size     : v3,
        }
    }
    
    public fun new<T0: store>() : SmartVector<T0> {
        empty<T0>()
    }
    
    public fun singleton<T0: store>(arg0: T0) : SmartVector<T0> {
        let v0 = empty<T0>();
        push_back<T0>(&mut v0, arg0);
        v0
    }
    
    // decompiled from Move bytecode v6
}
