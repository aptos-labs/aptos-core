/// This Data Structure is designed to hold properties.
/// It only support
///     1. adding new property.
///     2. update the value of existing property
///     3. get the value of a property
module AptosFramework::PropertyList {
    use Std::Vector;
    use Std::ASCII::String;
    use AptosFramework::SimpleMap::{Self, SimpleMap};

    const MAX_PROPERTY_MAP_SIZE: u64 = 1000;
    const EKEY_AREADY_EXIST_IN_PROPERTY_MAP: u64 = 1;
    const EPROPERTY_NUMBER_EXCEED_LIMIT: u64 = 2;
    const EPROPERTY_NOT_EXIST: u64 = 3;
    const EKEY_COUNT_NOT_MATCH_VALUE_COUNT: u64 = 4;
    const EKEY_COUNT_NOT_MATCH_TYPE_COUNT: u64 = 5;

    struct PropertyList has store {
        map: SimpleMap<String, PropertyValue>
    }

    struct PropertyValue has store, copy, drop {
        value: vector<u8>,
        type: String,
    }

    public fun new(
        default_keys: &vector<String>,
        default_values: &vector<vector<u8>>,
        default_types: &vector<String>
    ): PropertyList {
        assert!(Vector::length(default_keys) == Vector::length(default_values), EKEY_COUNT_NOT_MATCH_VALUE_COUNT);
        assert!(Vector::length(default_keys) == Vector::length(default_types), EKEY_COUNT_NOT_MATCH_TYPE_COUNT);
        let properties = PropertyList{
            map: SimpleMap::create<String, PropertyValue>()
        };
        let i = 0;
        while (i < Vector::length(default_keys)) {
            SimpleMap::add(
                &mut properties.map,
                *Vector::borrow(default_keys, i),
                PropertyValue{ value: *Vector::borrow(default_values, i), type: *Vector::borrow(default_types, i) }
            );
            i = i + 1;
        };
        properties
    }

    public fun contains(list: &PropertyList, key: &String): bool {
        SimpleMap::contains_key(&list.map, key)
    }

    public fun add(list: &mut PropertyList, key: String, value: PropertyValue) {
        assert!(! SimpleMap::contains_key(&list.map, &key), EKEY_AREADY_EXIST_IN_PROPERTY_MAP);
        assert!(SimpleMap::length<String, PropertyValue>(&list.map) <= MAX_PROPERTY_MAP_SIZE, EPROPERTY_NUMBER_EXCEED_LIMIT);
        SimpleMap::add(&mut list.map, key, value);
    }

    public fun size(list: &PropertyList): u64 {
        SimpleMap::length(&list.map)
    }

    public fun borrow(list: &PropertyList, key: &String): &PropertyValue {
        let found = contains(list, key);
        assert!(found, EPROPERTY_NOT_EXIST);
        SimpleMap::borrow(&list.map, key)
    }

    public fun get_value(list: &PropertyList, key: &String): vector<u8> {
        let found = contains(list, key);
        assert!(found, EPROPERTY_NOT_EXIST);
        SimpleMap::borrow(&list.map, key).value
    }

    public fun get_type(list: &PropertyList, key: &String, ): String {
        let found = contains(list, key);
        assert!(found, EPROPERTY_NOT_EXIST);
        SimpleMap::borrow(&list.map, key).type
    }

    public fun get_property_values(
        list: &PropertyList,
        keys: vector<String>
    ): vector<PropertyValue> {
        let res: vector<PropertyValue> = Vector::empty<PropertyValue>();
        let i = 0;
        while (i < Vector::length(&keys)) {
            let key = Vector::borrow(&keys, i);
            let value = borrow(list, key);
            Vector::push_back(&mut res, *value);
            i = i + 1;
        };
        res
    }

    public fun update_property_value(
        list: &mut PropertyList,
        key: &String,
        value: PropertyValue
    ) {
        let found = contains(list, key);
        assert!(found, EPROPERTY_NOT_EXIST);
        let property_val = SimpleMap::borrow_mut(&mut list.map, key);
        *property_val = value;
    }

    public fun remove_property(
        list: &mut PropertyList,
        key: &String
    ): (String, PropertyValue) {
        let found = contains(list, key);
        assert!(found, EPROPERTY_NOT_EXIST);

        SimpleMap::remove(&mut list.map, key)
    }

    public fun update_property_values(
        list: &mut PropertyList,
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

    #[test_only]
    fun create_property_list(): PropertyList {
        use Std::ASCII::string;
        let keys = vector<String>
        [
        string(b"attack"),
        string(b"durability"),
        string(b"type"),
        ];
        let values = vector<vector<u8>>[ b"10", b"5", b"weapon" ];
        let types = vector<String>[ string(b"integer"), string(b"integer"), string(b"String") ];
        new(&keys, &values, &types)
    }

    #[test]
    fun test_add_property(): PropertyList {
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
    fun test_update_property(): PropertyList {
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
    fun test_remove_property(): PropertyList {
        use Std::ASCII::string;
        let properties = create_property_list();
        assert!(size(&mut properties) == 3, 1);
        let (_, _) = remove_property(&mut properties, &string(b"attack"));
        assert!(size(&properties) == 2, 1);
        properties
    }
}
