spec aptos_token::property_map {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;

        let MAX_PROPERTY_MAP_SIZE = 1000;
        let MAX_PROPERTY_NAME_LENGTH  = 128;
    }

    spec new (
        keys: vector<String>,
        values: vector<vector<u8>>,
        types: vector<String>
    ): PropertyMap {
        // TODO: Can't handle abort in loop.
        pragma aborts_if_is_partial;
        let length = len(keys);

        aborts_if !(length <= MAX_PROPERTY_MAP_SIZE);
        aborts_if !(length == vector::length(values));
        aborts_if !(length == vector::length(types));
    }

    spec new_with_key_and_property_value (
        keys: vector<String>,
        values: vector<PropertyValue>
    ): PropertyMap {
        // TODO: Can't handle abort in loop.
        pragma aborts_if_is_partial;
        let length = vector::length(keys);
        aborts_if !(length <= MAX_PROPERTY_MAP_SIZE);
        aborts_if !(length == len(values));
    }

    spec empty(): PropertyMap {
        aborts_if false;
    }

    spec contains_key(map: &PropertyMap, key: &String): bool {
        aborts_if false;
    }

    spec add(map: &mut PropertyMap, key: String, value: PropertyValue) {
        use aptos_framework::simple_map;

        aborts_if !(string::length(key) <= MAX_PROPERTY_NAME_LENGTH);
        aborts_if !(!simple_map::spec_contains_key(map.map, key));
        aborts_if !(simple_map::spec_len(map.map) < MAX_PROPERTY_MAP_SIZE);
    }

    spec length(map: &PropertyMap): u64 {
        aborts_if false;
    }

    spec keys(map: &PropertyMap): vector<String> {
        pragma verify = false;
    }

    spec types(map: &PropertyMap): vector<String> {
        pragma verify = false;
    }

    spec values(map: &PropertyMap): vector<vector<u8>> {
        pragma verify = false;
    }

    spec borrow(map: &PropertyMap, key: &String): &PropertyValue {
        use aptos_framework::simple_map;
        aborts_if !simple_map::spec_contains_key(map.map, key);
    }

    /// Check utf8 for correctness and whether equal
    /// to `prop.type`
    spec read_string(map: &PropertyMap, key: &String): String {
        use std::string;
        use aptos_framework::simple_map;
        pragma aborts_if_is_partial;

        // TODO: Unable to handle abort from `from_bcs::to_string` because there is a function call at assert.
        aborts_if !simple_map::spec_contains_key(map.map, key);
        aborts_if !string::spec_internal_check_utf8(b"0x1::string::String");
        let prop = simple_map::spec_get(map.map, key);
        aborts_if prop.type != spec_utf8(b"0x1::string::String");
        aborts_if !aptos_std::from_bcs::deserializable<String>(prop.value);
    }

    spec fun spec_utf8(bytes: vector<u8>): String {
        String{bytes}
    }

    spec read_u8(map: &PropertyMap, key: &String): u8 {
        use std::string;
        use aptos_framework::simple_map;

        let str = b"u8";
        aborts_if !simple_map::spec_contains_key(map.map, key);
        aborts_if !string::spec_internal_check_utf8(str);
        let prop = simple_map::spec_get(map.map, key);
        aborts_if prop.type != spec_utf8(str);
        aborts_if !aptos_std::from_bcs::deserializable<u8>(prop.value);
    }

    spec read_u64(map: &PropertyMap, key: &String): u64 {
        use std::string;
        use aptos_framework::simple_map;

        let str = b"u64";
        aborts_if !simple_map::spec_contains_key(map.map, key);
        aborts_if !string::spec_internal_check_utf8(str);
        let prop = simple_map::spec_get(map.map, key);
        aborts_if prop.type != spec_utf8(str);
        aborts_if !aptos_std::from_bcs::deserializable<u64>(prop.value);
    }

    spec read_address(map: &PropertyMap, key: &String): address {
        use std::string;
        use aptos_framework::simple_map;

        let str = b"address";
        aborts_if !simple_map::spec_contains_key(map.map, key);
        aborts_if !string::spec_internal_check_utf8(str);
        let prop = simple_map::spec_get(map.map, key);
        aborts_if prop.type != spec_utf8(str);
        aborts_if !aptos_std::from_bcs::deserializable<address>(prop.value);
    }

    spec read_u128(map: &PropertyMap, key: &String): u128 {
        use std::string;
        use aptos_framework::simple_map;

        let str = b"u128";
        aborts_if !simple_map::spec_contains_key(map.map, key);
        aborts_if !string::spec_internal_check_utf8(str);
        let prop = simple_map::spec_get(map.map, key);
        aborts_if prop.type != spec_utf8(str);
        aborts_if !aptos_std::from_bcs::deserializable<u128>(prop.value);
    }

    spec read_bool(map: &PropertyMap, key: &String): bool {
        use std::string;
        use aptos_framework::simple_map;

        let str = b"bool";
        aborts_if !simple_map::spec_contains_key(map.map, key);
        aborts_if !string::spec_internal_check_utf8(str);
        let prop = simple_map::spec_get(map.map, key);
        aborts_if prop.type != spec_utf8(str);
        aborts_if !aptos_std::from_bcs::deserializable<bool>(prop.value);
    }

    spec borrow_value(property: &PropertyValue): vector<u8> {
        aborts_if false;
    }

    spec borrow_type(property: &PropertyValue): String {
        aborts_if false;
    }

    spec remove (
        map: &mut PropertyMap,
        key: &String
    ): (String, PropertyValue) {
        aborts_if !simple_map::spec_contains_key(map.map, key);
    }

    spec update_property_map (
        map: &mut PropertyMap,
        keys: vector<String>,
        values: vector<vector<u8>>,
        types: vector<String>,
    ) {
        // TODO: Can't handle abort in loop.
        pragma aborts_if_is_partial;
        let key_len = len(keys);
        let val_len = len(values);
        let typ_len = len(types);
        aborts_if !(key_len == val_len);
        aborts_if !(key_len == typ_len);
    }

    spec update_property_value (
        map: &mut PropertyMap,
        key: &String,
        value: PropertyValue
    ) {
        aborts_if !simple_map::spec_contains_key(map.map, key);
    }

    spec create_property_value_raw (
        value: vector<u8>,
        type: String
    ): PropertyValue {
        aborts_if false;
    }

    /// Abort according to the code
    spec create_property_value<T: copy>(data: &T): PropertyValue {
        use aptos_std::type_info::{type_name};

        let name = type_name<T>();
        aborts_if !string::spec_internal_check_utf8(b"bool");

        aborts_if name != spec_utf8(b"bool") &&
            !string::spec_internal_check_utf8(b"u8");

        aborts_if name != spec_utf8(b"bool") &&
            name != spec_utf8(b"u8") &&
            !string::spec_internal_check_utf8(b"u64");

        aborts_if name != spec_utf8(b"bool") &&
            name != spec_utf8(b"u8") &&
            name != spec_utf8(b"u64") &&
            !string::spec_internal_check_utf8(b"u128");

        aborts_if name != spec_utf8(b"bool") &&
            name != spec_utf8(b"u8") &&
            name != spec_utf8(b"u64") &&
            name != spec_utf8(b"u128") &&
            !string::spec_internal_check_utf8(b"address");

        aborts_if name != spec_utf8(b"bool") &&
            name != spec_utf8(b"u8") &&
            name != spec_utf8(b"u64") &&
            name != spec_utf8(b"u128") &&
            name != spec_utf8(b"address") &&
            !string::spec_internal_check_utf8(b"0x1::string::String");

        aborts_if name != spec_utf8(b"bool") &&
            name != spec_utf8(b"u8") &&
            name != spec_utf8(b"u64") &&
            name != spec_utf8(b"u128") &&
            name != spec_utf8(b"address") &&
            name != spec_utf8(b"0x1::string::String") &&
            !string::spec_internal_check_utf8(b"vector<u8>");
    }
}
