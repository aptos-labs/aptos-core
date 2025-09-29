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
/// All methods that start with iter_*, operate with IteratorPtr being `self`.
///
/// Uses cmp::compare for ordering, which compares primitive types natively, and uses common
/// lexicographical sorting for complex types.
///
/// TODO: all iterator functions are public(friend) for now, so that they can be modified in a
/// backward incompatible way. Type is also named IteratorPtr, so that Iterator is free to use later.
/// They are waiting for Move improvement that will allow references to be part of the struct,
/// allowing cleaner iterator APIs.
///
module aptos_std::ordered_map {
    friend aptos_std::big_ordered_map;

    use std::vector;

    use std::option::{Self, Option};
    use std::cmp;
    use std::error;

    /// Map key already exists
    const EKEY_ALREADY_EXISTS: u64 = 1;
    /// Map key is not found
    const EKEY_NOT_FOUND: u64 = 2;
    // Trying to do an operation on an IteratorPtr that would go out of bounds
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

    /// An iterator pointing to a valid position in an ordered map, or to the end.
    ///
    /// TODO: Once fields can be (mutable) references, this class will be deprecated.
    enum IteratorPtr has copy, drop {
        End,
        Position {
            /// The index of the iterator pointing to.
            index: u64,
        },
    }

    /// Create a new empty OrderedMap, using default (SortedVectorMap) implementation.
    public fun new<K, V>(): OrderedMap<K, V> {
        OrderedMap::SortedVectorMap {
            entries: vector::empty(),
        }
    }

    /// Create a OrderedMap from a vector of keys and values.
    /// Aborts with EKEY_ALREADY_EXISTS if duplicate keys are passed in.
    public fun new_from<K, V>(keys: vector<K>, values: vector<V>): OrderedMap<K, V> {
        let map = new();
        map.add_all(keys, values);
        map
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
        !self.find(key).iter_is_end(self)
    }

    public fun borrow<K, V>(self: &OrderedMap<K, V>, key: &K): &V {
        self.find(key).iter_borrow(self)
    }

    public fun borrow_mut<K, V>(self: &mut OrderedMap<K, V>, key: &K): &mut V {
        self.find(key).iter_borrow_mut(self)
    }

    /// Changes the key, while keeping the same value attached to it
    /// Aborts with EKEY_NOT_FOUND if `old_key` doesn't exist.
    /// Aborts with ENEW_KEY_NOT_IN_ORDER if `new_key` doesn't keep the order `old_key` was in.
    public(friend) fun replace_key_inplace<K: drop, V>(self: &mut OrderedMap<K, V>, old_key: &K, new_key: K) {
        let len = self.entries.length();
        let index = binary_search(old_key, &self.entries, 0, len);
        assert!(index < len, error::invalid_argument(EKEY_NOT_FOUND));

        assert!(old_key == &self.entries[index].key, error::invalid_argument(EKEY_NOT_FOUND));

        // check that after we update the key, order is going to be respected
        if (index > 0) {
            assert!(cmp::compare(&self.entries[index - 1].key, &new_key).is_lt(), error::invalid_argument(ENEW_KEY_NOT_IN_ORDER))
        };

        if (index + 1 < len) {
            assert!(cmp::compare(&new_key, &self.entries[index + 1].key).is_lt(), error::invalid_argument(ENEW_KEY_NOT_IN_ORDER))
        };

        self.entries[index].key = new_key;
    }

    /// Add multiple key/value pairs to the map. The keys must not already exist.
    /// Aborts with EKEY_ALREADY_EXISTS if key already exist, or duplicate keys are passed in.
    public fun add_all<K, V>(self: &mut OrderedMap<K, V>, keys: vector<K>, values: vector<V>) {
        // TODO: Can be optimized, by sorting keys and values, and then creating map.
        keys.zip(values, |key, value| {
            self.add(key, value);
        });
    }

    /// Add multiple key/value pairs to the map, overwrites values if they exist already,
    /// or if duplicate keys are passed in.
    public fun upsert_all<K: drop, V: drop>(self: &mut OrderedMap<K, V>, keys: vector<K>, values: vector<V>) {
        // TODO: Can be optimized, by sorting keys and values, and then creating map.
        keys.zip(values, |key, value| {
            self.upsert(key, value);
        });
    }

    /// Takes all elements from `other` and adds them to `self`,
    /// overwritting if any key is already present in self.
    public fun append<K: drop, V: drop>(self: &mut OrderedMap<K, V>, other: OrderedMap<K, V>) {
        self.append_impl(other);
    }

    /// Takes all elements from `other` and adds them to `self`.
    /// Aborts with EKEY_ALREADY_EXISTS if `other` has a key already present in `self`.
    public fun append_disjoint<K, V>(self: &mut OrderedMap<K, V>, other: OrderedMap<K, V>) {
        let overwritten = self.append_impl(other);
        assert!(overwritten.length() == 0, error::invalid_argument(EKEY_ALREADY_EXISTS));
        overwritten.destroy_empty();
    }

