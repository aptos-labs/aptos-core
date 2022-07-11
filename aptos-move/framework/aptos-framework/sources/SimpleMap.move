/// This module provides a solution for sorted maps, that is it has the properties that
/// 1) Keys point to Values
/// 2) Each Key must be unique
/// 3) A Key can be found within O(Log N) time
/// 4) The data is stored as a sorted by Key
/// 5) Adds and removals take O(N) time
module AptosFramework::SimpleMap {
    use Std::Errors;
    use Std::Option;
    use Std::Vector;
    use AptosFramework::Comparator;

    const EKEY_ALREADY_EXISTS: u64 = 0;
    const EKEY_NOT_FOUND: u64 = 1;
    const EINDEX_OUT_OF_BOUNDS: u64 = 2; 

    struct SimpleMap<Key: store, Value: store> has store {
        data: vector<Element<Key, Value>>,
    }

    struct Element<Key: store, Value: store> has store {
        key: Key,
        value: Value,
    }

    public fun length<Key: store, Value: store>(map: &SimpleMap<Key, Value>): u64 {
        Vector::length(&map.data)
    }

    public fun create<Key: store, Value: store>(): SimpleMap<Key, Value> {
        SimpleMap {
            data: Vector::empty(),
        }
    }

    public fun borrow<Key: store, Value: store>(
        map: &SimpleMap<Key, Value>,
        key: &Key,
    ): &Value {
        let (maybe_idx, _) = find(map, key);
        assert!(Option::is_some(&maybe_idx), Errors::invalid_argument(EKEY_NOT_FOUND));
        let idx = Option::extract(&mut maybe_idx);
        &Vector::borrow(&map.data, idx).value
    }

    public fun borrow_mut<Key: store, Value: store>(
        map: &mut SimpleMap<Key, Value>,
        key: &Key,
    ): &mut Value {
        let (maybe_idx, _) = find(map, key);
        assert!(Option::is_some(&maybe_idx), Errors::invalid_argument(EKEY_NOT_FOUND));
        let idx = Option::extract(&mut maybe_idx);
        &mut Vector::borrow_mut(&mut map.data, idx).value
    }

    public fun contains_key<Key: store, Value: store>(
        map: &SimpleMap<Key, Value>,
        key: &Key,
    ): bool {
        let (maybe_idx, _) = find(map, key);
        Option::is_some(&maybe_idx)
    }

    public fun destroy_empty<Key: store, Value: store>(map: SimpleMap<Key, Value>) {
        let SimpleMap { data } = map;
        Vector::destroy_empty(data);
    }

    public fun add<Key: store, Value: store>(
        map: &mut SimpleMap<Key, Value>,
        key: Key,
        value: Value,
    ) {
        let (maybe_idx, maybe_placement) = find(map, &key);
        assert!(Option::is_none(&maybe_idx), Errors::invalid_argument(EKEY_ALREADY_EXISTS));

        // Append to the end and then swap elements until the list is ordered again
        Vector::push_back(&mut map.data, Element { key, value });

        let placement = Option::extract(&mut maybe_placement);
        let end = Vector::length(&map.data) - 1;
        while (placement < end) {
          Vector::swap(&mut map.data, placement, end);
          placement = placement + 1;
        };
    }

    public fun remove<Key: store, Value: store>(
        map: &mut SimpleMap<Key, Value>,
        key: &Key,
    ): (Key, Value) {
        let (maybe_idx, _) = find(map, key);
        assert!(Option::is_some(&maybe_idx), Errors::invalid_argument(EKEY_NOT_FOUND));

        let placement = Option::extract(&mut maybe_idx);
        let end = Vector::length(&map.data) - 1;

        while (placement < end) {
            Vector::swap(&mut map.data, placement, placement + 1);
            placement = placement + 1;
        };

        let Element { key, value } = Vector::pop_back(&mut map.data);
        (key, value)
    }

    fun find<Key: store, Value: store>(
        map: &SimpleMap<Key, Value>,
        key: &Key,
    ): (Option::Option<u64>, Option::Option<u64>) {
        let length = Vector::length(&map.data);

        if (length == 0) {
            return (Option::none(), Option::some(0))
        };

        let left = 0;
        let right = length;

        while (left != right) {
            let mid = (left + right) / 2;
            let potential_key = &Vector::borrow(&map.data, mid).key;
            if (Comparator::is_smaller_than(&Comparator::compare(potential_key, key))) {
                left = mid + 1;
            } else {
                right = mid;
            };
        };

        if (left != length && key == &Vector::borrow(&map.data, left).key) {
            (Option::some(left), Option::none())
        } else {
            (Option::none(), Option::some(left))
        }
    }

    public fun get_entry<Key: store, Value: store>(
        map: &SimpleMap<Key, Value>,
        index: u64,
    ) : (&Key, &Value) {
        assert!(index < Vector::length(&map.data) && index >= 0, Errors::invalid_argument(EINDEX_OUT_OF_BOUNDS));

        (&Vector::borrow(&map.data, index).key, &Vector::borrow(&map.data, index).value)
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
        while (idx < Vector::length(&map.data)) {
            assert!(idx == Vector::borrow(&map.data, idx).key, idx);
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

    #[test]
    public fun test_get_entry_valid_index() {
        let map = create<u64, u64>();
        add(&mut map, 3, 1);
        add(&mut map, 5, 2);

        let (key, value) = get_entry(&map, 0);
        assert!(key == &3, 0);
        assert!(value == &1, 0);

        (key, value) = get_entry(&map, 1);
        assert!(key == &5, 0);
        assert!(value == &2, 0);

        remove(&mut map, &3);
        remove(&mut map, &5);

        destroy_empty(map);
    }

    #[test]
    #[expected_failure]
    public fun test_get_entry_invalid_index() {
        let map = create<u64, u64>();
        get_entry(&map, 0);
        destroy_empty(map);
    }
}
