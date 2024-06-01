module 0x42::simple_map {
    use std::error;
    use std::option;
    use std::vector;

    const EKEY_NOT_FOUND: u64 = 2;

    struct SimpleMap<Key, Value> has copy, drop, store {
        data: vector<Element<Key, Value>>,
    }

    struct Element<Key, Value> has copy, drop, store {
        key: Key,
        value: Value,
    }

    public fun borrow<Key: store, Value: store>(
        map: &SimpleMap<Key, Value>,
        key: &Key,
    ): &Value {
        let maybe_idx = find(map, key);
        assert!(option::is_some(&maybe_idx), error::invalid_argument(EKEY_NOT_FOUND));
        let idx = option::extract(&mut maybe_idx);
        // Below call triggers nested post processing for inner .data and outer .value selection
        &vector::borrow(&map.data, idx).value
    }


    fun find<Key: store, Value: store>(
        map: &SimpleMap<Key, Value>,
        key: &Key,
    ): option::Option<u64> {
        let leng = vector::length(&map.data);
        let i = 0;
        while (i < leng) {
            let element = vector::borrow(&map.data, i);
            if (&element.key == key) {
                return option::some(i)
            };
            i = i + 1;
        };
        option::none<u64>()
    }
}
