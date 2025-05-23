/// This module provides an implementation of an enumerable map, a data structure that maintains key-value pairs with
/// efficient operations for addition, removal, and retrieval.
/// It allows for enumeration of keys in insertion order, bulk operations, and updates while ensuring data consistency.
/// The module includes error handling and a suite of test functions for validation.
module supra_std::enumerable_map {
    use std::error;
    use std::vector;
    use aptos_std::table;

    /// Key is already present in the map
    const EKEY_ALREADY_ADDED: u64 = 1;
    /// Key is absent in the map
    const EKEY_ABSENT: u64 = 2;
    /// Vector is empty
    const EVECTOR_EMPTY: u64 = 3;

    /// Enumerable Map to store the key value pairs
    struct EnumerableMap<K: copy + drop, V: store+drop+copy> has store {
        /// List of all keys
        list: vector<K>,
        /// Key mapped to a tuple containing the (position of key in list and value corresponding to the key)
        map: table::Table<K, Tuple<V>>,
    }

    /// Tuple to store the position of key in list and value corresponding to the key
    struct Tuple<V: store+drop+copy> has store, copy, drop {
        position: u64,
        value: V,
    }

    /// Return type
    struct KeyValue<K: copy + drop, V: store+drop+copy> has store, copy, drop {
        key: K,
        value: V,
    }

    /// To create an empty enum map
    public fun new_map<K: copy + drop, V: store + drop + copy>(): EnumerableMap<K, V> {
        return EnumerableMap<K, V> { list: vector::empty<K>(), map: table::new<K, Tuple<V>>() }
    }


    /// Add Single Key in the Enumerable Map
    public fun add_value<K: copy+drop, V: store+drop+copy>(map: &mut EnumerableMap<K, V>, key: K, value: V) {
        assert!(!contains(map, key), error::already_exists(EKEY_ALREADY_ADDED));
        table::add(&mut map.map, key, Tuple<V> { position: vector::length(&map.list), value });
        vector::push_back(&mut map.list, key);
    }

    /// Add Multiple Keys in the Enumerable Map
    public fun add_value_bulk<K: copy+drop, V: store+drop+copy>(
        map: &mut EnumerableMap<K, V>,
        keys: vector<K>,
        values: vector<V>
    ): vector<K> {
        assert!(!vector::is_empty(&values), error::invalid_argument(EVECTOR_EMPTY));
        let current_key_list_length = vector::length(&map.list);
        let updated_keys = vector::empty<K>();

        vector::zip_reverse(keys, values, |key, value| {
            if (!contains(map, key)) {
                table::add(&mut map.map, key, Tuple<V> { position: current_key_list_length, value });
                vector::push_back(&mut map.list, key);
                current_key_list_length = current_key_list_length + 1;

                vector::push_back(&mut updated_keys, key);
            };
        });

        return updated_keys
    }

    /// Update the value of a key thats already present in the Enumerable Map and return old value
    public fun update_value<K: copy+drop, V: store+drop+copy>(
        map: &mut EnumerableMap<K, V>,
        key: K,
        new_value: V
    ): V {
        assert!(contains(map, key), error::not_found(EKEY_ABSENT));
        let old_value = table::borrow(&mut map.map, key).value;
        table::borrow_mut(&mut map.map, key).value = new_value;
        old_value
    }

    /// Remove single Key from the Enumerable Map
    public fun remove_value<K: copy+drop, V: store+drop+copy>(map: &mut EnumerableMap<K, V>, key: K): V {
        assert!(contains(map, key), error::not_found(EKEY_ABSENT));

        let map_last_index = vector::length(&map.list) - 1;
        let index_of_element = table::borrow(&map.map, key).position;
        let tuple_to_modify = table::borrow_mut(&mut map.map, *vector::borrow(&map.list, map_last_index));

        vector::swap(&mut map.list, index_of_element, map_last_index);
        tuple_to_modify.position = index_of_element;
        vector::pop_back(&mut map.list);
        table::remove(&mut map.map, key).value
    }

