/// This module provides an implementation for an ordered map.
///
/// Keys point to values, and each key in the map must be unique.
///
/// Currently, one implementation is provided, backed by a single sorted vector.
///
/// That means that keys can be found within O(log N) time.
/// Adds and removals take O(N) time, but the constant factor is small,
/// as it does only O(log N) comparisons, and does efficient mem-copy with vector operations.
///
/// Additionally, it provides a way to lookup and iterate over sorted keys, making range query
/// take O(log N + R) time (where R is number of elements in the range).
///
/// Most methods operate with OrderedMap being `self`.
/// All methods that start with iter_*, operate with Iterator being `self`.
///
/// Uses cmp::compare for ordering, which compares primitive types natively, and uses common
/// lexicographical sorting for complex types.
module aptos_std::ordered_map {
    // friend aptos_std::big_ordered_map;

    use std::vector;

    use std::option::{Self, Option};
    use std::cmp;
    use std::error;
    use std::mem;

    /// Map key already exists
    const EKEY_ALREADY_EXISTS: u64 = 1;
    /// Map key is not found
    const EKEY_NOT_FOUND: u64 = 2;
    // Trying to do an operation on an Iterator that would go out of bounds
    const EITER_OUT_OF_BOUNDS: u64 = 3;
    /// New key used in replace_key_inplace doesn't respect the order
    const ENEW_KEY_NOT_IN_ORDER: u64 = 4;

    /// Individual entry holding (key, value) pair
    struct Entry<K, V> has drop, copy, store {
        key: K,
        value: V,
    }

    /// The OrderedMap datastructure.
    enum OrderedMap<K, V> has drop, copy, store {
        /// sorted-vector based implementation of OrderedMap
        SortedVectorMap {
            /// List of entries, sorted by key.
            entries: vector<Entry<K, V>>,
        }
    }

    /// An iterator pointing to a position between two elements in the
    enum Iterator has copy, drop {
        // TODO: see if it is cleaner/faster to have end-iter be Position { index = len() }
        // and not have a separate variant for it.
        End,
        Position {
            /// The index of the iterator pointing to.
            index: u64,
        },
    }

    /// Creates a new empty OrderedMap, using default (SortedVectorMap) implementation.
    public fun new<K, V>(): OrderedMap<K, V> {
        OrderedMap::SortedVectorMap {
            entries: vector::empty(),
        }
    }

    /// Number of elements in the map.
    public fun length<K, V>(self: &OrderedMap<K, V>): u64 {
        self.entries.length()
    }

    /// Whether map is empty.
    public fun is_empty<K, V>(self: &OrderedMap<K, V>): bool {
        self.entries.is_empty()
    }

    /// Add a key/value pair to the map.
    /// Aborts with EKEY_ALREADY_EXISTS if key already exist.
    public fun add<K, V>(self: &mut OrderedMap<K, V>, key: K, value: V) {
        let len = self.entries.length();
        let index = binary_search(&key, &self.entries, 0, len);

        // key must not already be inside.
        assert!(index >= len || &self.entries[index].key != &key, error::invalid_argument(EKEY_ALREADY_EXISTS));
        self.entries.insert(index, Entry { key, value });
    }

    /// If the key doesn't exist in the map, inserts the key/value, and returns none.
    /// Otherwise, updates the value under the given key, and returns the old value.
    public fun upsert<K: drop, V>(self: &mut OrderedMap<K, V>, key: K, value: V): Option<V> {
        let len = self.entries.length();
        let index = binary_search(&key, &self.entries, 0, len);

        if (index < len && &self.entries[index].key == &key) {
            let Entry {
                key: _,
                value: old_value,
            } = self.entries.replace(index, Entry { key, value });
            option::some(old_value)
        } else {
            self.entries.insert(index, Entry { key, value });
            option::none()
        }
    }

    /// Remove a key/value pair from the map.
    /// Aborts with EKEY_NOT_FOUND if `key` doesn't exist.
    public fun remove<K: drop, V>(self: &mut OrderedMap<K, V>, key: &K): V {
        let len = self.entries.length();
        let index = binary_search(key, &self.entries, 0, len);
        assert!(index < len, error::invalid_argument(EKEY_NOT_FOUND));
        let Entry { key: old_key, value } = self.entries.remove(index);
        assert!(key == &old_key, error::invalid_argument(EKEY_NOT_FOUND));
        value
    }

