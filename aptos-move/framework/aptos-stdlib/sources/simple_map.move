/// This module provides a solution for sorted maps, that is it has the properties that
/// 1) Keys point to Values
/// 2) Each Key must be unique
/// 3) A Key can be found within O(N) time
/// 4) The keys are unsorted.
/// 5) Adds and removals take O(N) time
module aptos_std::simple_map {
    use std::error;
    use std::option;
    use std::vector;

    /// Map key already exists
    const EKEY_ALREADY_EXISTS: u64 = 1;
    /// Map key is not found
    const EKEY_NOT_FOUND: u64 = 2;

    struct SimpleMap<Key, Value> has copy, drop, store {
        data: vector<Element<Key, Value>>,
    }

    struct Element<Key, Value> has copy, drop, store {
        key: Key,
        value: Value,
    }

    public fun length<Key: store, Value: store>(map: &SimpleMap<Key, Value>): u64 {
        vector::length(&map.data)
    }

    public fun create<Key: store, Value: store>(): SimpleMap<Key, Value> {
        SimpleMap {
            data: vector::empty(),
        }
    }

    public fun borrow<Key: store, Value: store>(
        map: &SimpleMap<Key, Value>,
        key: &Key,
    ): &Value {
        let maybe_idx = find(map, key);
        assert!(option::is_some(&maybe_idx), error::invalid_argument(EKEY_NOT_FOUND));
        let idx = option::extract(&mut maybe_idx);
        &vector::borrow(&map.data, idx).value
    }

    public fun borrow_mut<Key: store, Value: store>(
        map: &mut SimpleMap<Key, Value>,
        key: &Key,
    ): &mut Value {
        let maybe_idx = find(map, key);
        assert!(option::is_some(&maybe_idx), error::invalid_argument(EKEY_NOT_FOUND));
        let idx = option::extract(&mut maybe_idx);
        &mut vector::borrow_mut(&mut map.data, idx).value
    }

    public fun contains_key<Key: store, Value: store>(
        map: &SimpleMap<Key, Value>,
        key: &Key,
    ): bool {
        let maybe_idx = find(map, key);
        option::is_some(&maybe_idx)
    }

    public fun destroy_empty<Key: store, Value: store>(map: SimpleMap<Key, Value>) {
        let SimpleMap { data } = map;
        vector::destroy_empty(data);
    }

    public fun add<Key: store, Value: store>(
        map: &mut SimpleMap<Key, Value>,
        key: Key,
        value: Value,
    ) {
        let maybe_idx = find(map, &key);
        assert!(option::is_none(&maybe_idx), error::invalid_argument(EKEY_ALREADY_EXISTS));

        vector::push_back(&mut map.data, Element { key, value });
    }

    public fun remove<Key: store, Value: store>(
        map: &mut SimpleMap<Key, Value>,
        key: &Key,
    ): (Key, Value) {
        let maybe_idx = find(map, key);
        assert!(option::is_some(&maybe_idx), error::invalid_argument(EKEY_NOT_FOUND));
        let placement = option::extract(&mut maybe_idx);
        let Element { key, value } = vector::swap_remove(&mut map.data, placement);
        (key, value)
    }

    fun find<Key: store, Value: store>(
        map: &SimpleMap<Key, Value>,
        key: &Key,
    ): option::Option<u64>{
        let leng = vector::length(&map.data);
        let i = 0;
        while (i < leng) {
            let element = vector::borrow(&map.data, i);
            if (&element.key == key){
                return option::some(i)
            };
            i = i + 1;
        };
        option::none<u64>()
    }

    /// Apply the function to each key-value pair in the map, consuming it.
    public inline fun for_each<Key, Value>(map: SimpleMap<Key, Value>, f: |Key, Value|) {
        let SimpleMap {data} = map;
        vector::for_each(data, |elem| {
            let Element {key, value} = elem;
            f(key, value)
        })
    }

    /// Apply the function to a reference of each key-value pair in the map.
    public inline fun for_each_ref<Key, Value>(map: &SimpleMap<Key, Value>, f: |&Key, &Value|) {
        vector::for_each_ref(&map.data, |elem| {
            let e : &Element<Key, Value> = elem;
            f(&e.key, &e.value)
        })
    }

    /// Apply the function to a reference of each key-value pair in the map.
    public inline fun for_each_mut<Key, Value>(map: &mut SimpleMap<Key, Value>, f: |&Key, &mut Value|) {
        vector::for_each_mut(&mut map.data, |elem| {
            let e : &mut Element<Key, Value> = elem;
            f(&mut e.key, &mut e.value)
        })
    }

    /// Fold the function over the key-value pairs of the map.
    public inline fun fold<Accumulator, Key, Value>(
        map: SimpleMap<Key, Value>,
        init: Accumulator,
        f: |Accumulator,Key,Value|Accumulator
    ): Accumulator {
        for_each(map, |key, value| init = f(init, key, value));
        init
    }

    /// Map the function over the key-value pairs of the map.
    public inline fun map<Key, Value1, Value2>(
        map: SimpleMap<Key, Value1>,
        f: |Value1|Value2
    ): SimpleMap<Key, Value2> {
        let data = vector::empty();
        for_each(map, |key, value| vector::push_back(&mut data, Element {key, value: f(value)}));
        SimpleMap {data}
    }

