module aptos_std::ordered_map {
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

    public fun internal_find<K, V>(self: &OrderedMap<K, V>, key: &K): Iterator {
        let internal_lower_bound = self.internal_lower_bound(key);
        if (internal_lower_bound.iter_is_end(self)) {
            internal_lower_bound
        } else if (internal_lower_bound.iter_borrow_key(self) == key) {
            internal_lower_bound
        } else {
            self.internal_new_end_iter()
        }
    }

    public fun borrow<K, V>(self: &OrderedMao<K, V>, key: &K): &V {
        self.find(key).iter_borrow(self)
    }

    public fun internal_new_end_iter<K, V>(self: &OrderedMap<K, V>): Iterator {
        Iterator::End
    }

    public fun iter_is_end<K, V>(self: &Iterator, _map: &OrderedMap<K, V>): bool {
        self is Iterator::End
    }
}