    /// Returns whether map contains a given key.
    public fun contains<K, V>(self: &OrderedMap<K, V>, key: &K): bool {
        !self.find(key).iter_is_end()
    }

    /// Changes the key, with keeping the same value attached to it
    /// Aborts with EKEY_NOT_FOUND if `old_key` doesn't exist.
    /// Aborts with ENEW_KEY_NOT_IN_ORDER if `new_key` doesn't keep the order `old_key` was in.
    public(friend) fun replace_key_inplace<K: drop, V>(self: &mut OrderedMap<K, V>, old_key: &K, new_key: K) {
        let len = self.entries.length();
        let index = binary_search(old_key, &self.entries, 0, len);
        assert!(index < len, error::invalid_argument(EKEY_NOT_FOUND));

        if (index > 0) {
            assert!(cmp::compare(&self.entries[index - 1].key, &new_key).is_lt(), error::invalid_argument(ENEW_KEY_NOT_IN_ORDER))
        };

        if (index + 1 < len) {
            assert!(cmp::compare(&new_key, &self.entries[index + 1].key).is_lt(), error::invalid_argument(ENEW_KEY_NOT_IN_ORDER))
        };

        let entry = self.entries.borrow_mut(index);
        assert!(old_key == &entry.key, error::invalid_argument(EKEY_NOT_FOUND));
        entry.key = new_key;
    }

    /// Takes all elements from `other` and adds them to `self`.
    public fun append<K, V>(self: &mut OrderedMap<K, V>, other: OrderedMap<K, V>) {
        let OrderedMap::SortedVectorMap {
            entries: other_entries,
        } = other;

        if (other_entries.is_empty()) {
            other_entries.destroy_empty();
            return;
        };

        if (self.entries.is_empty()) {
            self.entries.append(other_entries);
            return;
        };

        // Optimization: if all elements in `other` are larger than all elements in `self`, we can just move them over.
        if (cmp::compare(&self.entries.borrow(self.entries.length() - 1).key, &other_entries.borrow(0).key).is_lt()) {
            self.entries.append(other_entries);
            return;
        };

        // TODO: can be implemented more efficiently, as we know both maps are already sorted.
        while (!other_entries.is_empty()) {
            let Entry { key, value } = other_entries.pop_back();
            self.add(key, value);
        };

        other_entries.destroy_empty();
    }

    /// Splits the collection into two, such to leave `self` with `at` number of elements.
    /// Returns a newly allocated map containing the elements in the range [at, len).
    /// After the call, the original map will be left containing the elements [0, at)
    /// with its previous capacity unchanged.
    public fun split_off<K, V>(self: &mut OrderedMap<K, V>, at: u64): OrderedMap<K, V> {
        let rest = self.entries.trim(at);

        OrderedMap::SortedVectorMap {
            entries: rest
        }
    }

    /// Returns an iterator pointing to the first element that is greater or equal to the provided
    /// key, or an end iterator if such element doesn't exist.
    public fun lower_bound<K, V>(self: &OrderedMap<K, V>, key: &K): Iterator {
        let entries = &self.entries;
        let len = entries.length();

        let index = binary_search(key, entries, 0, len);
        if (index == len) {
            self.new_end_iter()
        } else {
            new_iter(index)
        }
    }

    /// Returns an iterator pointing to the element that equals to the provided key, or an end
    /// iterator if the key is not found.
    public fun find<K, V>(self: &OrderedMap<K, V>, key: &K): Iterator {
        let lower_bound = self.lower_bound(key);
        if (lower_bound.iter_is_end()) {
            lower_bound
        } else if (lower_bound.iter_borrow_key(self) == key) {
            lower_bound
        } else {
            self.new_end_iter()
        }
    }

    /// Returns the begin iterator.
    public fun new_begin_iter<K, V>(self: &OrderedMap<K, V>): Iterator {
        if (self.is_empty()) {
            return Iterator::End;
        };

        new_iter(0)
    }

    /// Returns the end iterator.
    public fun new_end_iter<K, V>(self: &OrderedMap<K, V>): Iterator {
        Iterator::End
    }

    // ========== Section for methods opearting on iterators ========
    // Note: After any modifications to the map, do not use any of the iterators obtained beforehand.
    // Operations on iterators after map is modified are unexpected/incorrect.

    // TODO: see if it is more understandable if iterator points between elements,
    // and there is iter_borrow_next and iter_borrow_prev, and provide iter_insert.