    /// Remove Multiple Keys from the Enumerable Map
    public fun remove_value_bulk<K: copy+drop, V: store+drop+copy>(
        map: &mut EnumerableMap<K, V>,
        keys: vector<K>
    ): vector<K> {
        assert!(!vector::is_empty(&keys), error::invalid_argument(EVECTOR_EMPTY));

        let removed_keys = vector::empty<K>();

        vector::for_each_reverse(keys, |key| {
            if (contains(map, key)) {
                remove_value(map, key);
                vector::push_back(&mut removed_keys, key);
            };
        });

        return removed_keys
    }

    /// Will clear the entire data from the Enumerable Map
    public fun clear<K: copy+drop, V: store+drop+copy>(map: &mut EnumerableMap<K, V>) {
        let list = get_map_list(map);
        if (vector::is_empty(&list)) {
            return
        };
        remove_value_bulk(map, list);
    }

    /// Returns the value of a key that is present in Enumerable Map
    public fun get_value<K: copy+drop, V: store+drop+copy>(map: & EnumerableMap<K, V>, key: K): V {
        table::borrow(&map.map, key).value
    }

    /// Returns reference to the value of a key that is present in Enumerable Map
    public fun get_value_ref<K: copy+drop, V: store+drop+copy>(map: & EnumerableMap<K, V>, key: K): &V {
        &table::borrow(&map.map, key).value
    }

    /// Retrieves the key at the specified index from the EnumerableMap's key list.
    public fun get_key_by_index<K: copy+drop, V: store+drop+copy>(set: &EnumerableMap<K, V>, index: u64): K {
        *vector::borrow(&set.list, index)
    }

    /// Returns the value of a key that is present in Enumerable Map
    public fun get_value_mut<K: copy+drop, V: store+drop+copy>(map: &mut EnumerableMap<K, V>, key: K): &mut V {
        &mut table::borrow_mut(&mut map.map, key).value
    }

    /// Returns the list of keys that the Enumerable Map contains
    public fun get_map_list<K: copy+drop, V: store+drop+copy>(map: &EnumerableMap<K, V>): vector<K> {
        return map.list
    }

    /// Check whether Key is present into the Enumerable map or not
    public fun contains<K: copy+drop, V: store+drop+copy>(map: &EnumerableMap<K, V>, key: K): bool {
        table::contains(&map.map, key)
    }

    /// Return current length of the EnumerableSetRing
    public fun length<K: copy+drop, V: store+drop+copy>(set: &EnumerableMap<K, V>): u64 {
        return vector::length(&set.list)
    }

    /// Apply the function to each element in the EnumerableMap.
    public inline fun for_each_value<K: copy+drop, V: store+drop+copy>(set: &EnumerableMap<K, V>, f: |V|) {
        let i = 0;
        let len = length(set);
        while (i < len) {
            let key = get_key_by_index(set, i);
            f(*get_value_ref(set, key));
            i = i + 1
        }
    }

    /// Apply the function to a reference of each element in the EnumerableMap.
    public inline fun for_each_value_ref<K: copy+drop, V: store+drop+copy>(set: &EnumerableMap<K, V>, f: |&V|) {
        let i = 0;
        let len = length(set);
        while (i < len) {
            let key = get_key_by_index(set, i);
            f(get_value_ref(set, key));
            i = i + 1
        }
    }

    /// Apply the function to a mutable reference in the EnumerableMap.
    public inline fun for_each_value_mut<K: copy+drop, V: store+drop+copy>(set: &mut EnumerableMap<K, V>, f: |&mut V|) {
        let i = 0;
        let len = length(set);
        while (i < len) {
            let key = get_key_by_index(set, i);
            f(get_value_mut(set, key));
            i = i + 1
        }
    }