    /// Takes all elements from `other` and adds them to `self`, returning list of entries in self that were overwritten.
    fun append_impl<K, V>(self: &mut OrderedMap<K, V>, other: OrderedMap<K, V>): vector<Entry<K,V>> {
        let OrderedMap::SortedVectorMap {
            entries: other_entries,
        } = other;
        let overwritten = vector::empty();

        if (other_entries.is_empty()) {
            other_entries.destroy_empty();
            return overwritten;
        };

        if (self.entries.is_empty()) {
            self.entries.append(other_entries);
            return overwritten;
        };

        // Optimization: if all elements in `other` are larger than all elements in `self`, we can just move them over.
        if (cmp::compare(&self.entries.borrow(self.entries.length() - 1).key, &other_entries.borrow(0).key).is_lt()) {
            self.entries.append(other_entries);
            return overwritten;
        };

        // In O(n), traversing from the back, build reverse sorted result, and then reverse it back
        let reverse_result = vector::empty();
        let cur_i = self.entries.length() - 1;
        let other_i = other_entries.length() - 1;

        // after the end of the loop, other_entries is empty, and any leftover is in entries
        loop {
            let ord = cmp::compare(&self.entries[cur_i].key, &other_entries[other_i].key);
            if (ord.is_gt()) {
                reverse_result.push_back(self.entries.pop_back());
                if (cur_i == 0) {
                    // make other_entries empty, and rest in entries.
                    // TODO cannot use mem::swap until it is public/released
                    // mem::swap(&mut self.entries, &mut other_entries);
                    self.entries.append(other_entries);
                    break;
                } else {
                    cur_i -= 1;
                };
            } else {
                // is_lt or is_eq
                if (ord.is_eq()) {
                    // we skip the entries one, and below put in the result one from other.
                    overwritten.push_back(self.entries.pop_back());

                    if (cur_i == 0) {
                        // make other_entries empty, and rest in entries.
                        // TODO cannot use mem::swap until it is public/released
                        // mem::swap(&mut self.entries, &mut other_entries);
                        self.entries.append(other_entries);
                        break;
                    } else {
                        cur_i -= 1;
                    };
                };

                reverse_result.push_back(other_entries.pop_back());
                if (other_i == 0) {
                    other_entries.destroy_empty();
                    break;
                } else {
                    other_i -= 1;
                };
            };
        };

        self.entries.reverse_append(reverse_result);

        overwritten
    }

    /// Splits the collection into two, such to leave `self` with `at` number of elements.
    /// Returns a newly allocated map containing the elements in the range [at, len).
    /// After the call, the original map will be left containing the elements [0, at).
    public fun trim<K, V>(self: &mut OrderedMap<K, V>, at: u64): OrderedMap<K, V> {
        let rest = self.entries.trim(at);

        OrderedMap::SortedVectorMap {
            entries: rest
        }
    }

    public fun borrow_front<K, V>(self: &OrderedMap<K, V>): (&K, &V) {
        let entry = self.entries.borrow(0);
        (&entry.key, &entry.value)
    }

    public fun borrow_back<K, V>(self: &OrderedMap<K, V>): (&K, &V) {
        let entry = self.entries.borrow(self.entries.length() - 1);
        (&entry.key, &entry.value)
    }

    public fun pop_front<K, V>(self: &mut OrderedMap<K, V>): (K, V) {
        let Entry { key, value } = self.entries.remove(0);
        (key, value)
    }

    public fun pop_back<K, V>(self: &mut OrderedMap<K, V>): (K, V) {
        let Entry { key, value } = self.entries.pop_back();
        (key, value)
    }

    public fun prev_key<K: copy, V>(self: &OrderedMap<K, V>, key: &K): Option<K> {
        let it = self.lower_bound(key);
        if (it.iter_is_begin(self)) {
            option::none()
        } else {
            option::some(*it.iter_prev(self).iter_borrow_key(self))
        }
    }

    public fun next_key<K: copy, V>(self: &OrderedMap<K, V>, key: &K): Option<K> {
        let it = self.lower_bound(key);
        if (it.iter_is_end(self)) {
            option::none()
        } else {
            let cur_key = it.iter_borrow_key(self);
            if (key == cur_key) {
                let it = it.iter_next(self);
                if (it.iter_is_end(self)) {
                    option::none()
                } else {
                    option::some(*it.iter_borrow_key(self))
                }
            } else {
                option::some(*cur_key)
            }
        }
    }

    // TODO: see if it is more understandable if iterator points between elements,
    // and there is iter_borrow_next and iter_borrow_prev, and provide iter_insert.

