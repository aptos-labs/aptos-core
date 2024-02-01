module 0x1::simple_map {
    struct Element<T0, T1> has copy, drop, store {
        key: T0,
        value: T1,
    }
    
    struct SimpleMap<T0, T1> has copy, drop, store {
        data: vector<Element<T0, T1>>,
    }
    
    public fun borrow<T0: store, T1: store>(arg0: &SimpleMap<T0, T1>, arg1: &T0) : &T1 {
        let v0 = find<T0, T1>(arg0, arg1);
        assert!(0x1::option::is_some<u64>(&v0), 0x1::error::invalid_argument(2));
        &0x1::vector::borrow<Element<T0, T1>>(&arg0.data, 0x1::option::extract<u64>(&mut v0)).value
    }
    
    public fun borrow_mut<T0: store, T1: store>(arg0: &mut SimpleMap<T0, T1>, arg1: &T0) : &mut T1 {
        let v0 = find<T0, T1>(arg0, arg1);
        assert!(0x1::option::is_some<u64>(&v0), 0x1::error::invalid_argument(2));
        let v1 = 0x1::vector::borrow_mut<Element<T0, T1>>(&mut arg0.data, 0x1::option::extract<u64>(&mut v0));
        &mut v1.value
    }
    
    public fun destroy_empty<T0: store, T1: store>(arg0: SimpleMap<T0, T1>) {
        let SimpleMap { data: v0 } = arg0;
        0x1::vector::destroy_empty<Element<T0, T1>>(v0);
    }
    
    public fun length<T0: store, T1: store>(arg0: &SimpleMap<T0, T1>) : u64 {
        0x1::vector::length<Element<T0, T1>>(&arg0.data)
    }
    
    public fun add<T0: store, T1: store>(arg0: &mut SimpleMap<T0, T1>, arg1: T0, arg2: T1) {
        let v0 = find<T0, T1>(arg0, &arg1);
        assert!(0x1::option::is_none<u64>(&v0), 0x1::error::invalid_argument(1));
        let v1 = Element<T0, T1>{
            key   : arg1, 
            value : arg2,
        };
        0x1::vector::push_back<Element<T0, T1>>(&mut arg0.data, v1);
    }
    
    public fun add_all<T0: store, T1: store>(arg0: &mut SimpleMap<T0, T1>, arg1: vector<T0>, arg2: vector<T1>) {
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
    
    public fun contains_key<T0: store, T1: store>(arg0: &SimpleMap<T0, T1>, arg1: &T0) : bool {
        let v0 = find<T0, T1>(arg0, arg1);
        0x1::option::is_some<u64>(&v0)
    }
    
    public fun create<T0: store, T1: store>() : SimpleMap<T0, T1> {
        new<T0, T1>()
    }
    
    fun find<T0: store, T1: store>(arg0: &SimpleMap<T0, T1>, arg1: &T0) : 0x1::option::Option<u64> {
        let v0 = 0;
        while (v0 < 0x1::vector::length<Element<T0, T1>>(&arg0.data)) {
            if (&0x1::vector::borrow<Element<T0, T1>>(&arg0.data, v0).key == arg1) {
                return 0x1::option::some<u64>(v0)
            };
            v0 = v0 + 1;
        };
        0x1::option::none<u64>()
    }
    
    public fun keys<T0: copy, T1>(arg0: &SimpleMap<T0, T1>) : vector<T0> {
        let v0 = &arg0.data;
        let v1 = 0x1::vector::empty<T0>();
        let v2 = 0;
        while (v2 < 0x1::vector::length<Element<T0, T1>>(v0)) {
            0x1::vector::push_back<T0>(&mut v1, 0x1::vector::borrow<Element<T0, T1>>(v0, v2).key);
            v2 = v2 + 1;
        };
        v1
    }
    
    public fun new<T0: store, T1: store>() : SimpleMap<T0, T1> {
        SimpleMap<T0, T1>{data: 0x1::vector::empty<Element<T0, T1>>()}
    }
    
    public fun new_from<T0: store, T1: store>(arg0: vector<T0>, arg1: vector<T1>) : SimpleMap<T0, T1> {
        let v0 = new<T0, T1>();
        add_all<T0, T1>(&mut v0, arg0, arg1);
        v0
    }
    
    public fun remove<T0: store, T1: store>(arg0: &mut SimpleMap<T0, T1>, arg1: &T0) : (T0, T1) {
        let v0 = find<T0, T1>(arg0, arg1);
        assert!(0x1::option::is_some<u64>(&v0), 0x1::error::invalid_argument(2));
        let Element {
            key   : v1,
            value : v2,
        } = 0x1::vector::swap_remove<Element<T0, T1>>(&mut arg0.data, 0x1::option::extract<u64>(&mut v0));
        (v1, v2)
    }
    
    public fun to_vec_pair<T0: store, T1: store>(arg0: SimpleMap<T0, T1>) : (vector<T0>, vector<T1>) {
        let v0 = 0x1::vector::empty<T0>();
        let v1 = 0x1::vector::empty<T1>();
        let SimpleMap { data: v2 } = arg0;
        0x1::vector::reverse<Element<T0, T1>>(&mut v2);
        let v3 = v2;
        let v4 = 0x1::vector::length<Element<T0, T1>>(&v3);
        while (v4 > 0) {
            let Element {
                key   : v5,
                value : v6,
            } = 0x1::vector::pop_back<Element<T0, T1>>(&mut v3);
            0x1::vector::push_back<T0>(&mut v0, v5);
            0x1::vector::push_back<T1>(&mut v1, v6);
            v4 = v4 - 1;
        };
        0x1::vector::destroy_empty<Element<T0, T1>>(v3);
        (v0, v1)
    }
    
    public fun upsert<T0: store, T1: store>(arg0: &mut SimpleMap<T0, T1>, arg1: T0, arg2: T1) : (0x1::option::Option<T0>, 0x1::option::Option<T1>) {
        let v0 = &mut arg0.data;
        let v1 = 0x1::vector::length<Element<T0, T1>>(v0);
        let v2 = 0;
        while (v2 < v1) {
            if (&0x1::vector::borrow<Element<T0, T1>>(v0, v2).key == &arg1) {
                let v3 = Element<T0, T1>{
                    key   : arg1, 
                    value : arg2,
                };
                0x1::vector::push_back<Element<T0, T1>>(v0, v3);
                0x1::vector::swap<Element<T0, T1>>(v0, v2, v1);
                let Element {
                    key   : v4,
                    value : v5,
                } = 0x1::vector::pop_back<Element<T0, T1>>(v0);
                return (0x1::option::some<T0>(v4), 0x1::option::some<T1>(v5))
            };
            v2 = v2 + 1;
        };
        let v6 = Element<T0, T1>{
            key   : arg1, 
            value : arg2,
        };
        0x1::vector::push_back<Element<T0, T1>>(&mut arg0.data, v6);
        (0x1::option::none<T0>(), 0x1::option::none<T1>())
    }
    
    public fun values<T0, T1: copy>(arg0: &SimpleMap<T0, T1>) : vector<T1> {
        let v0 = &arg0.data;
        let v1 = 0x1::vector::empty<T1>();
        let v2 = 0;
        while (v2 < 0x1::vector::length<Element<T0, T1>>(v0)) {
            0x1::vector::push_back<T1>(&mut v1, 0x1::vector::borrow<Element<T0, T1>>(v0, v2).value);
            v2 = v2 + 1;
        };
        v1
    }
    
    // decompiled from Move bytecode v6
}