    /// Returns the next iterator, or none if already at the end iterator.
    /// Note: Requires that the map is not changed after the input iterator is generated.
    public fun iter_next<K, V>(self: Iterator, map: &OrderedMap<K, V>): Iterator {
        assert!(!self.iter_is_end(), error::invalid_argument(EITER_OUT_OF_BOUNDS));

        let index = self.index + 1;
        if (index < map.entries.length()) {
            new_iter(index)
        } else {
            map.new_end_iter()
        }
    }

    /// Returns the previous iterator, or none if already at the begin iterator.
    /// Note: Requires that the map is not changed after the input iterator is generated.
    public fun iter_prev<K, V>(self: Iterator, map: &OrderedMap<K, V>): Iterator {
        assert!(!self.iter_is_begin(map), error::invalid_argument(EITER_OUT_OF_BOUNDS));

        let index = if (self is Iterator::End) {
            map.entries.length() - 1
        } else {
            self.index - 1
        };

        new_iter(index)
    }

    /// Returns whether the iterator is a begin iterator.
    public fun iter_is_begin<K, V>(self: &Iterator, map: &OrderedMap<K, V>): bool {
        if (self is Iterator::End) {
            map.is_empty()
        } else {
            self.index == 0
        }
    }

    /// Returns true iff the iterator is a begin iterator from a non-empty collection.
    /// (I.e. if iterator points to a valid element)
    /// This method doesn't require having access to map, unlike iter_is_begin.
    public(friend) fun iter_is_begin_from_non_empty(self: &Iterator): bool {
        if (self is Iterator::End) {
            false
        } else {
            self.index == 0
        }
    }

    /// Returns whether the iterator is an end iterator.
    public fun iter_is_end(self: &Iterator): bool {
        self is Iterator::End
    }

    /// Borrows the key given iterator points to.
    /// Note: Requires that the map is not changed after the input iterator is generated.
    public fun iter_borrow_key<K, V>(self: &Iterator, map: &OrderedMap<K, V>): &K {
        assert!(!(self is Iterator::End), error::invalid_argument(EITER_OUT_OF_BOUNDS));

        &map.entries.borrow(self.index).key
    }

    /// Borrows the value given iterator points to.
    /// Note: Requires that the map is not changed after the input iterator is generated.
    public fun iter_borrow<K, V>(self: Iterator, map: &OrderedMap<K, V>): &V {
        assert!(!(self is Iterator::End), error::invalid_argument(EITER_OUT_OF_BOUNDS));
        &map.entries.borrow(self.index).value
    }

    /// Mutably borrows the value iterator points to.
    /// Note: Requires that the map is not changed after the input iterator is generated.
    public fun iter_borrow_mut<K, V>(self: Iterator, map: &mut OrderedMap<K, V>): &mut V {
        assert!(!(self is Iterator::End), error::invalid_argument(EITER_OUT_OF_BOUNDS));
        &mut map.entries.borrow_mut(self.index).value
    }

    /// Removes (key, value) pair iterator points to, returning the previous value.
    /// Aborts with EKEY_NOT_FOUND if iterator is pointing to the end.
    /// Note: Requires that the map is not changed after the input iterator is generated.
    public fun iter_remove<K: drop, V>(self: Iterator, map: &mut OrderedMap<K, V>): V {
        assert!(!(self is Iterator::End), error::invalid_argument(EKEY_NOT_FOUND));

        let Entry { key: _, value } = map.entries.remove(self.index);
        value
    }

    /// Replaces the value iterator is pointing to, returning the previous value.
    /// Aborts with EKEY_NOT_FOUND if iterator is pointing to the end.
    /// Note: Requires that the map is not changed after the input iterator is generated.
    public fun iter_replace<K, V>(self: Iterator, map: &mut OrderedMap<K, V>, value: V): V {
        assert!(!(self is Iterator::End), error::invalid_argument(EKEY_NOT_FOUND));

        let entry = map.entries.borrow_mut(self.index);
        mem::replace(&mut entry.value, value)
    }

    /// Destroys empty map.
    /// Aborts if `self` is not empty.
    public fun destroy_empty<K, V>(self: OrderedMap<K, V>) {
        let OrderedMap::SortedVectorMap { entries } = self;
        // assert!(entries.is_empty(), E_NOT_EMPTY);
        entries.destroy_empty();
    }

    // ========= Section with inline for-loop methods =======

