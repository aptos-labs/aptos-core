/// The core of an ordered map: entries kept sorted by key, binary search.
module 0x42::ordered_map {
    use std::vector;

    struct Entry has copy, drop, store {
        key: u64,
        value: u64,
    }

    struct Map has copy, drop, store {
        entries: vector<Entry>,
    }

    const E_PRESENT: u64 = 1;
    const E_ABSENT: u64 = 2;

    public fun empty(): Map {
        Map { entries: vector::empty() }
    }

    public fun length(map: &Map): u64 {
        vector::length(&map.entries)
    }

    /// The first index whose key is not below `key`.
    fun lower_bound(map: &Map, key: u64): u64 {
        let low = 0;
        let high = vector::length(&map.entries);
        while (low < high) {
            let mid = low + (high - low) / 2;
            if (vector::borrow(&map.entries, mid).key < key) {
                low = mid + 1;
            } else {
                high = mid;
            }
        };
        low
    }

    public fun contains(map: &Map, key: u64): bool {
        let index = lower_bound(map, key);
        index < vector::length(&map.entries) && vector::borrow(&map.entries, index).key == key
    }

    public fun get(map: &Map, key: u64): u64 {
        let index = lower_bound(map, key);
        assert!(index < vector::length(&map.entries), E_ABSENT);
        let entry = vector::borrow(&map.entries, index);
        assert!(entry.key == key, E_ABSENT);
        entry.value
    }

    public fun add(map: &mut Map, key: u64, value: u64) {
        let index = lower_bound(map, key);
        if (index < vector::length(&map.entries)) {
            assert!(vector::borrow(&map.entries, index).key != key, E_PRESENT);
        };
        vector::push_back(&mut map.entries, Entry { key, value });
        let i = vector::length(&map.entries) - 1;
        while (i > index) {
            vector::swap(&mut map.entries, i, i - 1);
            i = i - 1;
        };
    }

    public fun remove(map: &mut Map, key: u64): u64 {
        let index = lower_bound(map, key);
        assert!(index < vector::length(&map.entries), E_ABSENT);
        let entry = vector::remove(&mut map.entries, index);
        assert!(entry.key == key, E_ABSENT);
        entry.value
    }
    spec length {
        ensures result == len(map.entries);
        aborts_if false;
    }
}