    /// Iterates over each key-value pair in an EnumerableMap and applies the provided function
    public inline fun for_each_keyval<K: copy+drop, V: store+drop+copy>(set: &EnumerableMap<K, V>, f: |K, V|) {
        let i = 0;
        let len = length(set);
        while (i < len) {
            let key = get_key_by_index(set, i);
            f(key, *get_value_ref(set, key));
            i = i + 1
        }
    }

    /// Filter the enumerableMap using the boolean function, removing all elements for which `p(v)` is not true.
    public inline fun filter<K: copy+drop, V: store+drop+copy>(set: &EnumerableMap<K, V>, p: |&V|bool): vector<V> {
        let result = vector<V>[];
        for_each_value_ref(set, |v| {
            if (p(v)) vector::push_back(&mut result, *v);
        });
        result
    }

    /// Transforms values in an EnumerableMap using the provided function and returns a vector of results.
    public inline fun map<K: copy+drop, V: store+drop+copy, T>(set: &EnumerableMap<K, V>, f: |V|T): vector<T> {
        let result = vector<T>[];
        for_each_value(set, |elem| vector::push_back(&mut result, f(elem)));
        result
    }

    /// Transforms values in an EnumerableMap by reference using the provided function and returns a vector of results.
    public inline fun map_ref<K: copy+drop, V: store+drop+copy, T>(set: &EnumerableMap<K, V>, f: |&V|T): vector<T> {
        let result = vector<T>[];
        for_each_value_ref(set, |elem| vector::push_back(&mut result, f(elem)));
        result
    }

    /// Applies a filter and transformation function to values in an EnumerableMap, returning a vector of results.
    public inline fun filter_map<K: copy+drop, V: store+drop+copy, T>(
        set: &EnumerableMap<K, V>,
        f: |V| (bool, T)
    ): vector<T> {
        let result = vector<T>[];
        for_each_value(set, |v| {
            let (should_include, transformed_value) = f(v);
            if (should_include) {
                vector::push_back(&mut result, transformed_value);
            }
        });
        result
    }

    /// Applies a filter and transformation function to values in an EnumerableMap, returning a vector of results.
    public inline fun filter_map_ref<K: copy+drop, V: store+drop+copy, T>(
        set: &EnumerableMap<K, V>,
        f: |&V| (bool, T)
    ): vector<T> {
        let result = vector<T>[];
        for_each_value_ref(set, |v| {
            let (should_include, transformed_value) = f(v);
            if (should_include) {
                vector::push_back(&mut result, transformed_value);
            }
        });
        result
    }

    #[test_only]
    struct EnumerableMapTest<K: copy + drop, V: store+drop+copy> has key {
        e: EnumerableMap<K, V>
    }

    #[test_only]
    fun get_enum_map(): EnumerableMap<u256, u256> {
        let enum_map = new_map<u256, u256>();
        add_value(&mut enum_map, 1, 1);
        add_value(&mut enum_map, 2, 2);
        add_value(&mut enum_map, 3, 3);
        add_value(&mut enum_map, 4, 4);
        add_value(&mut enum_map, 5, 5);
        add_value(&mut enum_map, 6, 6);
        enum_map
    }

    #[test(owner= @0x1111)]
    public fun test_add_value(owner: &signer) {
        let enum_map = get_enum_map();

        assert!(contains(&enum_map, 3), 1);
        assert!(length(&enum_map) == 6, 2);

        move_to(owner, EnumerableMapTest { e: enum_map })
    }

    #[test(owner= @0x1111)]
    public fun test_add_value_bulk(owner: &signer) {
        let enum_map = get_enum_map();

        add_value_bulk(&mut enum_map, vector[7, 8, 9], vector[7, 8, 9]);

        assert!(contains(&enum_map, 8), 1);
        assert!(length(&enum_map) == 9, 2);

        move_to(owner, EnumerableMapTest { e: enum_map });
    }

