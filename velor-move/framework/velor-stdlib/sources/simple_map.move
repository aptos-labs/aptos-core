/// This module provides a solution for unsorted maps, that is it has the properties that
/// 1) Keys point to Values
/// 2) Each Key must be unique
/// 3) A Key can be found within O(N) time
/// 4) The keys are unsorted.
/// 5) Adds and removals take O(N) time
///
/// DEPRECATED: since it's implementation is inneficient, it
/// has been deprecated in favor of `ordered_map.move`.
module velor_std::simple_map {
    use std::error;
    use std::option;
    use std::vector;

    /// Map key already exists
    const EKEY_ALREADY_EXISTS: u64 = 1;
    /// Map key is not found
    const EKEY_NOT_FOUND: u64 = 2;

    /// DEPRECATED: since it's implementation is inneficient, it
    /// has been deprecated in favor of `ordered_map.move`.
    struct SimpleMap<Key, Value> has copy, drop, store {
        data: vector<Element<Key, Value>>,
    }

    struct Element<Key, Value> has copy, drop, store {
        key: Key,
        value: Value,
    }

    public fun length<Key: store, Value: store>(self: &SimpleMap<Key, Value>): u64 {
        self.data.length()
    }

    /// Create an empty SimpleMap.
    public fun new<Key: store, Value: store>(): SimpleMap<Key, Value> {
        SimpleMap {
            data: vector::empty(),
        }
    }

    /// Create a SimpleMap from a vector of keys and values. The keys must be unique.
    public fun new_from<Key: store, Value: store>(
        keys: vector<Key>,
        values: vector<Value>,
    ): SimpleMap<Key, Value> {
        let map = new();
        map.add_all(keys, values);
        map
    }

    #[deprecated]
    /// Create an empty SimpleMap.
    /// This function is deprecated, use `new` instead.
    public fun create<Key: store, Value: store>(): SimpleMap<Key, Value> {
        new()
    }

    public fun borrow<Key: store, Value: store>(
        self: &SimpleMap<Key, Value>,
        key: &Key,
    ): &Value {
        let maybe_idx = self.find(key);
        assert!(maybe_idx.is_some(), error::invalid_argument(EKEY_NOT_FOUND));
        let idx = maybe_idx.extract();
        &self.data.borrow(idx).value
    }

    public fun borrow_mut<Key: store, Value: store>(
        self: &mut SimpleMap<Key, Value>,
        key: &Key,
    ): &mut Value {
        let maybe_idx = self.find(key);
        assert!(maybe_idx.is_some(), error::invalid_argument(EKEY_NOT_FOUND));
        let idx = maybe_idx.extract();
        &mut self.data.borrow_mut(idx).value
    }

    public fun contains_key<Key: store, Value: store>(
        self: &SimpleMap<Key, Value>,
        key: &Key,
    ): bool {
        let maybe_idx = self.find(key);
        maybe_idx.is_some()
    }

    public fun destroy_empty<Key: store, Value: store>(self: SimpleMap<Key, Value>) {
        let SimpleMap { data } = self;
        data.destroy_empty();
    }

    /// Add a key/value pair to the map. The key must not already exist.
    public fun add<Key: store, Value: store>(
        self: &mut SimpleMap<Key, Value>,
        key: Key,
        value: Value,
    ) {
        let maybe_idx = self.find(&key);
        assert!(maybe_idx.is_none(), error::invalid_argument(EKEY_ALREADY_EXISTS));

        self.data.push_back(Element { key, value });
    }

    /// Add multiple key/value pairs to the map. The keys must not already exist.
    public fun add_all<Key: store, Value: store>(
        self: &mut SimpleMap<Key, Value>,
        keys: vector<Key>,
        values: vector<Value>,
    ) {
        keys.zip(values, |key, value| {
            self.add(key, value);
        });
    }

    /// Insert key/value pair or update an existing key to a new value
    public fun upsert<Key: store, Value: store>(
        self: &mut SimpleMap<Key, Value>,
        key: Key,
        value: Value
    ): (std::option::Option<Key>, std::option::Option<Value>) {
        let data = &mut self.data;
        let len = data.length();
        for (i in 0..len) {
            let element = data.borrow(i);
            if (&element.key == &key) {
                data.push_back(Element { key, value });
                data.swap(i, len);
                let Element { key, value } = data.pop_back();
                return (std::option::some(key), std::option::some(value))
            };
        };
        self.data.push_back(Element { key, value });
        (std::option::none(), std::option::none())
    }

    /// Return all keys in the map. This requires keys to be copyable.
    public fun keys<Key: copy, Value>(self: &SimpleMap<Key, Value>): vector<Key> {
        self.data.map_ref(|e| {
            e.key
        })
    }

    /// Return all values in the map. This requires values to be copyable.
    public fun values<Key, Value: copy>(self: &SimpleMap<Key, Value>): vector<Value> {
        self.data.map_ref(|e| {
            e.value
        })
    }

