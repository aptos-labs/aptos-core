/// PropertyMap is a specialization of SimpleMap for Tokens.
/// It maps a String key to a PropertyValue that consists of type (string) and value (vecotr<u8>)
module aptos_token::property_map {
    use std::vector;
    use std::string::{Self, String};
    use aptos_framework::simple_map::{Self, SimpleMap};

    const MAX_PROPERTY_MAP_SIZE: u64 = 1000;
    const EKEY_AREADY_EXIST_IN_PROPERTY_MAP: u64 = 1;
    const EPROPERTY_NUMBER_EXCEED_LIMIT: u64 = 2;
    const EPROPERTY_NOT_EXIST: u64 = 3;
    const EKEY_COUNT_NOT_MATCH_VALUE_COUNT: u64 = 4;
    const EKEY_COUNT_NOT_MATCH_TYPE_COUNT: u64 = 5;

    struct PropertyMap has store {
        map: SimpleMap<String, PropertyValue>,
    }

    struct PropertyValue has store, copy, drop {
        value: vector<u8>,
        type: String,
    }

    public fun new(
        keys: vector<String>,
        values: vector<vector<u8>>,
        types: vector<String>
    ): PropertyMap {
        assert!(vector::length(&keys) == vector::length(&values), EKEY_COUNT_NOT_MATCH_VALUE_COUNT);
        assert!(vector::length(&keys) == vector::length(&types), EKEY_COUNT_NOT_MATCH_TYPE_COUNT);
        let properties = PropertyMap{
            map: simple_map::create<String, PropertyValue>(),
        };
        let i = 0;
        while (i < vector::length(&keys)) {
            simple_map::add(
                &mut properties.map,
                *vector::borrow(&keys, i),
                PropertyValue{ value: *vector::borrow(&values, i), type: *vector::borrow(&types, i) }
            );
            i = i + 1;
        };
        properties
    }

    public fun contains_key(list: &PropertyMap, key: &String): bool {
        simple_map::contains_key(&list.map, key)
    }

    public fun add(list: &mut PropertyMap, key: String, value: PropertyValue) {
        assert!(! simple_map::contains_key(&list.map, &key), EKEY_AREADY_EXIST_IN_PROPERTY_MAP);
        assert!(simple_map::length<String, PropertyValue>(&list.map) <= MAX_PROPERTY_MAP_SIZE, EPROPERTY_NUMBER_EXCEED_LIMIT);
        simple_map::add(&mut list.map, key, value);
    }

    public fun length(list: &PropertyMap): u64 {
        simple_map::length(&list.map)
    }

    public fun borrow(list: &PropertyMap, key: &String): &PropertyValue {
        let found = contains_key(list, key);
        assert!(found, EPROPERTY_NOT_EXIST);
        simple_map::borrow(&list.map, key)
    }

    public fun borrow_value(property: &PropertyValue): vector<u8> {
        *&property.value
    }

    public fun borrow_type(property: &PropertyValue): String {
        *&property.type
    }

    public fun remove(
        list: &mut PropertyMap,
        key: &String
    ): (String, PropertyValue) {
        let found = contains_key(list, key);
        assert!(found, EPROPERTY_NOT_EXIST);

        simple_map::remove(&mut list.map, key)
    }

    /// update a property map with new values
    /// assume not deleting old keys, only add or update key values
    public fun update_property_map(
        map: &mut PropertyMap,
        property_keys: vector<String>,
        property_values: vector<vector<u8>>,
        property_types: vector<String>
    ) {
        let i = 0;
        while (i < vector::length(&property_keys)) {
            let key = vector::borrow(&property_keys, i);
            let value = *vector::borrow(&property_values, i);
            let type = *vector::borrow(&property_types, i);
            if (contains_key(map, key)) {
                let pv = PropertyValue {
                    value,
                    type,
                };

                update_property_value(map, key, pv);
            };
            i = i + 1;
        };
    }

    public fun update_property_value(
        list: &mut PropertyMap,
        key: &String,
        value: PropertyValue
    ) {
        let found = contains_key(list, key);
        assert!(found, EPROPERTY_NOT_EXIST);
        let property_val = simple_map::borrow_mut(&mut list.map, key);
        *property_val = value;
    }

    public fun generate_string_vector(values: vector<vector<u8>>): vector<String> {
        let vals: vector<String> = vector::empty<String>();
        let i = 0;
        while (i < vector::length(&values)) {
            vector::push_back(&mut vals, string::utf8(*vector::borrow(&mut values, i )));
            i = i + 1;
        };
        vals
    }

    #[test_only]
    fun create_property_list(): PropertyMap {
        use std::string::utf8;
        let keys = vector<String>[ utf8(b"attack"), utf8(b"durability"), utf8(b"type")];
        let values = vector<vector<u8>>[ b"10", b"5", b"weapon" ];
        let types = vector<String>[ utf8(b"integer"), utf8(b"integer"), utf8(b"String") ];
        new(keys, values, types)
    }

    #[test]
    fun test_add_property(): PropertyMap {
        use std::string::utf8;
        let properties = create_property_list();
        add(
            &mut properties, utf8(b"level"),
            PropertyValue{
                value: b"1",
                type: utf8(b"integer")
            });
        assert!(
            borrow(&properties, &utf8(b"level")).value == b"1",
            EPROPERTY_NOT_EXIST);
        properties
    }

    #[test]
    fun test_update_property(): PropertyMap {
        use std::string::utf8;
        let properties = create_property_list();
        update_property_value(&mut properties, &utf8(b"attack"), PropertyValue{ value: b"7", type: utf8(b"integer") });
        assert!(
            borrow(&properties, &utf8(b"attack")).value == b"7",
            1
        );
        properties
    }

    #[test]
    fun test_remove_property(): PropertyMap {
        use std::string::utf8;
        let properties = create_property_list();
        assert!(length(&mut properties) == 3, 1);
        let (_, _) = remove(&mut properties, &utf8(b"attack"));
        assert!(length(&properties) == 2, 1);
        properties
    }

    #[test_only]
    public fun test_create_property_value(type: String, value: vector<u8>): PropertyValue {
        PropertyValue {
            type,
            value
        }
    }
}