    #[test(owner= @0x1111)]
    #[expected_failure(abort_code = 3, location = Self)]
    public fun test_remove_value(owner: &signer) {
        let enum_map = get_enum_map();

        remove_value(&mut enum_map, 1);
        assert!(vector::borrow(&enum_map.list, 0) == &6, 1);
        assert!(table::borrow(&enum_map.map, 6).position == 0, 11);
        remove_value(&mut enum_map, 2);
        assert!(vector::borrow(&enum_map.list, 1) == &5, 2);
        assert!(table::borrow(&enum_map.map, 5).position == 1, 22);
        remove_value(&mut enum_map, 3);
        assert!(vector::borrow(&enum_map.list, 2) == &4, 3);
        assert!(table::borrow(&enum_map.map, 4).position == 2, 33);

        assert!(length(&enum_map) == 3, 4);
        // Check that the removed key does not exists
        assert!(contains(&enum_map, 3), 3);

        move_to(owner, EnumerableMapTest { e: enum_map })
    }

    #[test(owner= @0x1111)]
    #[expected_failure(abort_code = 2, location = Self)]
    public fun test_remove_bulk_value(owner: &signer) {
        let enum_map = get_enum_map();

        remove_value_bulk(&mut enum_map, vector[1, 2, 3]);

        assert!(contains(&enum_map, 4), 1);
        assert!(length(&enum_map) == 6, 2);

        move_to(owner, EnumerableMapTest { e: enum_map })
    }

    #[test(owner= @0x1111)]
    #[expected_failure(abort_code = 3, location = Self)]
    public fun test_update_value(owner: &signer) {
        let enum_map = get_enum_map();

        update_value(&mut enum_map, 1, 7);

        assert!(contains(&enum_map, 4), 1);
        assert!(length(&enum_map) == 6, 2);
        assert!(get_value(&enum_map, 1) == 1, 3);

        move_to(owner, EnumerableMapTest { e: enum_map })
    }

    #[test(owner= @0x1111)]
    public fun test_clear(owner: &signer) {
        let enum_map = get_enum_map();

        clear(&mut enum_map);

        assert!(length(&enum_map) == 0, 2);

        // Empty map clearing does not throw an error
        clear(&mut enum_map);

        move_to(owner, EnumerableMapTest { e: enum_map })
    }

    #[test(owner= @0x1111)]
    public fun test_for_each_value_and_ref(owner: &signer) {
        let enum_map = get_enum_map();

        let i = 1;
        for_each_value_ref(&enum_map, |v| {
            assert!(v == &i, 100);
            i = i + 1;
        });

        let j = 1;
        for_each_value_mut<u256, u256>(&mut enum_map, |v| {
            *v = j + 1; // update value with 1 increament
            j = j + 1;
        });

        let k = 1;
        for_each_value(&enum_map, |v| {
            assert!(v == k + 1, 300);
            k = k + 1;
        });

        move_to(owner, EnumerableMapTest { e: enum_map })
    }

    #[test(owner= @0x1111)]
    public fun test_filter(owner: &signer) {
        let enum_map = get_enum_map();

        let result = filter(&enum_map, |v| *v > 3);

        assert!(result == vector[4, 5, 6], 300);

        move_to(owner, EnumerableMapTest { e: enum_map })
    }

    #[test(owner= @0x1111)]
    public fun test_map_and_ref(owner: &signer) {
        let enum_map = get_enum_map();

        let result = map(&enum_map, |v| v * 3);

        assert!(result == vector[3, 6, 9, 12, 15, 18], 400);

        let result = map_ref(&enum_map, |v| *v * 2);

        assert!(result == vector[2, 4, 6, 8, 10, 12], 500);

        move_to(owner, EnumerableMapTest { e: enum_map })
    }

    #[test(owner= @0x1111)]
    public fun test_filter_map_and_ref(owner: &signer) {
        let enum_map = get_enum_map();

        let result = filter_map(&enum_map, |v|
            if (v % 2 == 0) (true, v)
            else (false, 0)
        );

        assert!(result == vector[2, 4, 6], 600);

        move_to(owner, EnumerableMapTest { e: enum_map })
    }
}