    /// Returns an iterator pointing to the first element that is greater or equal to the provided
    /// key, or an end iterator if such element doesn't exist.
    public(friend) fun lower_bound<K, V>(self: &OrderedMap<K, V>, key: &K): IteratorPtr {
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
    public(friend) fun find<K, V>(self: &OrderedMap<K, V>, key: &K): IteratorPtr {
        let lower_bound = self.lower_bound(key);
        if (lower_bound.iter_is_end(self)) {
            lower_bound
        } else if (lower_bound.iter_borrow_key(self) == key) {
            lower_bound
        } else {
            self.new_end_iter()
        }
    }

    /// Returns the begin iterator.
    public(friend) fun new_begin_iter<K, V>(self: &OrderedMap<K, V>): IteratorPtr {
        if (self.is_empty()) {
            return IteratorPtr::End;
        };

        new_iter(0)
    }

    /// Returns the end iterator.
    public(friend) fun new_end_iter<K, V>(self: &OrderedMap<K, V>): IteratorPtr {
        IteratorPtr::End
    }

    // ========== Section for methods opearting on iterators ========
    // Note: After any modifications to the map, do not use any of the iterators obtained beforehand.
    // Operations on iterators after map is modified are unexpected/incorrect.

    /// Returns the next iterator, or none if already at the end iterator.
    /// Note: Requires that the map is not changed after the input iterator is generated.
    public(friend) fun iter_next<K, V>(self: IteratorPtr, map: &OrderedMap<K, V>): IteratorPtr {
        assert!(!self.iter_is_end(map), error::invalid_argument(EITER_OUT_OF_BOUNDS));

        let index = self.index + 1;
        if (index < map.entries.length()) {
            new_iter(index)
        } else {
            map.new_end_iter()
        }
    }

    /// Returns the previous iterator, or none if already at the begin iterator.
    /// Note: Requires that the map is not changed after the input iterator is generated.
    public(friend) fun iter_prev<K, V>(self: IteratorPtr, map: &OrderedMap<K, V>): IteratorPtr {
        assert!(!self.iter_is_begin(map), error::invalid_argument(EITER_OUT_OF_BOUNDS));

        let index = if (self is IteratorPtr::End) {
            map.entries.length() - 1
        } else {
            self.index - 1
        };

        new_iter(index)
    }

    /// Returns whether the iterator is a begin iterator.
    public(friend) fun iter_is_begin<K, V>(self: &IteratorPtr, map: &OrderedMap<K, V>): bool {
        if (self is IteratorPtr::End) {
            map.is_empty()
        } else {
            self.index == 0
        }
    }

    /// Returns true iff the iterator is a begin iterator from a non-empty collection.
    /// (I.e. if iterator points to a valid element)
    /// This method doesn't require having access to map, unlike iter_is_begin.
    public(friend) fun iter_is_begin_from_non_empty(self: &IteratorPtr): bool {
        if (self is IteratorPtr::End) {
            false
        } else {
            self.index == 0
        }
    }

    /// Returns whether the iterator is an end iterator.
    public(friend) fun iter_is_end<K, V>(self: &IteratorPtr, _map: &OrderedMap<K, V>): bool {
        self is IteratorPtr::End
    }

    /// Borrows the key given iterator points to.
    /// Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
    /// Note: Requires that the map is not changed after the input iterator is generated.
    public(friend) fun iter_borrow_key<K, V>(self: &IteratorPtr, map: &OrderedMap<K, V>): &K {
        assert!(!(self is IteratorPtr::End), error::invalid_argument(EITER_OUT_OF_BOUNDS));

        &map.entries.borrow(self.index).key
    }

    /// Borrows the value given iterator points to.
    /// Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
    /// Note: Requires that the map is not changed after the input iterator is generated.
    public(friend) fun iter_borrow<K, V>(self: IteratorPtr, map: &OrderedMap<K, V>): &V {
        assert!(!(self is IteratorPtr::End), error::invalid_argument(EITER_OUT_OF_BOUNDS));
        &map.entries.borrow(self.index).value
    }

    /// Mutably borrows the value iterator points to.
    /// Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
    /// Note: Requires that the map is not changed after the input iterator is generated.
    public(friend) fun iter_borrow_mut<K, V>(self: IteratorPtr, map: &mut OrderedMap<K, V>): &mut V {
        assert!(!(self is IteratorPtr::End), error::invalid_argument(EITER_OUT_OF_BOUNDS));
        &mut map.entries.borrow_mut(self.index).value
    }

    /// Removes (key, value) pair iterator points to, returning the previous value.
    /// Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
    /// Note: Requires that the map is not changed after the input iterator is generated.
    public(friend) fun iter_remove<K: drop, V>(self: IteratorPtr, map: &mut OrderedMap<K, V>): V {
        assert!(!(self is IteratorPtr::End), error::invalid_argument(EITER_OUT_OF_BOUNDS));

        let Entry { key: _, value } = map.entries.remove(self.index);
        value
    }

    /// Replaces the value iterator is pointing to, returning the previous value.
    /// Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
    /// Note: Requires that the map is not changed after the input iterator is generated.
    public(friend) fun iter_replace<K: copy + drop, V>(self: IteratorPtr, map: &mut OrderedMap<K, V>, value: V): V {
        assert!(!(self is IteratorPtr::End), error::invalid_argument(EITER_OUT_OF_BOUNDS));

        // TODO once mem::replace is public/released, update to:
        // let entry = map.entries.borrow_mut(self.index);
        // mem::replace(&mut entry.value, value)
        let key = map.entries[self.index].key;
        let Entry {
            key: _,
            value: prev_value,
        } = map.entries.replace(self.index, Entry { key, value });
        prev_value
    }

    /// Add key/value pair to the map, at the iterator position (before the element at the iterator position).
    /// Aborts with ENEW_KEY_NOT_IN_ORDER is key is not larger than the key before the iterator,
    /// or smaller than the key at the iterator position.
    public(friend) fun iter_add<K, V>(self: IteratorPtr, map: &mut OrderedMap<K, V>, key: K, value: V) {
        let len = map.entries.length();
        let insert_index = if (self is IteratorPtr::End) {
            len
        } else {
            self.index
        };

        if (insert_index > 0) {
            assert!(cmp::compare(&map.entries[insert_index - 1].key, &key).is_lt(), error::invalid_argument(ENEW_KEY_NOT_IN_ORDER))
        };

        if (insert_index < len) {
            assert!(cmp::compare(&key, &map.entries[insert_index].key).is_lt(), error::invalid_argument(ENEW_KEY_NOT_IN_ORDER))
        };

        map.entries.insert(insert_index, Entry { key, value });
    }

    /// Destroys empty map.
    /// Aborts if `self` is not empty.
    public fun destroy_empty<K, V>(self: OrderedMap<K, V>) {
        let OrderedMap::SortedVectorMap { entries } = self;
        // assert!(entries.is_empty(), E_NOT_EMPTY);
        entries.destroy_empty();
    }

    // ========= Section with views and inline for-loop methods =======

    /// Return all keys in the map. This requires keys to be copyable.
    public fun keys<K: copy, V>(self: &OrderedMap<K, V>): vector<K> {
        self.entries.map_ref(|e| {
            let e: &Entry<K, V> = e;
            e.key
        })
    }

    /// Return all values in the map. This requires values to be copyable.
    public fun values<K, V: copy>(self: &OrderedMap<K, V>): vector<V> {
        self.entries.map_ref(|e| {
            let e: &Entry<K, V> = e;
            e.value
        })
    }

    /// Transform the map into two vectors with the keys and values respectively
    /// Primarily used to destroy a map
    public fun to_vec_pair<K, V>(self: OrderedMap<K, V>): (vector<K>, vector<V>) {
        let keys: vector<K> = vector::empty();
        let values: vector<V> = vector::empty();
        let OrderedMap::SortedVectorMap { entries } = self;
        entries.for_each(|e| {
            let Entry { key, value } = e;
            keys.push_back(key);
            values.push_back(value);
        });
        (keys, values)
    }

    /// For maps that cannot be dropped this is a utility to destroy them
    /// using lambdas to destroy the individual keys and values.
    public inline fun destroy<K, V>(
        self: OrderedMap<K, V>,
        dk: |K|,
        dv: |V|
    ) {
        let (keys, values) = self.to_vec_pair();
        keys.destroy(|_k| dk(_k));
        values.destroy(|_v| dv(_v));
    }

    /// Apply the function to each key-value pair in the map, consuming it.
    public inline fun for_each<K, V>(
        self: OrderedMap<K, V>,
        f: |K, V|
    ) {
        let (keys, values) = self.to_vec_pair();
        keys.zip(values, |k, v| f(k, v));
    }

    /// Apply the function to a reference of each key-value pair in the map.
    ///
    /// Current implementation is O(n * log(n)). After function values will be optimized
    /// to O(n).
    public inline fun for_each_ref<K: copy + drop, V>(self: &OrderedMap<K, V>, f: |&K, &V|) {
        // This implementation is innefficient: O(log(n)) for next_key / borrow lookups every time,
        // but is the only one available through the public API.
        if (!self.is_empty()) {
            let (k, v) = self.borrow_front();
            f(k, v);

            let cur_k = self.next_key(k);
            while (cur_k.is_some()) {
                let k = cur_k.destroy_some();
                f(&k, self.borrow(&k));

                cur_k = self.next_key(&k);
            };
        };

        // TODO: if we make iterator api public update to:
        // let iter = self.new_begin_iter();
        // while (!iter.iter_is_end(self)) {
        //     f(iter.iter_borrow_key(self), iter.iter_borrow(self));
        //     iter = iter.iter_next(self);
        // }

        // TODO: once move supports private functions udpate to:
        // vector::for_each_ref(
        //     &self.entries,
        //     |entry| {
        //         f(&entry.key, &entry.value)
        //     }
        // );
    }

    // TODO: Temporary friend implementaiton, until for_each_ref can be made efficient.
    public(friend) inline fun for_each_ref_friend<K: copy + drop, V>(self: &OrderedMap<K, V>, f: |&K, &V|) {
        let iter = self.new_begin_iter();
        while (!iter.iter_is_end(self)) {
            f(iter.iter_borrow_key(self), iter.iter_borrow(self));
            iter = iter.iter_next(self);
        }
    }

    /// Apply the function to a mutable reference of each key-value pair in the map.
    ///
    /// Current implementation is O(n * log(n)). After function values will be optimized
    /// to O(n).
    public inline fun for_each_mut<K: copy + drop, V>(self: &mut OrderedMap<K, V>, f: |&K, &mut V|) {
        // This implementation is innefficient: O(log(n)) for next_key / borrow lookups every time,
        // but is the only one available through the public API.
        if (!self.is_empty()) {
            let (k, _v) = self.borrow_front();

            let k = *k;
            let done = false;
            while (!done) {
                f(&k, self.borrow_mut(&k));

                let cur_k = self.next_key(&k);
                if (cur_k.is_some()) {
                    k = cur_k.destroy_some();
                } else {
                    done = true;
                }
            };
        };

        // TODO: if we make iterator api public update to:
        // let iter = self.new_begin_iter();
        // while (!iter.iter_is_end(self)) {
        //     let key = *iter.iter_borrow_key(self);
        //     f(key, iter.iter_borrow_mut(self));
        //     iter = iter.iter_next(self);
        // }

        // TODO: once move supports private functions udpate to:
        // vector::for_each_mut(
        //     &mut self.entries,
        //     |entry| {
        //         f(&mut entry.key, &mut entry.value)
        //     }
        // );
    }

    // ========= Section with private methods ===============

    inline fun new_iter(index: u64): IteratorPtr {
        IteratorPtr::Position {
            index: index,
        }
    }

    // return index containing the key, or insert position.
    // I.e. index of first element that has key larger or equal to the passed `key` argument.
    fun binary_search<K, V>(key: &K, entries: &vector<Entry<K, V>>, start: u64, end: u64): u64 {
        let l = start;
        let r = end;
        while (l != r) {
            let mid = l + ((r - l) >> 1);
            let comparison = cmp::compare(&entries.borrow(mid).key, key);
            if (comparison.is_lt()) {
                l = mid + 1;
            } else {
                r = mid;
            };
        };
        l
    }

    // see if useful, and add
    //
    // public fun iter_num_below<K, V>(self: IteratorPtr, map: &OrderedMap<K, V>): u64 {
    //     if (self.iter_is_end()) {
    //         map.entries.length()
    //     } else {
    //         self.index
    //     }
    // }

    spec module {
        pragma verify = true;
    }

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
            assert!(cmp::compare(&self.entries.borrow(i).key, &self.entries.borrow(i - 1).key).is_gt(), 1);
            i += 1;
        };
    }

    #[test_only]
    fun validate_iteration<K: drop + copy + store, V: store>(self: &OrderedMap<K, V>) {
        let expected_num_elements = self.length();
        let num_elements = 0;
        let it = self.new_begin_iter();
        while (!it.iter_is_end(self)) {
            num_elements += 1;
            it = it.iter_next(self);
        };
        assert!(num_elements == expected_num_elements, 2);

        let num_elements = 0;
        let it = self.new_end_iter();
        while (!it.iter_is_begin(self)) {
            it = it.iter_prev(self);
            num_elements += 1;
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
        map.validate_map();
        map.add(1, 1);
        map.validate_map();
        map.add(2, 2);
        map.validate_map();
        let r1 = map.upsert(3, 3);
        map.validate_map();
        assert!(r1 == option::none(), 4);
        map.add(4, 4);
        map.validate_map();
        let r2 = map.upsert(4, 8);
        map.validate_map();
        assert!(r2 == option::some(4), 5);
        map.add(5, 5);
        map.validate_map();
        map.add(6, 6);
        map.validate_map();

        map.remove(&5);
        map.validate_map();
        map.remove(&4);
        map.validate_map();
        map.remove(&1);
        map.validate_map();
        map.remove(&3);
        map.validate_map();
        map.remove(&2);
        map.validate_map();
        map.remove(&6);
        map.validate_map();

        map.destroy_empty();
    }

    #[test]
    fun test_add_remove_many() {
        let map = new<u64, u64>();

        assert!(map.length() == 0, 0);
        assert!(!map.contains(&3), 1);
        map.add(3, 1);
        assert!(map.length() == 1, 2);
        assert!(map.contains(&3), 3);
        assert!(map.borrow(&3) == &1, 4);
        *map.borrow_mut(&3) = 2;
        assert!(map.borrow(&3) == &2, 5);

        assert!(!map.contains(&2), 6);
        map.add(2, 5);
        assert!(map.length() == 2, 7);
        assert!(map.contains(&2), 8);
        assert!(map.borrow(&2) == &5, 9);
        *map.borrow_mut(&2) = 9;
        assert!(map.borrow(&2) == &9, 10);

        map.remove(&2);
        assert!(map.length() == 1, 11);
        assert!(!map.contains(&2), 12);
        assert!(map.borrow(&3) == &2, 13);

        map.remove(&3);
        assert!(map.length() == 0, 14);
        assert!(!map.contains(&3), 15);

        map.destroy_empty();
    }

    #[test]
    fun test_add_all() {
        let map = new<u64, u64>();

        assert!(map.length() == 0, 0);
        map.add_all(vector[2, 1, 3], vector[20, 10, 30]);

        assert!(map == new_from(vector[1, 2, 3], vector[10, 20, 30]), 1);

        assert!(map.length() == 3, 1);
        assert!(map.borrow(&1) == &10, 2);
        assert!(map.borrow(&2) == &20, 3);
        assert!(map.borrow(&3) == &30, 4);
    }

    #[test]
    #[expected_failure(abort_code = 0x20002, location = Self)] /// EKEY_ALREADY_EXISTS
    fun test_add_all_mismatch() {
        new_from(vector[1, 3], vector[10]);
    }

    #[test]
    fun test_upsert_all() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.upsert_all(vector[7, 2, 3], vector[70, 20, 35]);
        assert!(map == new_from(vector[1, 2, 3, 5, 7], vector[10, 20, 35, 50, 70]), 1);
    }

    #[test]
    #[expected_failure(abort_code = 0x10001, location = Self)] /// EKEY_ALREADY_EXISTS
    fun test_new_from_duplicate() {
        new_from(vector[1, 3, 1, 5], vector[10, 30, 11, 50]);
    }

    #[test]
    #[expected_failure(abort_code = 0x20002, location = Self)] /// EKEY_ALREADY_EXISTS
    fun test_upsert_all_mismatch() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.upsert_all(vector[2], vector[20, 35]);
    }

    #[test]
    fun test_to_vec_pair() {
        let (keys, values) = new_from(vector[3, 1, 5], vector[30, 10, 50]).to_vec_pair();
        assert!(keys == vector[1, 3, 5], 1);
        assert!(values == vector[10, 30, 50], 2);
    }

    #[test]
    fun test_keys() {
        let map = new<u64, u64>();
        assert!(map.keys() == vector[], 0);
        map.add(2, 1);
        map.add(3, 1);

        assert!(map.keys() == vector[2, 3], 0);
    }

    #[test]
    fun test_values() {
        let map = new<u64, u64>();
        assert!(map.values() == vector[], 0);
        map.add(2, 1);
        map.add(3, 2);

        assert!(map.values() == vector[1, 2], 0);
    }

    #[test]
    fun test_for_each_variants() {
        let keys = vector[1, 3, 5];
        let values = vector[10, 30, 50];
        let map = new_from(keys, values);

        let index = 0;
        map.for_each_ref(|k, v| {
            assert!(keys[index] == *k);
            assert!(values[index] == *v);
            index += 1;
        });

        let index = 0;
        map.for_each_mut(|k, v| {
            assert!(keys[index] == *k);
            assert!(values[index] == *v);
            *v += 1;
            index += 1;
        });

        let index = 0;
        map.for_each(|k, v| {
            assert!(keys[index] == k);
            assert!(values[index] + 1 == v);
            index += 1;
        });
    }

    #[test]
    #[expected_failure(abort_code = 0x10001, location = Self)] /// EKEY_ALREADY_EXISTS
    fun test_add_twice() {
        let map = new<u64, u64>();
        map.add(3, 1);
        map.add(3, 1);

        map.remove(&3);
        map.destroy_empty();
    }

    #[test]
    #[expected_failure(abort_code = 0x10002, location = Self)] /// EKEY_NOT_FOUND
    fun test_remove_twice_1() {
        let map = new<u64, u64>();
        map.add(3, 1);
        map.remove(&3);
        map.remove(&3);

        map.destroy_empty();
    }

    #[test]
    #[expected_failure(abort_code = 0x10002, location = Self)] /// EKEY_NOT_FOUND
    fun test_remove_twice_2() {
        let map = new<u64, u64>();
        map.add(3, 1);
        map.add(4, 1);
        map.remove(&3);
        map.remove(&3);

        map.destroy_empty();
    }

    #[test]
    fun test_upsert_test() {
        let map = new<u64, u64>();
        // test adding 3 elements using upsert
        map.upsert<u64, u64>(1, 1);
        map.upsert(2, 2);
        map.upsert(3, 3);

        assert!(map.length() == 3, 0);
        assert!(map.contains(&1), 1);
        assert!(map.contains(&2), 2);
        assert!(map.contains(&3), 3);
        assert!(map.borrow(&1) == &1, 4);
        assert!(map.borrow(&2) == &2, 5);
        assert!(map.borrow(&3) == &3, 6);

        // change mapping 1->1 to 1->4
        map.upsert(1, 4);

        assert!(map.length() == 3, 7);
        assert!(map.contains(&1), 8);
        assert!(map.borrow(&1) == &4, 9);
    }

    #[test]
    fun test_append() {
        {
            let map = new<u16, u16>();
            let other = new();
            map.append(other);
            assert!(map.is_empty(), 0);
        };
        {
            let map = new_from(vector[1, 2], vector[10, 20]);
            let other = new();
            map.append(other);
            assert!(map == new_from(vector[1, 2], vector[10, 20]), 1);
        };
        {
            let map = new();
            let other = new_from(vector[1, 2], vector[10, 20]);
            map.append(other);
            assert!(map == new_from(vector[1, 2], vector[10, 20]), 2);
        };
        {
            let map = new_from(vector[1, 2, 3], vector[10, 20, 30]);
            let other = new_from(vector[4, 5], vector[40, 50]);
            map.append(other);
            assert!(map == new_from(vector[1, 2, 3, 4, 5], vector[10, 20, 30, 40, 50]), 3);
        };
        {
            let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
            let other = new_from(vector[2, 4], vector[20, 40]);
            map.append(other);
            assert!(map == new_from(vector[1, 2, 3, 4, 5], vector[10, 20, 30, 40, 50]), 4);
        };
        {
            let map = new_from(vector[2, 4], vector[20, 40]);
            let other = new_from(vector[1, 3, 5], vector[10, 30, 50]);
            map.append(other);
            assert!(map == new_from(vector[1, 2, 3, 4, 5], vector[10, 20, 30, 40, 50]), 6);
        };
        {
            let map = new_from(vector[1], vector[10]);
            let other = new_from(vector[1], vector[11]);
            map.append(other);
            assert!(map == new_from(vector[1], vector[11]), 7);
        }
    }

    #[test]
    fun test_append_disjoint() {
        let map = new_from(vector[1, 2, 3], vector[10, 20, 30]);
        let other = new_from(vector[4, 5], vector[40, 50]);
        map.append_disjoint(other);
        assert!(map == new_from(vector[1, 2, 3, 4, 5], vector[10, 20, 30, 40, 50]), 1);
    }

    #[test]
    #[expected_failure(abort_code = 0x10001, location = Self)] /// EKEY_ALREADY_EXISTS
    fun test_append_disjoint_abort() {
        let map = new_from(vector[1], vector[10]);
        let other = new_from(vector[1], vector[11]);
        map.append_disjoint(other);
    }

    #[test]
    fun test_trim() {
        let map = new_from(vector[1, 2, 3], vector[10, 20, 30]);
        let rest = map.trim(2);
        assert!(map == new_from(vector[1, 2], vector[10, 20]), 1);
        assert!(rest == new_from(vector[3], vector[30]), 2);
    }

    #[test]
    fun test_non_iterator_ordering() {
        let map = new_from(vector[1, 2, 3], vector[10, 20, 30]);
        assert!(map.prev_key(&1).is_none(), 1);
        assert!(map.next_key(&1) == option::some(2), 1);

        assert!(map.prev_key(&2) == option::some(1), 2);
        assert!(map.next_key(&2) == option::some(3), 3);

        assert!(map.prev_key(&3) == option::some(2), 4);
        assert!(map.next_key(&3).is_none(), 5);

        let (front_k, front_v) = map.borrow_front();
        assert!(front_k == &1, 6);
        assert!(front_v == &10, 7);

        let (back_k, back_v) = map.borrow_back();
        assert!(back_k == &3, 8);
        assert!(back_v == &30, 9);

        let (front_k, front_v) = map.pop_front();
        assert!(front_k == 1, 10);
        assert!(front_v == 10, 11);

        let (back_k, back_v) = map.pop_back();
        assert!(back_k == 3, 12);
        assert!(back_v == 30, 13);
    }

    #[test]
    fun test_replace_key_inplace() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.replace_key_inplace(&5, 6);
        assert!(map == new_from(vector[1, 3, 6], vector[10, 30, 50]), 1);
        map.replace_key_inplace(&3, 4);
        assert!(map == new_from(vector[1, 4, 6], vector[10, 30, 50]), 2);
        map.replace_key_inplace(&1, 0);
        assert!(map == new_from(vector[0, 4, 6], vector[10, 30, 50]), 3);
    }

    #[test]
    #[expected_failure(abort_code = 0x10002, location = Self)] /// EKEY_NOT_FOUND
    fun test_replace_key_inplace_not_found_1() {
        let map = new_from(vector[1, 3, 6], vector[10, 30, 50]);
        map.replace_key_inplace(&4, 5);

    }

    #[test]
    #[expected_failure(abort_code = 0x10002, location = Self)] /// EKEY_NOT_FOUND
    fun test_replace_key_inplace_not_found_2() {
        let map = new_from(vector[1, 3, 6], vector[10, 30, 50]);
        map.replace_key_inplace(&7, 8);
    }

    #[test]
    #[expected_failure(abort_code = 0x10004, location = Self)] /// ENEW_KEY_NOT_IN_ORDER
    fun test_replace_key_inplace_not_in_order_1() {
        let map = new_from(vector[1, 3, 6], vector[10, 30, 50]);
        map.replace_key_inplace(&3, 7);
    }

    #[test]
    #[expected_failure(abort_code = 0x10004, location = Self)] /// ENEW_KEY_NOT_IN_ORDER
    fun test_replace_key_inplace_not_in_order_2() {
        let map = new_from(vector[1, 3, 6], vector[10, 30, 50]);
        map.replace_key_inplace(&1, 3);
    }

    #[test]
    #[expected_failure(abort_code = 0x10004, location = Self)] /// ENEW_KEY_NOT_IN_ORDER
    fun test_replace_key_inplace_not_in_order_3() {
        let map = new_from(vector[1, 3, 6], vector[10, 30, 50]);
        map.replace_key_inplace(&6, 3);
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = Self)] /// EITER_OUT_OF_BOUNDS
    public fun test_iter_end_next_abort() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.new_end_iter().iter_next(&map);
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = Self)] /// EITER_OUT_OF_BOUNDS
    public fun test_iter_end_borrow_key_abort() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.new_end_iter().iter_borrow_key(&map);
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = Self)] /// EITER_OUT_OF_BOUNDS
    public fun test_iter_end_borrow_abort() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.new_end_iter().iter_borrow(&map);
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = Self)] /// EITER_OUT_OF_BOUNDS
    public fun test_iter_end_borrow_mut_abort() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.new_end_iter().iter_borrow_mut(&mut map);
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = Self)] /// EITER_OUT_OF_BOUNDS
    public fun test_iter_begin_prev_abort() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.new_begin_iter().iter_prev(&map);
    }

    #[test]
    public fun test_iter_is_begin_from_non_empty() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        let iter = map.new_begin_iter();
        assert!(iter.iter_is_begin(&map), 1);
        assert!(iter.iter_is_begin_from_non_empty(), 1);

        iter = iter.iter_next(&map);
        assert!(!iter.iter_is_begin(&map), 1);
        assert!(!iter.iter_is_begin_from_non_empty(), 1);

        let map = new<u64, u64>();
        let iter = map.new_begin_iter();
        assert!(iter.iter_is_begin(&map), 1);
        assert!(!iter.iter_is_begin_from_non_empty(), 1);
    }

    #[test]
    public fun test_iter_remove() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.new_begin_iter().iter_next(&map).iter_remove(&mut map);
        assert!(map == new_from(vector[1, 5], vector[10, 50]), 1);
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = Self)] /// EITER_OUT_OF_BOUNDS
        public fun test_iter_remove_abort() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.new_end_iter().iter_remove(&mut map);
    }

    #[test]
    public fun test_iter_replace() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.new_begin_iter().iter_next(&map).iter_replace(&mut map, 35);
        assert!(map == new_from(vector[1, 3, 5], vector[10, 35, 50]), 1);
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = Self)] /// EITER_OUT_OF_BOUNDS
        public fun test_iter_replace_abort() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.new_end_iter().iter_replace(&mut map, 35);
    }

    #[test]
    public fun test_iter_add() {
        {
            let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
            map.new_begin_iter().iter_add(&mut map, 0, 5);
            assert!(map == new_from(vector[0, 1, 3, 5], vector[5, 10, 30, 50]), 1);
        };
        {
            let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
            map.new_begin_iter().iter_next(&map).iter_add(&mut map, 2, 20);
            assert!(map == new_from(vector[1, 2, 3, 5], vector[10, 20, 30, 50]), 2);
        };
        {
            let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
            map.new_end_iter().iter_add(&mut map, 6, 60);
            assert!(map == new_from(vector[1, 3, 5, 6], vector[10, 30, 50, 60]), 3);
        };
        {
            let map = new();
            map.new_end_iter().iter_add(&mut map, 1, 10);
            assert!(map == new_from(vector[1], vector[10]), 4);
        };
    }

    #[test]
    #[expected_failure(abort_code = 0x10004, location = Self)] /// ENEW_KEY_NOT_IN_ORDER
    public fun test_iter_add_abort_1() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.new_begin_iter().iter_add(&mut map, 1, 5);
    }

    #[test]
    #[expected_failure(abort_code = 0x10004, location = Self)] /// ENEW_KEY_NOT_IN_ORDER
    public fun test_iter_add_abort_2() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.new_end_iter().iter_add(&mut map, 5, 55);
    }

    #[test]
    #[expected_failure(abort_code = 0x10004, location = Self)] /// ENEW_KEY_NOT_IN_ORDER
    public fun test_iter_add_abort_3() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.new_begin_iter().iter_next(&map).iter_add(&mut map, 1, 15);
    }

    #[test]
    #[expected_failure(abort_code = 0x10004, location = Self)] /// ENEW_KEY_NOT_IN_ORDER
    public fun test_iter_add_abort_4() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        map.new_begin_iter().iter_next(&map).iter_add(&mut map, 3, 25);
    }

    #[test]
	public fun test_ordered_map_append_2() {
        let map = new_from(vector[1, 2], vector[10, 20]);
        let other = new_from(vector[1, 2], vector[100, 200]);
        map.append(other);
        assert!(map == new_from(vector[1, 2], vector[100, 200]));
    }

    #[test]
	public fun test_ordered_map_append_3() {
        let map = new_from(vector[1, 2, 3, 4, 5], vector[10, 20, 30, 40, 50]);
        let other = new_from(vector[2, 4], vector[200, 400]);
        map.append(other);
        assert!(map == new_from(vector[1, 2, 3, 4, 5], vector[10, 200, 30, 400, 50]));
    }

    #[test]
	public fun test_ordered_map_append_4() {
        let map = new_from(vector[3, 4, 5, 6, 7], vector[30, 40, 50, 60, 70]);
        let other = new_from(vector[1, 2, 4, 6], vector[100, 200, 400, 600]);
        map.append(other);
        assert!(map == new_from(vector[1, 2, 3, 4, 5, 6, 7], vector[100, 200, 30, 400, 50, 600, 70]));
    }

    #[test]
	public fun test_ordered_map_append_5() {
        let map = new_from(vector[1, 3, 5], vector[10, 30, 50]);
        let other = new_from(vector[0, 2, 4, 6], vector[0, 200, 400, 600]);
        map.append(other);
        aptos_std::debug::print(&map);
        assert!(map == new_from(vector[0, 1, 2, 3, 4, 5, 6], vector[0, 10, 200, 30, 400, 50, 600]));
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
            assert!(!it.iter_is_end(&map), 6);
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

    #[verify_only]
    fun test_verify_borrow_front_key() {
        let keys: vector<u64> = vector[1, 2, 3];
        let values: vector<u64> = vector[4, 5, 6];
        let map = new_from(keys, values);
        let (key, value) = map.borrow_front();
        spec {
            assert keys[0] == 1;
            assert vector::spec_contains(keys, 1);
            assert spec_contains_key(map, key);
            assert spec_get(map, key) == value;
            assert key == (1 as u64);
        };
    }

    #[verify_only]
    fun test_verify_borrow_back_key() {
        let keys: vector<u64> = vector[1, 2, 3];
        let values: vector<u64> = vector[4, 5, 6];
        let map = new_from(keys, values);
        let (key, value) = map.borrow_back();
        spec {
            assert keys[2] == 3;
            assert vector::spec_contains(keys, 3);
            assert spec_contains_key(map, key);
            assert spec_get(map, key) == value;
            assert key == (3 as u64);
        };
    }

    #[verify_only]
    fun test_verify_upsert() {
        let keys: vector<u64> = vector[1, 2, 3];
        let values: vector<u64> = vector[4, 5, 6];
        let map = new_from(keys, values);
        spec {
            assert spec_len(map) == 3;
        };
        let (_key, _value) = map.borrow_back();
        let result_1 = map.upsert(4, 5);
        spec {
            assert spec_contains_key(map, 4);
            assert spec_get(map, 4) == 5;
            assert option::spec_is_none(result_1);
            assert spec_len(map) == 4;
        };
        let result_2 = map.upsert(4, 6);
        spec {
            assert spec_contains_key(map, 4);
            assert spec_get(map, 4) == 6;
            assert option::spec_is_some(result_2);
            assert option::spec_borrow(result_2) == 5;
        };
        spec {
            assert keys[0] == 1;
            assert spec_contains_key(map, 1);
            assert spec_get(map, 1) == 4;
        };
        let v = map.remove(&1);
        spec {
            assert v == 4;
        };
        map.remove(&2);
        map.remove(&3);
        map.remove(&4);
        spec {
            assert !spec_contains_key(map, 1);
            assert !spec_contains_key(map, 2);
            assert !spec_contains_key(map, 3);
            assert !spec_contains_key(map, 4);
            assert spec_len(map) == 0;
        };
        map.destroy_empty();
    }

    #[verify_only]
    fun test_verify_next_key() {
        let keys: vector<u64> = vector[1, 2, 3];
        let values: vector<u64> = vector[4, 5, 6];
        let map = new_from(keys, values);
        let result_1 = map.next_key(&3);
        spec {
            assert option::spec_is_none(result_1);
        };
        let result_2 = map.next_key(&1);
        spec {
            assert keys[0] == 1;
            assert spec_contains_key(map, 1);
            assert keys[1] == 2;
            assert spec_contains_key(map, 2);
            assert option::spec_is_some(result_2);
            assert option::spec_borrow(result_2) == 2;
        };
    }

    #[verify_only]
    fun test_verify_prev_key() {
        let keys: vector<u64> = vector[1, 2, 3];
        let values: vector<u64> = vector[4, 5, 6];
        let map = new_from(keys, values);
        let result_1 = map.prev_key(&1);
        spec {
            assert option::spec_is_none(result_1);
        };
        let result_2 = map.prev_key(&3);
        spec {
            assert keys[0] == 1;
            assert spec_contains_key(map, 1);
            assert keys[1] == 2;
            assert spec_contains_key(map, 2);
            assert option::spec_is_some(result_2);
        };
    }

     #[verify_only]
     fun test_aborts_if_new_from_1(): OrderedMap<u64, u64> {
        let keys: vector<u64> = vector[1, 2, 3, 1];
        let values: vector<u64> = vector[4, 5, 6, 7];
        spec {
            assert keys[0] == 1;
            assert keys[3] == 1;
        };
        let map = new_from(keys, values);
        map
     }

     spec test_aborts_if_new_from_1 {
        aborts_if true;
     }

     #[verify_only]
     fun test_aborts_if_new_from_2(keys: vector<u64>, values: vector<u64>): OrderedMap<u64, u64> {
        let map = new_from(keys, values);
        map
     }

     spec test_aborts_if_new_from_2 {
        aborts_if exists i in 0..len(keys), j in 0..len(keys) where i != j : keys[i] == keys[j];
        aborts_if len(keys) != len(values);
     }

     #[verify_only]
     fun test_aborts_if_remove(map: &mut OrderedMap<u64, u64>) {
        map.remove(&1);
     }

     spec test_aborts_if_remove {
        aborts_if !spec_contains_key(map, 1);
     }

}