    /// Apply the function to a reference of each key-value pair in the table.
    public inline fun for_each_ref<K, V>(self: &OrderedMap<K, V>, f: |&K, &V|) {
        let iter = self.new_begin_iter();
        while (!iter.iter_is_end()) {
            f(iter.iter_borrow_key(self), iter.iter_borrow(self));
            iter = iter.iter_next(self);
        }
        // vector::for_each_ref(
        //     &self.entries,
        //     |entry| {
        //         f(&entry.key, &entry.value)
        //     }
        // );
    }

    /// Apply the function to a mutable reference of each key-value pair in the table.
    public inline fun for_each_mut<K, V>(self: &mut OrderedMap<K, V>, f: |K, &mut V|) {
        let iter = self.new_begin_iter();
        while (!iter.iter_is_end()) {
            let key = *iter.iter_borrow_key(self);
            f(key, iter.iter_borrow_mut(self));
            iter = iter.iter_next(self);
        }
        // vector::for_each_mut(
        //     &mut self.entries,
        //     |entry| {
        //         f(&mut entry.key, &mut entry.value)
        //     }
        // );
    }

    // ========= Section with private methods ===============

    inline fun new_iter(index: u64): Iterator {
        Iterator::Position {
            index: index,
        }
    }

    // return index containing the key, or insert position.
    // I.e. index of first element that has key larger or equal to the passed `key` argument.
    fun binary_search<K, V>(key: &K, entries: &vector<Entry<K, V>>, start: u64, end: u64): u64 {
        let l = start;
        let r = end;
        while (l != r) {
            let mid = l + (r - l) / 2;
            let comparison = cmp::compare(&entries.borrow(mid).key, key);
            // TODO: check why this short-circuiting actually performs worse
            // if (comparison.is_equal()) {
            //     // there can only be one equal value, so end the search.
            //     return mid;
            // } else
            if (comparison.is_lt()) {
                l = mid + 1;
            } else {
                r = mid;
            };
        };
        l
    }

    // public fun num_below_iter<K, V>(self: Iterator, map: &OrderedMap<K, V>): u64 {
    //     if (self.iter_is_end()) {
    //         map.entries.length()
    //     } else {
    //         self.index
    //     }
    // }

    // ================= Section for tests =====================

    #[test_only]
    fun print_map<K, V>(self: &OrderedMap<K, V>) {
        aptos_std::debug::print(&self.entries);
    }

    #[test_only]
    public fun validate_ordered<K, V>(self: &OrderedMap<K, V>) {
        let len = self.entries.length();
        let i = 1;
        while (i < len) {
            assert!(cmp::compare(&self.entries.borrow(i).key, &self.entries.borrow(i - 1).key).is_greater_than(), 1);
            i = i + 1;
        };
    }

    #[test_only]
    fun validate_iteration<K: drop + copy + store, V: store>(self: &OrderedMap<K, V>) {
        let expected_num_elements = self.length();
        let num_elements = 0;
        let it = self.new_begin_iter();
        while (!it.iter_is_end()) {
            num_elements = num_elements + 1;
            it = it.iter_next(self);
        };
        assert!(num_elements == expected_num_elements, 2);

        let num_elements = 0;
        let it = self.new_end_iter();
        while (!it.iter_is_begin(self)) {
            it = it.iter_prev(self);
            num_elements = num_elements + 1;
        };
        assert!(num_elements == expected_num_elements, 3);
    }

    #[test_only]
    fun validate_map<K: drop + copy + store, V: store>(self: &OrderedMap<K, V>) {
        self.validate_ordered();
        self.validate_iteration();
    }

    #[test]
    fun test_map_small() {
        let map = new();
        map.print_map(); map.validate_map();
        map.add(1, 1); map.print_map(); map.validate_map();
        map.add(2, 2); map.print_map(); map.validate_map();
        let r1 = map.upsert(3, 3); map.print_map(); map.validate_map();
        assert!(r1 == option::none(), 4);
        map.add(4, 4); map.print_map(); map.validate_map();
        let r2 = map.upsert(4, 8); map.print_map(); map.validate_map();
        assert!(r2 == option::some(4), 5);
        map.add(5, 5); map.print_map(); map.validate_map();
        map.add(6, 6); map.print_map(); map.validate_map();

        map.remove(&5); map.print_map(); map.validate_map();
        map.remove(&4); map.print_map(); map.validate_map();
        map.remove(&1); map.print_map(); map.validate_map();
        map.remove(&3); map.print_map(); map.validate_map();
        map.remove(&2); map.print_map(); map.validate_map();
        map.remove(&6); map.print_map(); map.validate_map();

        map.destroy_empty();
    }