    /// Transform the map into two vectors with the keys and values respectively
    /// Primarily used to destroy a map
    public fun to_vec_pair<Key: store, Value: store>(
        self: SimpleMap<Key, Value>): (vector<Key>, vector<Value>) {
        let keys: vector<Key> = vector::empty();
        let values: vector<Value> = vector::empty();
        let SimpleMap { data } = self;
        data.for_each(|e| {
            let Element { key, value } = e;
            keys.push_back(key);
            values.push_back(value);
        });
        (keys, values)
    }

    /// For maps that cannot be dropped this is a utility to destroy them
    /// using lambdas to destroy the individual keys and values.
    public inline fun destroy<Key: store, Value: store>(
        self: SimpleMap<Key, Value>,
        dk: |Key|,
        dv: |Value|
    ) {
        let (keys, values) = self.to_vec_pair();
        keys.destroy(|_k| dk(_k));
        values.destroy(|_v| dv(_v));
    }

    /// Remove a key/value pair from the map. The key must exist.
    public fun remove<Key: store, Value: store>(
        self: &mut SimpleMap<Key, Value>,
        key: &Key,
    ): (Key, Value) {
        let maybe_idx = self.find(key);
        assert!(maybe_idx.is_some(), error::invalid_argument(EKEY_NOT_FOUND));
        let placement = maybe_idx.extract();
        let Element { key, value } = self.data.swap_remove(placement);
        (key, value)
    }

    fun find<Key: store, Value: store>(
        self: &SimpleMap<Key, Value>,
        key: &Key,
    ): option::Option<u64> {
        let len = self.data.length();
        for (i in 0..len) {
            let element = self.data.borrow(i);
            if (&element.key == key) {
                return option::some(i)
            };
        };
        option::none<u64>()
    }

    #[test]
    public fun test_add_remove_many() {
        let map = create<u64, u64>();

        assert!(map.length() == 0, 0);
        assert!(!map.contains_key(&3), 1);
        map.add(3, 1);
        assert!(map.length() == 1, 2);
        assert!(map.contains_key(&3), 3);
        assert!(map.borrow(&3) == &1, 4);
        *map.borrow_mut(&3) = 2;
        assert!(map.borrow(&3) == &2, 5);

        assert!(!map.contains_key(&2), 6);
        map.add(2, 5);
        assert!(map.length() == 2, 7);
        assert!(map.contains_key(&2), 8);
        assert!(map.borrow(&2) == &5, 9);
        *map.borrow_mut(&2) = 9;
        assert!(map.borrow(&2) == &9, 10);

        map.remove(&2);
        assert!(map.length() == 1, 11);
        assert!(!map.contains_key(&2), 12);
        assert!(map.borrow(&3) == &2, 13);

        map.remove(&3);
        assert!(map.length() == 0, 14);
        assert!(!map.contains_key(&3), 15);

        map.destroy_empty();
    }

    #[test]
    public fun test_add_all() {
        let map = create<u64, u64>();

        assert!(map.length() == 0, 0);
        map.add_all(vector[1, 2, 3], vector[10, 20, 30]);
        assert!(map.length() == 3, 1);
        assert!(map.borrow(&1) == &10, 2);
        assert!(map.borrow(&2) == &20, 3);
        assert!(map.borrow(&3) == &30, 4);

        map.remove(&1);
        map.remove(&2);
        map.remove(&3);
        map.destroy_empty();
    }

    #[test]
    public fun test_keys() {
        let map = create<u64, u64>();
        assert!(map.keys() == vector[], 0);
        map.add(2, 1);
        map.add(3, 1);

        assert!(map.keys() == vector[2, 3], 0);
    }

    #[test]
    public fun test_values() {
        let map = create<u64, u64>();
        assert!(map.values() == vector[], 0);
        map.add(2, 1);
        map.add(3, 2);

        assert!(map.values() == vector[1, 2], 0);
    }

    #[test]
    #[expected_failure]
    public fun test_add_twice() {
        let map = create<u64, u64>();
        map.add(3, 1);
        map.add(3, 1);

        map.remove(&3);
        map.destroy_empty();
    }

    #[test]
    #[expected_failure]
    public fun test_remove_twice() {
        let map = create<u64, u64>();
        map.add(3, 1);
        map.remove(&3);
        map.remove(&3);

        map.destroy_empty();
    }

    #[test]
    public fun test_upsert_test() {
        let map = create<u64, u64>();
        // test adding 3 elements using upsert
        map.upsert::<u64, u64>(1, 1);
        map.upsert(2, 2);
        map.upsert(3, 3);

        assert!(map.length() == 3, 0);
        assert!(map.contains_key(&1), 1);
        assert!(map.contains_key(&2), 2);
        assert!(map.contains_key(&3), 3);
        assert!(map.borrow(&1) == &1, 4);
        assert!(map.borrow(&2) == &2, 5);
        assert!(map.borrow(&3) == &3, 6);

        // change mapping 1->1 to 1->4
        map.upsert(1, 4);

        assert!(map.length() == 3, 7);
        assert!(map.contains_key(&1), 8);
        assert!(map.borrow(&1) == &4, 9);
    }
}
