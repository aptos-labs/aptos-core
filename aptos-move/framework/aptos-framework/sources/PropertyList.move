/// This Data Structure is designed to hold properties.
/// It only support
///     1. adding new property.
///     2. update the value of existing property
///     3. get the value of a property
module AptosFramework::PropertyList {
    use Std::Vector;
    use Std::ASCII;

    const MAX_PROPERTY_MAP_SIZE: u64 = 1000;
    const EKEY_AREADY_EXIST_IN_PROPERTY_MAP: u64 = 1;
    const EPROPERTY_NUMBER_EXCEED_LIMIT: u64 = 2;
    const EPROPERTY_NOT_EXIST: u64 = 2;
    const EKEY_COUNT_NOT_MATCH_VALUE_COUNT: u64 = 3;

    struct PropertyList<K: copy + drop + store, V: copy + drop + store> has copy, store, drop {
        keys: vector<K>,
        values: vector<V>
    }

    public fun new<K:copy + drop + store, V: copy + drop + store>(
        default_keys: &vector<K>,
        default_values: &vector<V>
    ): PropertyList<K, V> {
        assert!(Vector::length(default_keys) == Vector::length(default_values), EKEY_COUNT_NOT_MATCH_VALUE_COUNT);
        let properties = PropertyList<K, V> {
            keys: Vector::empty<K>(),
            values: Vector::empty<V>()
        };
        let i = 0;
        while (i < Vector::length(default_keys)) {
            Self::add(
                &mut properties,
                *Vector::borrow(default_keys, i),
                *Vector::borrow(default_values, i)
            );
            i  = i + 1;
        };
        properties
    }
    public fun contains<K: copy + drop + store, V: copy + drop + store>(list: &PropertyList<K, V>, key: &K): bool {
        let (found, _) = Vector::index_of(&list.keys, key);
        found
    }

    public fun add<K: copy + drop + store, V: copy + drop + store>(list: &mut PropertyList<K, V>, key: K, val: V) {
        assert!(!Vector::contains(&list.keys, &key), EKEY_AREADY_EXIST_IN_PROPERTY_MAP);
        assert!(Self::size(list) <= MAX_PROPERTY_MAP_SIZE, EPROPERTY_NUMBER_EXCEED_LIMIT);
        Vector::push_back(&mut list.keys, key);
        Vector::push_back(&mut list.values, val);
    }

    public fun size<K: copy + drop + store, V: copy + drop + store>(map: &PropertyList<K, V>): u64 {
        Vector::length(&map.keys)
    }

    public fun get<K: copy + drop + store, V:  copy + drop + store>(list: &PropertyList<K, V>, key: &K,): V {
        let (found, index) = Vector::index_of(&list.keys, key);
        assert!(found, EPROPERTY_NOT_EXIST);
        *Vector::borrow(&list.values, index)
    }

    public fun get_property_values<K: copy + drop + store, V:  copy + drop + store>(
        list: &PropertyList<K, V>, keys: vector<K>): vector<V> {
        let res: vector<V> = Vector::empty<V>();
        let i = 0;
        while (i < Vector::length(&keys)) {
            let (found, index) = Vector::index_of(&list.keys, Vector::borrow( &keys, i));
            assert!(found, EPROPERTY_NOT_EXIST);
            Vector::push_back(&mut res, *Vector::borrow(&list.values, index));
            i = i + 1;
        };
        res
    }

    public fun update_property_value<K: copy + drop + store, V:  copy + drop + store>(
        list: &mut PropertyList<K, V>, key: &K, value: V ) {
        let (found, index) = Vector::index_of(&list.keys, key);
        assert!(found, EPROPERTY_NOT_EXIST);
        let val = Vector::borrow_mut(&mut list.values, index);
        *val = value;
    }

    public fun remove_property<K: copy + drop + store, V:  copy + drop + store >(
        list: &mut PropertyList<K, V>,
        key: &K ) {
        let (found, index) = Vector::index_of(&list.keys, key);
        assert!(found, EPROPERTY_NOT_EXIST);

        Vector::swap_remove(&mut list.keys, index);
        Vector::swap_remove(&mut list.values, index);
    }

    public fun update_property_values<K: copy + drop + store, V:  copy + drop + store>(
        list: &mut PropertyList<K, V>, keys: &vector<K>, values: vector<V>){
        let i = 0;
        while (i < Vector::length(keys)) {
            let key = Vector::borrow(keys, i);
            let value = Vector::borrow(&values, i);
            update_property_value(list, key, *value);
            i = i + 1;
        };
    }

    #[test_only]
    fun create_property_list(): PropertyList<ASCII::String, ASCII::String>{
        let keys = vector<ASCII::String>[
            ASCII::string(b"attack"),
            ASCII::string(b"durability")
        ];
        let values = vector<ASCII::String>[ASCII::string(b"10"), ASCII::string(b"5")];
        new<ASCII::String, ASCII::String>(&keys, &values)
    }

    #[test]
    fun test_add_property(){
        let properties = create_property_list();
        add<ASCII::String, ASCII::String>(
            &mut properties, ASCII::string(b"level"),
            ASCII::string(b"1"));
        assert!(
            get(&properties, &ASCII::string(b"level")) == ASCII::string(b"1"),
            EPROPERTY_NOT_EXIST );
    }

    #[test]
    fun test_update_property(){
        let properties = create_property_list();
        update_property_value(&mut properties, &ASCII::string(b"attack"), ASCII::string(b"7"));
        assert!(
            get(&properties, &ASCII::string(b"attack")) == ASCII::string(b"7"),
            1
        );
    }

    #[test]
    fun test_remove_property(){
        let properties = create_property_list();
        assert!(size(&mut properties) == 2, 1);
        remove_property(&mut properties, &ASCII::string(b"attack"));
        assert!(size(&properties) == 1, 1);
    }

}