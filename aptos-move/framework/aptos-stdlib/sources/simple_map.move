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
    use std::option::Option;

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

    public fun to_vec<Key: store, Value: store>(
        map: SimpleMap<Key, Value>): vector<Element<Key, Value>> {
        let SimpleMap { data } = map;
        data
    }

    public fun split_element<Key: store, Value: store>(e: Element<Key, Value>): (Key, Value) {
        let Element { key, value} = e;
        (key, value)
    }

    public fun upsert<Key: store, Value: store>(
        map: &mut SimpleMap<Key, Value>,
        key: Key,
        value: Value
    ): (Option<Key>, Option<Value>) {
        let len = std::vector::length(&map.data);
        let i = 0;
        while (i < len) {
            let element = vector::borrow(&mut map.data, i);
            if (&element.key == &key) {
                let Element {key: _k, value: _v} = vector::swap_remove(&mut map.data, i);
                vector::push_back(&mut map.data, Element { key, value});
                return (option::some(_k), option::some(_v))
            };
            i = i + 1;
        };
        vector::push_back(&mut map.data, Element { key, value });
        return (option::none(), option::none())
    }

    public inline fun upsert_drop<Key: store, Value: store>(
        map: &mut SimpleMap<Key, Value>,
        key: Key,
        value: Value,
        drop: |Key, Value|
    ) {
        let len = std::vector::length(&map.data);
        let i = 0;
        while (i < len) {
            let element = vector::borrow(&mut map.data, i);
            if (&element.key == &key) {
                break
            };
            i = i + 1;
        };
        if (i == len) {
            vector::push_back(&mut map.data, Element { key, value });
        } else {
            let Element {key: _k, value: _v} = vector::swap_remove(&mut map.data, i);
            vector::push_back(&mut map.data, Element { key, value});
            drop(_k, _v);
        }
    }

    public inline fun destroy<Key: store, Value: store>(
        map: SimpleMap<Key, Value>,
        d: |Key, Value|
    ) {
        let vec = to_vec(map);
        std::vector::destroy(vec, |e| { let (_k, _v) = split_element(e); d(_k, _v) });
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
    struct OnlyMove has store { val: u64 }

    #[test]
    public fun upsert_test() {
        let map = create<u64, OnlyMove>();
        // test adding 3 elements using upsert
        let (_, o1) = upsert(&mut map, 1, OnlyMove { val: 1 } );
        let (_, o2) = upsert(&mut map, 2, OnlyMove { val: 2 } );
        let (_, o3) = upsert(&mut map, 3, OnlyMove { val: 3 } );

        assert!(length(&map) == 3, 0);
        assert!(contains_key(&map, &1), 1);
        assert!(contains_key(&map, &2), 2);
        assert!(contains_key(&map, &3), 3);
        assert!(borrow(&map, &1).val == 1, 4);
        assert!(borrow(&map, &2).val == 2, 5);
        assert!(borrow(&map, &3).val == 3, 6);

        // change mapping 1->1 to 1->4
        let (_, o4) = upsert(&mut map, 1, OnlyMove { val: 4 } );

        assert!(length(&map) == 3, 7);
        assert!(contains_key(&map, &1), 8);
        assert!(borrow(&map, &1).val == 4, 9);

        option::destroy(o1, |o| { let OnlyMove { val: _ } = o; });
        option::destroy(o2, |o| { let OnlyMove { val: _ } = o; });
        option::destroy(o3, |o| { let OnlyMove { val: _ } = o; });
        option::destroy(o4, |o| { let OnlyMove { val: _ } = o; });

        destroy(map, |_k, _v| { let OnlyMove { val: _ } = _v; });
    }

    #[test]
    public fun upsert_drop_test() {
        let map = create<u64, OnlyMove>();
        // test adding 3 elements using upsert
        upsert_drop(&mut map, 1, OnlyMove { val: 1 }, |_k, _v| { let OnlyMove { val: _ } = _v; });
        upsert_drop(&mut map, 2, OnlyMove { val: 2 }, |_k, _v| { let OnlyMove { val: _ } = _v; });
        upsert_drop(&mut map, 3, OnlyMove { val: 3 }, |_k, _v| { let OnlyMove { val: _ } = _v; });

        assert!(length(&map) == 3, 0);
        assert!(contains_key(&map, &1), 1);
        assert!(contains_key(&map, &2), 2);
        assert!(contains_key(&map, &3), 3);
        assert!(borrow(&map, &1).val == 1, 4);
        assert!(borrow(&map, &2).val == 2, 5);
        assert!(borrow(&map, &3).val == 3, 6);

        // change mapping 1->1 to 1->4
        upsert_drop(&mut map, 1, OnlyMove { val: 4 }, |_k, _v| { let OnlyMove { val: _ } = _v; });

        assert!(length(&map) == 3, 7);
        assert!(contains_key(&map, &1), 8);
        assert!(borrow(&map, &1).val == 4, 9);

        destroy(map, |_k, _v| { let OnlyMove { val: _ } = _v; });
    }
}
