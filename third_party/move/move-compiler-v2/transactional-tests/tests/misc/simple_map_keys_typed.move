//# publish
module 0x42::simple_map {
    use 0x1::vector;

    struct SimpleMap<Key, Value> has copy, drop, store {
        data: vector<Element<Key, Value>>,
    }

    struct Element<Key, Value> has copy, drop, store {
        key: Key,
        value: Value,
    }


    /// Return all keys in the map. This requires keys to be copyable.
    public fun keys<Key: copy, Value>(map: &SimpleMap<Key, Value>): vector<Key> {
        map_ref(&map.data, |e: &Element<Key, Value>| {
            e.key
        })
    }

   public inline fun map_ref<Element, NewElement>(
        v: &vector<Element>,
        f: |&Element|NewElement
    ): vector<NewElement> {
        let result = vector<NewElement>[];
        for_each_ref(v, |elem: &Element| vector::push_back(&mut result, f(elem)));
        result
    }

    public inline fun for_each_ref<Element>(v: &vector<Element>, f: |&Element|) {
        let i = 0;
        let len = vector::length(v);
        while (i < len) {
            f(vector::borrow(v, i));
            i = i + 1
        }
    }

    public fun run() {
        let entry = Element{key: 1, value: 2};
        let data = vector[entry, entry, entry];
        let map = SimpleMap{data};
        let keys = keys(&map);
        assert!(keys == vector[1, 1, 1], 33);
    }
}

//# run  0x42::simple_map::run
