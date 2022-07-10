/// PropertyMap is a specialization of SimpleMap for Tokens.
/// It maps a String key to a PropertyValue that consists of a String type and a vec value
module AptosFramework::PropertyMap {
    use Std::Vector;
    use Std::ASCII::{Self, String};
    use AptosFramework::SimpleMap::{Self, SimpleMap};

    const MAX_PROPERTY_MAP_SIZE: u64 = 1000;
    const EKEY_AREADY_EXIST_IN_PROPERTY_MAP: u64 = 1;
    const EPROPERTY_NUMBER_EXCEED_LIMIT: u64 = 2;
    const EPROPERTY_NOT_EXIST: u64 = 3;
    const EKEY_COUNT_NOT_MATCH_VALUE_COUNT: u64 = 4;
    const EKEY_COUNT_NOT_MATCH_TYPE_COUNT: u64 = 5;

    struct PropertyMap has store {
        map: SimpleMap<String, PropertyValue>
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
        assert!(Vector::length(&keys) == Vector::length(&values), EKEY_COUNT_NOT_MATCH_VALUE_COUNT);
        assert!(Vector::length(&keys) == Vector::length(&types), EKEY_COUNT_NOT_MATCH_TYPE_COUNT);
        let properties = PropertyMap{
            map: SimpleMap::create<String, PropertyValue>()
        };
        let i = 0;
        while (i < Vector::length(&keys)) {
            SimpleMap::add(
                &mut properties.map,
                *Vector::borrow(&keys, i),
                PropertyValue{ value: *Vector::borrow(&values, i), type: *Vector::borrow(&types, i) }
            );
            i = i + 1;
        };
        properties
    }

    public fun contains_key(list: &PropertyMap, key: &String): bool {
        SimpleMap::contains_key(&list.map, key)
    }

    public fun add(list: &mut PropertyMap, key: String, value: PropertyValue) {
        assert!(! SimpleMap::contains_key(&list.map, &key), EKEY_AREADY_EXIST_IN_PROPERTY_MAP);
        assert!(SimpleMap::length<String, PropertyValue>(&list.map) <= MAX_PROPERTY_MAP_SIZE, EPROPERTY_NUMBER_EXCEED_LIMIT);
        SimpleMap::add(&mut list.map, key, value);
    }

    public fun length(list: &PropertyMap): u64 {
        SimpleMap::length(&list.map)
    }

    public fun borrow(list: &PropertyMap, key: &String): &PropertyValue {
        let found = contains_key(list, key);
        assert!(found, EPROPERTY_NOT_EXIST);
        SimpleMap::borrow(&list.map, key)
    }

    public fun borrow_value(property: &PropertyValue): vector<u8> {
        *&property.value
    }

    public fun borrow_type(property: &PropertyValue): String {
        *&property.type
    }

    public fun update_property_value(
        list: &mut PropertyMap,
        key: &String,
        value: PropertyValue
    ) {
        let found = contains_key(list, key);
        assert!(found, EPROPERTY_NOT_EXIST);
        let property_val = SimpleMap::borrow_mut(&mut list.map, key);
        *property_val = value;
    }

    public fun remove(
        list: &mut PropertyMap,
        key: &String
    ): (String, PropertyValue) {
        let found = contains_key(list, key);
        assert!(found, EPROPERTY_NOT_EXIST);

        SimpleMap::remove(&mut list.map, key)
    }

    public fun update_property_values(
        list: &mut PropertyMap,
        keys: &vector<String>,
        values: vector<PropertyValue>
    ) {
        let i = 0;
        while (i < Vector::length(keys)) {
            let key = Vector::borrow(keys, i);
            let value = Vector::borrow(&values, i);
            update_property_value(list, key, *value);
            i = i + 1;
        };
    }

    public fun generate_string_vector(values: vector<vector<u8>>): vector<String> {
        let vals: vector<String> = Vector::empty<String>();
        let i = 0;
        while (i < Vector::length(&values)) {
            Vector::push_back(&mut vals, ASCII::string(*Vector::borrow(&mut values, i )));
            i = i + 1;
        };
        vals
    }

    #[test_only]
    fun create_property_list(): PropertyMap {
        use Std::ASCII::string;
        let keys = vector<String>[ string(b"attack"), string(b"durability"), string(b"type")];
        let values = vector<vector<u8>>[ b"10", b"5", b"weapon" ];
        let types = vector<String>[ string(b"integer"), string(b"integer"), string(b"String") ];
        new(keys, values, types)
    }

    #[test]
    fun test_add_property(): PropertyMap {
        use Std::ASCII::string;
        let properties = create_property_list();
        add(
            &mut properties, string(b"level"),
            PropertyValue{
                value: b"1",
                type: string(b"integer")
            });
        assert!(
            borrow(&properties, &string(b"level")).value == b"1",
            EPROPERTY_NOT_EXIST);
        properties
    }

    #[test]
    fun test_update_property(): PropertyMap {
        use Std::ASCII::string;
        let properties = create_property_list();
        update_property_value(&mut properties, &string(b"attack"), PropertyValue{ value: b"7", type: string(b"integer") });
        assert!(
            borrow(&properties, &string(b"attack")).value == b"7",
            1
        );
        properties
    }

    #[test]
    fun test_remove_property(): PropertyMap {
        use Std::ASCII::string;
        let properties = create_property_list();
        assert!(length(&mut properties) == 3, 1);
        let (_, _) = remove(&mut properties, &string(b"attack"));
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
