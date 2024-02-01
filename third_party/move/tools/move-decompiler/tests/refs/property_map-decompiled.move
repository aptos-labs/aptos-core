module 0x1337::property_map {
    struct PropertyMap has copy, drop, store {
        map: 0x1::simple_map::SimpleMap<0x1::string::String, PropertyValue>,
    }
    
    struct PropertyValue has copy, drop, store {
        value: vector<u8>,
        type: 0x1::string::String,
    }
    
    public fun add(arg0: &mut PropertyMap, arg1: 0x1::string::String, arg2: PropertyValue) {
        assert!(0x1::string::length(&arg1) <= 128, 0x1::error::invalid_argument(7));
        let v0 = 0x1::simple_map::length<0x1::string::String, PropertyValue>(&arg0.map) < 1000;
        assert!(v0, 0x1::error::invalid_state(2));
        0x1::simple_map::add<0x1::string::String, PropertyValue>(&mut arg0.map, arg1, arg2);
    }
    
    public fun borrow(arg0: &PropertyMap, arg1: &0x1::string::String) : &PropertyValue {
        assert!(contains_key(arg0, arg1), 3);
        0x1::simple_map::borrow<0x1::string::String, PropertyValue>(&arg0.map, arg1)
    }
    
    public fun contains_key(arg0: &PropertyMap, arg1: &0x1::string::String) : bool {
        0x1::simple_map::contains_key<0x1::string::String, PropertyValue>(&arg0.map, arg1)
    }
    
    public fun keys(arg0: &PropertyMap) : vector<0x1::string::String> {
        0x1::simple_map::keys<0x1::string::String, PropertyValue>(&arg0.map)
    }
    
    public fun length(arg0: &PropertyMap) : u64 {
        0x1::simple_map::length<0x1::string::String, PropertyValue>(&arg0.map)
    }
    
    public fun remove(arg0: &mut PropertyMap, arg1: &0x1::string::String) : (0x1::string::String, PropertyValue) {
        assert!(contains_key(arg0, arg1), 0x1::error::not_found(3));
        0x1::simple_map::remove<0x1::string::String, PropertyValue>(&mut arg0.map, arg1)
    }
    
    public fun values(arg0: &PropertyMap) : vector<vector<u8>> {
        let v0 = 0x1::simple_map::values<0x1::string::String, PropertyValue>(&arg0.map);
        let v1 = &v0;
        let v2 = vector[];
        let v3 = 0;
        while (v3 < 0x1::vector::length<PropertyValue>(v1)) {
            0x1::vector::push_back<vector<u8>>(&mut v2, 0x1::vector::borrow<PropertyValue>(v1, v3).value);
            v3 = v3 + 1;
        };
        v2
    }
    
    public fun empty() : PropertyMap {
        PropertyMap{map: 0x1::simple_map::create<0x1::string::String, PropertyValue>()}
    }
    
    public fun borrow_type(arg0: &PropertyValue) : 0x1::string::String {
        arg0.type
    }
    
    public fun borrow_value(arg0: &PropertyValue) : vector<u8> {
        arg0.value
    }
    
    public fun create_property_value<T0: copy>(arg0: &T0) : PropertyValue {
        let v0 = 0x1::type_info::type_name<T0>();
        let v1 = 0x1::string::utf8(b"bool");
        if (v0 == v1 || v0 == 0x1::string::utf8(b"u8") || v0 == 0x1::string::utf8(b"u64") || v0 == 0x1::string::utf8(b"u128") || v0 == 0x1::string::utf8(b"address") || v0 == 0x1::string::utf8(b"0x1::string::String")) {
            create_property_value_raw(0x1::bcs::to_bytes<T0>(arg0), v0)
        } else {
            create_property_value_raw(0x1::bcs::to_bytes<T0>(arg0), 0x1::string::utf8(b"vector<u8>"))
        }
    }
    
    public fun create_property_value_raw(arg0: vector<u8>, arg1: 0x1::string::String) : PropertyValue {
        PropertyValue{
            value : arg0, 
            type  : arg1,
        }
    }
    
    public fun new(arg0: vector<0x1::string::String>, arg1: vector<vector<u8>>, arg2: vector<0x1::string::String>) : PropertyMap {
        let v0 = 0x1::vector::length<0x1::string::String>(&arg0);
        assert!(v0 <= 1000, 0x1::error::invalid_argument(2));
        assert!(v0 == 0x1::vector::length<vector<u8>>(&arg1), 0x1::error::invalid_argument(4));
        assert!(v0 == 0x1::vector::length<0x1::string::String>(&arg2), 0x1::error::invalid_argument(5));
        let v1 = empty();
        let v2 = 0;
        while (v2 < v0) {
            let v3 = *0x1::vector::borrow<0x1::string::String>(&arg0, v2);
            assert!(0x1::string::length(&v3) <= 128, 0x1::error::invalid_argument(7));
            let v4 = *0x1::vector::borrow<0x1::string::String>(&arg2, v2);
            let v5 = PropertyValue{
                value : *0x1::vector::borrow<vector<u8>>(&arg1, v2), 
                type  : v4,
            };
            0x1::simple_map::add<0x1::string::String, PropertyValue>(&mut v1.map, v3, v5);
            v2 = v2 + 1;
        };
        v1
    }
    
    public fun new_with_key_and_property_value(arg0: vector<0x1::string::String>, arg1: vector<PropertyValue>) : PropertyMap {
        let v0 = 0x1::vector::length<0x1::string::String>(&arg0);
        assert!(v0 <= 1000, 0x1::error::invalid_argument(2));
        assert!(v0 == 0x1::vector::length<PropertyValue>(&arg1), 0x1::error::invalid_argument(4));
        let v1 = empty();
        let v2 = 0;
        while (v2 < v0) {
            let v3 = *0x1::vector::borrow<0x1::string::String>(&arg0, v2);
            let v4 = *0x1::vector::borrow<PropertyValue>(&arg1, v2);
            assert!(0x1::string::length(&v3) <= 128, 0x1::error::invalid_argument(7));
            add(&mut v1, v3, v4);
            v2 = v2 + 1;
        };
        v1
    }
    
    public fun read_address(arg0: &PropertyMap, arg1: &0x1::string::String) : address {
        let v0 = borrow(arg0, arg1);
        assert!(v0.type == 0x1::string::utf8(b"address"), 0x1::error::invalid_state(6));
        0x1::from_bcs::to_address(v0.value)
    }
    
    public fun read_bool(arg0: &PropertyMap, arg1: &0x1::string::String) : bool {
        let v0 = borrow(arg0, arg1);
        assert!(v0.type == 0x1::string::utf8(b"bool"), 0x1::error::invalid_state(6));
        0x1::from_bcs::to_bool(v0.value)
    }
    
    public fun read_string(arg0: &PropertyMap, arg1: &0x1::string::String) : 0x1::string::String {
        let v0 = borrow(arg0, arg1);
        assert!(v0.type == 0x1::string::utf8(b"0x1::string::String"), 0x1::error::invalid_state(6));
        0x1::from_bcs::to_string(v0.value)
    }
    
    public fun read_u128(arg0: &PropertyMap, arg1: &0x1::string::String) : u128 {
        let v0 = borrow(arg0, arg1);
        assert!(v0.type == 0x1::string::utf8(b"u128"), 0x1::error::invalid_state(6));
        0x1::from_bcs::to_u128(v0.value)
    }
    
    public fun read_u64(arg0: &PropertyMap, arg1: &0x1::string::String) : u64 {
        let v0 = borrow(arg0, arg1);
        assert!(v0.type == 0x1::string::utf8(b"u64"), 0x1::error::invalid_state(6));
        0x1::from_bcs::to_u64(v0.value)
    }
    
    public fun read_u8(arg0: &PropertyMap, arg1: &0x1::string::String) : u8 {
        let v0 = borrow(arg0, arg1);
        assert!(v0.type == 0x1::string::utf8(b"u8"), 0x1::error::invalid_state(6));
        0x1::from_bcs::to_u8(v0.value)
    }
    
    public fun types(arg0: &PropertyMap) : vector<0x1::string::String> {
        let v0 = 0x1::simple_map::values<0x1::string::String, PropertyValue>(&arg0.map);
        let v1 = &v0;
        let v2 = 0x1::vector::empty<0x1::string::String>();
        let v3 = 0;
        while (v3 < 0x1::vector::length<PropertyValue>(v1)) {
            let v4 = 0x1::vector::borrow<PropertyValue>(v1, v3).type;
            0x1::vector::push_back<0x1::string::String>(&mut v2, v4);
            v3 = v3 + 1;
        };
        v2
    }
    
    public fun update_property_map(arg0: &mut PropertyMap, arg1: vector<0x1::string::String>, arg2: vector<vector<u8>>, arg3: vector<0x1::string::String>) {
        let v0 = 0x1::vector::length<0x1::string::String>(&arg1);
        assert!(v0 == 0x1::vector::length<vector<u8>>(&arg2), 0x1::error::invalid_state(4));
        assert!(v0 == 0x1::vector::length<0x1::string::String>(&arg3), 0x1::error::invalid_state(5));
        let v1 = 0;
        while (v1 < v0) {
            let v2 = 0x1::vector::borrow<0x1::string::String>(&arg1, v1);
            let v3 = *0x1::vector::borrow<0x1::string::String>(&arg3, v1);
            let v4 = PropertyValue{
                value : *0x1::vector::borrow<vector<u8>>(&arg2, v1), 
                type  : v3,
            };
            if (contains_key(arg0, v2)) {
                update_property_value(arg0, v2, v4);
            } else {
                add(arg0, *v2, v4);
            };
            v1 = v1 + 1;
        };
    }
    
    public fun update_property_value(arg0: &mut PropertyMap, arg1: &0x1::string::String, arg2: PropertyValue) {
        *0x1::simple_map::borrow_mut<0x1::string::String, PropertyValue>(&mut arg0.map, arg1) = arg2;
    }
    
    // decompiled from Move bytecode v6
}