    /// Map the function over the key-value pairs of the map without modifying it.
    public inline fun map_ref<Key: copy, Value1, Value2>(
        map: &SimpleMap<Key, Value1>,
        f: |&Value1|Value2
    ): SimpleMap<Key, Value2> {
        let data = vector::empty();
        for_each_ref(map, |key, value| {
            let key = *key;
            vector::push_back(&mut data, Element {key, value: f(value)});
        });
        SimpleMap {data}
    }

    /// Filter entries in the map.
    public inline fun filter<Key:drop, Value:drop>(
        map: SimpleMap<Key, Value>,
        p: |&Value|bool
    ): SimpleMap<Key, Value> {
        let data = vector::empty();
        for_each(map, |key, value| {
            if (p(&value)) {
                vector::push_back(&mut data, Element {key, value});
            }
        });
        SimpleMap {data}
    }

    /// Return true if any key-value pair in the map satisfies the predicate.
    public inline fun any<Key, Value>(
        map: &SimpleMap<Key, Value>,
        p: |&Key, &Value|bool
    ): bool {
        let SimpleMap {data} = map;
        let result = false;
        let i = 0;
        while (i < vector::length(data)) {
            let Element {key, value} = vector::borrow(data, i);
            result = p(key, value);
            if (result) {
                break
            };
            i = i + 1
        };
        result
    }

    /// Return true if all key-value pairs in the map satisfies the predicate.
    public inline fun all<Key, Value>(
        map: &SimpleMap<Key, Value>,
        p: |&Key, &Value|bool
    ): bool {
        !any(map, |k, v| !p(k, v))
    }


    #[test]
    public fun add_remove_many() {
        let map = create<u64, u64>();

        assert!(length(&map) == 0, 0);
        assert!(!contains_key(&map, &3), 1);
        add(&mut map, 3, 1);
        assert!(length(&map) == 1, 2);
        assert!(contains_key(&map, &3), 3);
        assert!(borrow(&map, &3) == &1, 4);
        *borrow_mut(&mut map, &3) = 2;
        assert!(borrow(&map, &3) == &2, 5);

        assert!(!contains_key(&map, &2), 6);
        add(&mut map, 2, 5);
        assert!(length(&map) == 2, 7);
        assert!(contains_key(&map, &2), 8);
        assert!(borrow(&map, &2) == &5, 9);
        *borrow_mut(&mut map, &2) = 9;
        assert!(borrow(&map, &2) == &9, 10);

        remove(&mut map, &2);
        assert!(length(&map) == 1, 11);
        assert!(!contains_key(&map, &2), 12);
        assert!(borrow(&map, &3) == &2, 13);

        remove(&mut map, &3);
        assert!(length(&map) == 0, 14);
        assert!(!contains_key(&map, &3), 15);

        destroy_empty(map);
    }

    #[test]
    #[expected_failure]
    public fun add_twice() {
        let map = create<u64, u64>();
        add(&mut map, 3, 1);
        add(&mut map, 3, 1);

        remove(&mut map, &3);
        destroy_empty(map);
    }

    #[test]
    #[expected_failure]
    public fun remove_twice() {
        let map = create<u64, u64>();
        add(&mut map, 3, 1);
        remove(&mut map, &3);
        remove(&mut map, &3);

        destroy_empty(map);
    }

    #[test_only]
    fun make(k1: u64, v1: u64, k2: u64, v2: u64): SimpleMap<u64, u64> {
        let m = create();
        add(&mut m, k1, v1);
        add(&mut m, k2, v2);
        m
    }

    #[test]
    fun test_for_each() {
        let m = make(1, 4, 2, 5);
        let s = 0;
        for_each(m, |x, y| {
            s = s + x + y;
        });
        assert!(s == 12, 0)
    }

    #[test]
    fun test_for_each_ref() {
        let m = make(1, 4, 2, 5);
        let s = 0;
        for_each_ref(&m, |x, y| {
            s = s + *x + *y;
        });
        assert!(s == 12, 0)
    }

    #[test]
    fun test_for_each_mut() {
        let m = make(1, 4, 2, 5);
        for_each_mut(&mut m, |_key, val| {
            let val : &mut u64 = val;
            *val = *val + 1
        });
        assert!(*borrow(&m, &1) == 5, 1)
    }

    #[test]
    fun test_fold() {
        let m = make(1, 4, 2, 5);
        let r = fold(m, 0, |accu, key, val| {
            accu + key + val
        });
        assert!(r == 12, 0);
    }

    #[test]
    fun test_map() {
        let m = make(1, 4, 2, 5);
        let r = map(m, |val| val + 1);
        assert!(*borrow(&r, &1) == 5, 1)
    }

    #[test]
    fun test_map_ref() {
        let m = make(1, 4, 2, 5);
        let r = map_ref(&m, |val| *val + 1);
        assert!(*borrow(&r, &1) == 5, 1)
    }

    #[test]
    fun test_filter() {
        let m = make(1, 4, 2, 5);
        let r = filter(m, |val| *val > 4);
        assert!(length(&r) == 1, 1);
        assert!(*borrow(&r, &2) == 5, 1)
    }

    #[test]
    fun test_any() {
        let m = make(1, 4, 2, 5);
        let r = any(&m, |_k, v| *v > 4);
        assert!(r, 1)
    }

    #[test]
    fun test_all() {
        let m = make(1, 4, 2, 5);
        let r = all(&m, |_k, v| *v > 4);
        assert!(!r, 1)
    }
}
