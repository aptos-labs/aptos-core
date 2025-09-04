spec velor_token::property_map {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;

        let MAX_PROPERTY_MAP_SIZE = 1000;
        let MAX_PROPERTY_NAME_LENGTH = 128;
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
        aborts_if !(length == len(values));
        aborts_if !(length == len(types));
    }

    spec new_with_key_and_property_value (
        keys: vector<String>,
        values: vector<PropertyValue>
    ): PropertyMap {
        // TODO: Can't handle abort in loop.
        pragma aborts_if_is_partial;
        let length = len(keys);
        aborts_if !(length <= MAX_PROPERTY_MAP_SIZE);
        aborts_if !(length == len(values));
    }

    spec empty(): PropertyMap {
        aborts_if false;
    }

    spec contains_key(self: &PropertyMap, key: &String): bool {
        aborts_if false;
    }

    spec add(self: &mut PropertyMap, key: String, value: PropertyValue) {
        use velor_framework::simple_map;

        aborts_if !(key.length() <= MAX_PROPERTY_NAME_LENGTH);
        aborts_if !(!simple_map::spec_contains_key(self.map, key));
        aborts_if !(simple_map::spec_len(self.map) < MAX_PROPERTY_MAP_SIZE);
    }

    spec length(self: &PropertyMap): u64 {
        aborts_if false;
    }

    spec keys(self: &PropertyMap): vector<String> {
        pragma verify = false;
    }

    spec types(self: &PropertyMap): vector<String> {
        pragma verify = false;
    }

    spec values(self: &PropertyMap): vector<vector<u8>> {
        pragma verify = false;
    }

    spec borrow(self: &PropertyMap, key: &String): &PropertyValue {
        use velor_framework::simple_map;
        aborts_if !simple_map::spec_contains_key(self.map, key);
    }

    /// Check utf8 for correctness and whether equal
    /// to `prop.type`
    spec read_string(self: &PropertyMap, key: &String): String {
        use std::string;
        use velor_framework::simple_map;
        pragma aborts_if_is_partial;

        // TODO: Unable to handle abort from `from_bcs::to_string` because there is a function call at assert.
        aborts_if !simple_map::spec_contains_key(self.map, key);
        aborts_if !string::spec_internal_check_utf8(b"0x1::string::String");
        let prop = simple_map::spec_get(self.map, key);
        aborts_if prop.type != spec_utf8(b"0x1::string::String");
        aborts_if !velor_std::from_bcs::deserializable<String>(prop.value);
    }

    spec fun spec_utf8(bytes: vector<u8>): String {
        String { bytes }
    }

    spec read_u8(self: &PropertyMap, key: &String): u8 {
        use std::string;
        use velor_framework::simple_map;

        let str = b"u8";
        aborts_if !simple_map::spec_contains_key(self.map, key);
        aborts_if !string::spec_internal_check_utf8(str);
        let prop = simple_map::spec_get(self.map, key);
        aborts_if prop.type != spec_utf8(str);
        aborts_if !velor_std::from_bcs::deserializable<u8>(prop.value);
    }

    spec read_u64(self: &PropertyMap, key: &String): u64 {
        use std::string;
        use velor_framework::simple_map;

        let str = b"u64";
        aborts_if !simple_map::spec_contains_key(self.map, key);
        aborts_if !string::spec_internal_check_utf8(str);
        let prop = simple_map::spec_get(self.map, key);
        aborts_if prop.type != spec_utf8(str);
        aborts_if !velor_std::from_bcs::deserializable<u64>(prop.value);
    }

    spec read_address(self: &PropertyMap, key: &String): address {
        use std::string;
        use velor_framework::simple_map;

        let str = b"address";
        aborts_if !simple_map::spec_contains_key(self.map, key);
        aborts_if !string::spec_internal_check_utf8(str);
        let prop = simple_map::spec_get(self.map, key);
        aborts_if prop.type != spec_utf8(str);
        aborts_if !velor_std::from_bcs::deserializable<address>(prop.value);
    }

    spec read_u128(self: &PropertyMap, key: &String): u128 {
        use std::string;
        use velor_framework::simple_map;

        let str = b"u128";
        aborts_if !simple_map::spec_contains_key(self.map, key);
        aborts_if !string::spec_internal_check_utf8(str);
        let prop = simple_map::spec_get(self.map, key);
        aborts_if prop.type != spec_utf8(str);
        aborts_if !velor_std::from_bcs::deserializable<u128>(prop.value);
    }

    spec read_bool(self: &PropertyMap, key: &String): bool {
        use std::string;
        use velor_framework::simple_map;

        let str = b"bool";
        aborts_if !simple_map::spec_contains_key(self.map, key);
        aborts_if !string::spec_internal_check_utf8(str);
        let prop = simple_map::spec_get(self.map, key);
        aborts_if prop.type != spec_utf8(str);
        aborts_if !velor_std::from_bcs::deserializable<bool>(prop.value);
    }

    spec borrow_value(self: &PropertyValue): vector<u8> {
        aborts_if false;
    }

    spec borrow_type(self: &PropertyValue): String {
        aborts_if false;
    }

    spec remove (
        self: &mut PropertyMap,
        key: &String
    ): (String, PropertyValue) {
        aborts_if !simple_map::spec_contains_key(self.map, key);
    }

    spec update_property_map (
        self: &mut PropertyMap,
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
        self: &mut PropertyMap,
        key: &String,
        value: PropertyValue
    ) {
        aborts_if !simple_map::spec_contains_key(self.map, key);
    }

    spec create_property_value_raw (
        value: vector<u8>,
        type: String
    ): PropertyValue {
        aborts_if false;
    }

    /// Abort according to the code
    spec create_property_value<T: copy>(data: &T): PropertyValue {
        use velor_std::type_info::{type_name};

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
