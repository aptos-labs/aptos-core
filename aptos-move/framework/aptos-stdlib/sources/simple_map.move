/// This module provides a solution for sorted maps, that is it has the properties that
/// 1) Keys point to Values
/// 2) Each Key must be unique
/// 3) A Key can be found within O(Log N) time
/// 4) The data is stored as sorted by Key
/// 5) Adds and removals take O(N) time
module aptos_std::simple_map {
    use std::error;
    use std::option;
    use std::vector;
    use aptos_std::comparator;

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
        let (maybe_idx, _) = find(map, key);
        assert!(option::is_some(&maybe_idx), error::invalid_argument(EKEY_NOT_FOUND));
        let idx = option::extract(&mut maybe_idx);
        &vector::borrow(&map.data, idx).value
    }

    public fun borrow_mut<Key: store, Value: store>(
        map: &mut SimpleMap<Key, Value>,
        key: &Key,
    ): &mut Value {
        let (maybe_idx, _) = find(map, key);
        assert!(option::is_some(&maybe_idx), error::invalid_argument(EKEY_NOT_FOUND));
        let idx = option::extract(&mut maybe_idx);
        &mut vector::borrow_mut(&mut map.data, idx).value
    }

    public fun contains_key<Key: store, Value: store>(
        map: &SimpleMap<Key, Value>,
        key: &Key,
    ): bool {
        let (maybe_idx, _) = find(map, key);
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
        let (maybe_idx, maybe_placement) = find(map, &key);
        assert!(option::is_none(&maybe_idx), error::invalid_argument(EKEY_ALREADY_EXISTS));

        // Append to the end and then swap elements until the list is ordered again
        vector::push_back(&mut map.data, Element { key, value });

        let placement = option::extract(&mut maybe_placement);
        let end = vector::length(&map.data) - 1;
        while (placement < end) {
            vector::swap(&mut map.data, placement, end);
            placement = placement + 1;
        };
    }

    public fun remove<Key: store, Value: store>(
        map: &mut SimpleMap<Key, Value>,
        key: &Key,
    ): (Key, Value) {
        let (maybe_idx, _) = find(map, key);
        assert!(option::is_some(&maybe_idx), error::invalid_argument(EKEY_NOT_FOUND));

        let placement = option::extract(&mut maybe_idx);
        let end = vector::length(&map.data) - 1;

        while (placement < end) {
            vector::swap(&mut map.data, placement, placement + 1);
            placement = placement + 1;
        };

        let Element { key, value } = vector::pop_back(&mut map.data);
        (key, value)
    }

    fun find<Key: store, Value: store>(
        map: &SimpleMap<Key, Value>,
        key: &Key,
    ): (option::Option<u64>, option::Option<u64>) {
        let length = vector::length(&map.data);

        if (length == 0) {
            return (option::none(), option::some(0))
        };

        let left = 0;
        let right = length;

        while (left != right) {
            let mid = left + (right - left) / 2;
            let potential_key = &vector::borrow(&map.data, mid).key;
            if (comparator::is_smaller_than(&comparator::compare(potential_key, key))) {
                left = mid + 1;
            } else {
                right = mid;
            };
        };

        if (left != length && key == &vector::borrow(&map.data, left).key) {
            (option::some(left), option::none())
        } else {
            (option::none(), option::some(left))
        }
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
    public fun test_several() {
        let map = create<u64, u64>();
        add(&mut map, 6, 6);
        add(&mut map, 1, 1);
        add(&mut map, 5, 5);
        add(&mut map, 2, 2);
        add(&mut map, 3, 3);
        add(&mut map, 0, 0);
        add(&mut map, 7, 7);
        add(&mut map, 4, 4);

        let idx = 0;
        while (idx < vector::length(&map.data)) {
            assert!(idx == vector::borrow(&map.data, idx).key, idx);
            idx = idx + 1;
        };

        remove(&mut map, &0);
        remove(&mut map, &1);
        remove(&mut map, &2);
        remove(&mut map, &3);
        remove(&mut map, &4);
        remove(&mut map, &5);
        remove(&mut map, &6);
        remove(&mut map, &7);

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
}