    #[test_only]
    public fun large_dataset(): vector<u64> {
        vector[383, 886, 777, 915, 793, 335, 386, 492, 649, 421, 362, 27, 690, 59, 763, 926, 540, 426, 172, 736, 211, 368, 567, 429, 782, 530, 862, 123, 67, 135, 929, 802, 22, 58, 69, 167, 393, 456, 11, 42, 229, 373, 421, 919, 784, 537, 198, 324, 315, 370, 413, 526, 91, 980, 956, 873, 862, 170, 996, 281, 305, 925, 84, 327, 336, 505, 846, 729, 313, 857, 124, 895, 582, 545, 814, 367, 434, 364, 43, 750, 87, 808, 276, 178, 788, 584, 403, 651, 754, 399, 932, 60, 676, 368, 739, 12, 226, 586, 94, 539, 795, 570, 434, 378, 467, 601, 97, 902, 317, 492, 652, 756, 301, 280, 286, 441, 865, 689, 444, 619, 440, 729, 31, 117, 97, 771, 481, 675, 709, 927, 567, 856, 497, 353, 586, 965, 306, 683, 219, 624, 528, 871, 732, 829, 503, 19, 270, 368, 708, 715, 340, 149, 796, 723, 618, 245, 846, 451, 921, 555, 379, 488, 764, 228, 841, 350, 193, 500, 34, 764, 124, 914, 987, 856, 743, 491, 227, 365, 859, 936, 432, 551, 437, 228, 275, 407, 474, 121, 858, 395, 29, 237, 235, 793, 818, 428, 143, 11, 928, 529]
    }

    #[test_only]
    public fun large_dataset_shuffled(): vector<u64> {
        vector[895, 228, 530, 784, 624, 335, 729, 818, 373, 456, 914, 226, 368, 750, 428, 956, 437, 586, 763, 235, 567, 91, 829, 690, 434, 178, 584, 426, 228, 407, 237, 497, 764, 135, 124, 421, 537, 270, 11, 367, 378, 856, 529, 276, 729, 618, 929, 227, 149, 788, 925, 675, 121, 795, 306, 198, 421, 350, 555, 441, 403, 932, 368, 383, 928, 841, 440, 771, 364, 902, 301, 987, 467, 873, 921, 11, 365, 340, 739, 492, 540, 386, 919, 723, 539, 87, 12, 782, 324, 862, 689, 395, 488, 793, 709, 505, 582, 814, 245, 980, 936, 736, 619, 69, 370, 545, 764, 886, 305, 551, 19, 865, 229, 432, 29, 754, 34, 676, 43, 846, 451, 491, 871, 500, 915, 708, 586, 60, 280, 652, 327, 172, 856, 481, 796, 474, 219, 651, 170, 281, 84, 97, 715, 857, 353, 862, 393, 567, 368, 777, 97, 315, 526, 94, 31, 167, 123, 413, 503, 193, 808, 649, 143, 42, 444, 317, 67, 926, 434, 211, 379, 570, 683, 965, 732, 927, 429, 859, 313, 528, 996, 117, 492, 336, 22, 399, 275, 802, 743, 124, 846, 58, 858, 286, 756, 601, 27, 59, 362, 793]
    }

    #[test]
    fun test_map_large() {
        use std::string;

        let map = new();
        let data = large_dataset();
        let shuffled_data = large_dataset_shuffled();

        let len = data.length();
        for (i in 0..len) {
            let element = *data.borrow(i);
            map.upsert(element, element);
            map.validate_map();
        };

        for (i in 0..len) {
            let element = shuffled_data.borrow(i);
            let it = map.find(element);
            assert!(!it.iter_is_end(), 6);
            assert!(it.iter_borrow_key(&map) == element, 7);

            let it_next = it.iter_next(&map);
            let it_after = map.lower_bound(&(*element + 1));

            assert!(it_next == it_after, 8);
        };

        let removed = vector::empty();
        for (i in 0..len) {
            let element = shuffled_data.borrow(i);
            if (!removed.contains(element)) {
                removed.push_back(*element);
                map.remove(element);
                map.validate_map();
            } else {
                assert!(!map.contains(element));
            };
        };

        map.destroy_empty();
    }
}
