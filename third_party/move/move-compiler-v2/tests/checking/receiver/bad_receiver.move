module velor_std::ordered_map {
    const EITER_OUT_OF_BOUNDS: u64 = 3;

    struct Entry<K, V> has drop, copy, store {
        key: K,
        value: V,
    }

    enum OrderedMap<K, V> has drop, copy, store {
        SortedVectorMap {
            entries: vector<Entry<K, V>>,
        }
    }

    enum Iterator has copy, drop {
        End,
        Position {
            index: u64,
        },
    }

    public fun iter_borrow_key<K, V>(self: &Iterator, map: &OrderedMap<K, V>): &K {
        assert!(!(self is Iterator::End), EITER_OUT_OF_BOUNDS);

        &map.entries.borrow(self.index).key
    }

    public fun find<K, V>(self: &OrderedMap<K, V>, key: &K): Iterator {
        let lower_bound = self.lower_bound(key);
        if (lower_bound.iter_is_end(self)) {
            lower_bound
        } else if (lower_bound.iter_borrow_key(self) == key) {
            lower_bound
        } else {
            self.new_end_iter()
        }
    }

    public fun borrow<K, V>(self: &OrderedMao<K, V>, key: &K): &V {
        self.find(key).iter_borrow(self)
    }

    public fun new_end_iter<K, V>(self: &OrderedMap<K, V>): Iterator {
        Iterator::End
    }

    public fun iter_is_end<K, V>(self: &Iterator, _map: &OrderedMap<K, V>): bool {
        self is Iterator::End
    }
}
