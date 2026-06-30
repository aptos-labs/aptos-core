spec aptos_framework::big_ordered_map {

    spec BigOrderedMap {
        pragma intrinsic = map,
            map_new = new,
            map_destroy_empty = destroy_empty,
            map_has_key = contains,
            map_add_no_override = add,
            map_borrow = borrow,
            map_borrow_mut = borrow_mut,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains_key,
            map_is_empty = is_empty;
    }

    spec native fun spec_len<K, V>(t: BigOrderedMap<K, V>): num;
    spec native fun spec_contains_key<K, V>(t: BigOrderedMap<K, V>, k: K): bool;
    spec native fun spec_set<K, V>(t: BigOrderedMap<K, V>, k: K, v: V): BigOrderedMap<K, V>;
    spec native fun spec_remove<K, V>(t: BigOrderedMap<K, V>, k: K): BigOrderedMap<K, V>;
    spec native fun spec_get<K, V>(t: BigOrderedMap<K, V>, k: K): V;


    spec new_with_config {
        pragma verify = false;
        pragma opaque;
        aborts_if inner_max_degree != 0
            && (inner_max_degree < 4 || (inner_max_degree as u64) > 4096);
        aborts_if leaf_max_degree != 0
            && (leaf_max_degree < 3 || (leaf_max_degree as u64) > 4096);
        ensures spec_len(result) == 0;
        ensures forall k: K: !spec_contains_key(result, k);
    }

    spec new {
        pragma intrinsic;
    }

    spec new_with_reusable {
        pragma verify = false;
        pragma opaque;
        aborts_if false;
        ensures spec_len(result) == 0;
        ensures forall k: K: !spec_contains_key(result, k);
    }

    spec new_with_type_size_hints {
        pragma verify = false;
        pragma opaque;
        aborts_if false;
        ensures spec_len(result) == 0;
        ensures forall k: K: !spec_contains_key(result, k);
    }

    spec borrow {
        pragma intrinsic;
    }

    spec borrow_mut {
        pragma intrinsic;
    }

    spec contains {
        pragma intrinsic;
    }

    spec destroy_empty {
        pragma intrinsic;
    }

    spec add {
        pragma intrinsic;
    }

    spec remove {
        pragma opaque;
        pragma verify = false;
        aborts_if !spec_contains_key(self, key);
        ensures !spec_contains_key(self, key);
        ensures spec_get(old(self), key) == result;
        ensures spec_len(old(self)) == spec_len(self) + 1;
        ensures spec_unchanged_except_at(self, key);
        // ensures forall k: K where k != key: spec_contains_key(self, k) ==> spec_get(self, k) == spec_get(old(self), k);
        // ensures forall k: K where k != key: spec_contains_key(old(self), k) == spec_contains_key(self, k);
    }

    spec fun spec_unchanged_except_at<K: drop + copy + store, V: store>(
        self: &mut BigOrderedMap<K, V>, key: &K
    ): bool {
        (forall k: K where k != key:
            spec_contains_key(self, k) == spec_contains_key(old(self), k))
        && (forall k: K where k != key && spec_contains_key(old(self), k):
            spec_get(self, k) == spec_get(old(self), k))
    }

    spec remove_or_none<K: drop + copy + store, V: store>(
        self: &mut BigOrderedMap<K, V>, key: &K
    ): Option<V> {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        // Hit: key was present
        ensures spec_contains_key(old(self), key) ==> (
            option::is_some(result)
            && option::spec_borrow(result) == spec_get(old(self), key)
            && !spec_contains_key(self, key)
            && spec_len(self) == spec_len(old(self)) - 1
        );
        // Miss: key was absent — map unchanged
        ensures !spec_contains_key(old(self), key) ==> (
            option::is_none(result)
            && spec_len(self) == spec_len(old(self))
        );
        ensures spec_unchanged_except_at(self, key);
    }

    spec is_empty {
        pragma intrinsic;
    }

    spec iter_is_end {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures result == (self is IteratorPtr::End<K>);
    }

    spec iter_borrow {
        pragma opaque;
        pragma verify = false;
        aborts_if iter_is_end(self, map);
        ensures result == spec_get(map, self.key);
    }

    // Body also asserts constant_kv_size OR bcs::constant_serialized_size<V>().is_some()
    // which is not expressible from spec context. Caller-side, iter_is_end is what's discharged.
    spec iter_borrow_mut {
        pragma opaque;
        pragma verify = false;
        aborts_if iter_is_end(self, map);
        ensures result == spec_get(map, self.key);
    }

    spec iter_is_begin {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        // self is End: returns true iff map is empty (End acts as both begin and end on []).
        ensures (self is IteratorPtr::End<K>) ==> (result <==> spec_len(map) == 0);
        // self is Some: returns true iff self.key is the smallest key currently in map.
        ensures !(self is IteratorPtr::End<K>) ==> (result <==>
            (spec_contains_key(map, self.key)
                && (forall k: K where spec_contains_key(map, k) && k != self.key:
                    std::cmp::compare(self.key, k) == std::cmp::Ordering::Less)));
    }

    // Returns the iterator pointing to the smallest key K in self with K >= input
    // key (compare not Less), or End if no such key exists.
    spec internal_lower_bound {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        // End iff no key >= input exists (all keys are Less than input).
        ensures iter_is_end(result, self) <==>
            (forall k: K where spec_contains_key(self, k):
                std::cmp::compare(k, key) == std::cmp::Ordering::Less);
        // Otherwise, result.key is in the map, >= input, and the smallest such.
        ensures !iter_is_end(result, self) ==> spec_contains_key(self, result.key);
        ensures !iter_is_end(result, self) ==>
            std::cmp::compare(result.key, key) != std::cmp::Ordering::Less;
        ensures !iter_is_end(result, self) ==>
            (forall k: K where spec_contains_key(self, k) && std::cmp::compare(k, key) != std::cmp::Ordering::Less:
                std::cmp::compare(result.key, k) != std::cmp::Ordering::Greater);
    }

    spec iter_borrow_key {
        pragma opaque;
        pragma verify = false;
        aborts_if self is IteratorPtr::End<K>;
        ensures result == self.key;
    }

    spec allocate_spare_slots {
        pragma verify = false;
        pragma opaque;
    }

    spec validate_size_and_init_max_degrees {
        pragma verify = false;
        pragma opaque;
    }

    spec validate_dynamic_size_and_init_max_degrees {
        pragma verify = false;
        pragma opaque;
    }

    spec validate_static_size_and_init_max_degrees {
        pragma verify = false;
        pragma opaque;
    }

    spec keys {
        pragma verify = false;
        pragma opaque;
        ensures forall k: K: vector::spec_contains(result, k) <==> spec_contains_key(self, k);
    }

    spec new_from<K: drop + copy + store, V: store>(keys: vector<K>, values: vector<V>): BigOrderedMap<K, V> {
        pragma opaque;
        pragma verify = false;
        aborts_if exists i in 0..len(keys), j in 0..len(keys) where i != j : keys[i] == keys[j];
        aborts_if len(keys) != len(values);
        ensures forall k: K {spec_contains_key(result, k)} : vector::spec_contains(keys,k) <==> spec_contains_key(result, k);
        ensures forall i in 0..len(keys) : spec_get(result, keys[i]) == values[i];
        ensures spec_len(result) == len(keys);
    }

    spec upsert {
        pragma opaque;
        pragma verify = false;
        ensures !spec_contains_key(old(self), key) ==> option::is_none(result);
        ensures spec_contains_key(self, key);
        ensures spec_get(self, key) == value;
        ensures spec_contains_key(old(self), key) ==> ((option::is_some(result)) && (option::spec_borrow(result) == spec_get(old(
            self), key)));
        ensures !spec_contains_key(old(self), key) ==> spec_len(old(self)) + 1 == spec_len(self);
        ensures spec_contains_key(old(self), key) ==> spec_len(old(self)) == spec_len(self);
        ensures spec_unchanged_except_at(self, key);
    }

    spec add_all {
        pragma opaque;
        pragma verify = false;
    }

    spec borrow_front<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>): (K, &V) {
        pragma opaque;
        pragma verify = false;
        ensures spec_contains_key(self, result_1);
        ensures spec_get(self, result_1) == result_2;
        ensures forall k: K where k != result_1: spec_contains_key(self, k) ==>
        std::cmp::compare(result_1, k) == std::cmp::Ordering::Less;
    }

    spec front_key<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>): K {
        pragma opaque;
        pragma verify = false;
        aborts_if spec_len(self) == 0;
        ensures spec_contains_key(self, result);
        ensures forall k: K where k != result: spec_contains_key(self, k) ==>
            std::cmp::compare(result, k) == std::cmp::Ordering::Less;
    }

    spec borrow_back {
        pragma opaque;
        pragma verify = false;
        ensures spec_contains_key(self, result_1);
        ensures spec_get(self, result_1) == result_2;
        ensures forall k: K where k != result_1: spec_contains_key(self, k) ==>
        std::cmp::compare(result_1, k) == std::cmp::Ordering::Greater;
    }

    spec back_key<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>): K {
        pragma opaque;
        pragma verify = false;
        aborts_if spec_len(self) == 0;
        ensures spec_contains_key(self, result);
        ensures forall k: K where k != result: spec_contains_key(self, k) ==>
            std::cmp::compare(result, k) == std::cmp::Ordering::Greater;
    }

    spec pop_front<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>): (K, V) {
        pragma opaque;
        pragma verify = false;
        ensures spec_contains_key(old(self), result_1);
        ensures result_2 == spec_get(old(self), result_1);
        ensures !spec_contains_key(self, result_1);
        ensures spec_len(self) == spec_len(old(self)) - 1;
        ensures spec_unchanged_except_at(self, result_1);
        ensures forall k: K where spec_contains_key(old(self), k) && k != result_1:
            std::cmp::compare(result_1, k) == std::cmp::Ordering::Less;
    }

    spec pop_back<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>): (K, V) {
        pragma opaque;
        pragma verify = false;
        ensures spec_contains_key(old(self), result_1);
        ensures result_2 == spec_get(old(self), result_1);
        ensures !spec_contains_key(self, result_1);
        ensures spec_len(self) == spec_len(old(self)) - 1;
        ensures spec_unchanged_except_at(self, result_1);
        ensures forall k: K where spec_contains_key(old(self), k) && k != result_1:
            std::cmp::compare(result_1, k) == std::cmp::Ordering::Greater;
    }

    spec prev_key<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, key: &K): Option<K> {
        pragma opaque;
        pragma verify = false;
        ensures result == std::option::spec_none() <==>
        (forall k: K {spec_contains_key(self, k)} where spec_contains_key(self, k)
        && k != key: std::cmp::compare(key, k) == std::cmp::Ordering::Less);
        ensures result.is_some() <==>
            spec_contains_key(self, option::spec_borrow(result)) &&
            (std::cmp::compare(option::spec_borrow(result), key) == std::cmp::Ordering::Less)
            && (forall k: K {spec_contains_key(self, k), std::cmp::compare(option::spec_borrow(result), k), std::cmp::compare(key, k)} where k != option::spec_borrow(result): ((spec_contains_key(self, k) &&
            std::cmp::compare(k, key) == std::cmp::Ordering::Less)) ==>
            std::cmp::compare(option::spec_borrow(result), k) == std::cmp::Ordering::Greater);
    }


    spec next_key<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, key: &K): Option<K>  {
        pragma opaque;
        pragma verify = false;
        ensures result == std::option::spec_none() <==>
        (forall k: K {spec_contains_key(self, k)} where spec_contains_key(self, k) && k != key:
        std::cmp::compare(key, k) == std::cmp::Ordering::Greater);
        ensures result.is_some() <==>
            spec_contains_key(self, option::spec_borrow(result)) &&
            (std::cmp::compare(option::spec_borrow(result), key) == std::cmp::Ordering::Greater)
            && (forall k: K {spec_contains_key(self, k)} where k != option::spec_borrow(result): ((spec_contains_key(self, k) &&
            std::cmp::compare(k, key) == std::cmp::Ordering::Greater)) ==>
            std::cmp::compare(option::spec_borrow(result), k) == std::cmp::Ordering::Less);
    }


    spec internal_find {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures iter_is_end(result, self) <==> !spec_contains_key(self, key);
        ensures !iter_is_end(result, self) ==> result.key == key;
    }

    spec internal_new_begin_iter {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures iter_is_end(result, self) <==> spec_len(self) == 0;
        ensures !iter_is_end(result, self) ==> spec_contains_key(self, result.key);
        // result.key is the smallest key in the map.
        ensures !iter_is_end(result, self) ==>
            (forall k: K where spec_contains_key(self, k) && k != result.key:
                std::cmp::compare(result.key, k) == std::cmp::Ordering::Less);
    }

    spec internal_new_end_iter {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures result is IteratorPtr::End<K>;
    }

    spec iter_next {
        pragma opaque;
        pragma verify = false;
        aborts_if iter_is_end(self, map);
    }

    spec iter_prev {
        pragma opaque;
        pragma verify = false;
        aborts_if iter_is_begin(self, map);
    }

    spec compute_length {
        pragma verify = false;
        pragma opaque;
        ensures result == spec_len(self);
    }

    spec iter_modify {
        pragma opaque;
        pragma verify = false;
        aborts_if iter_is_end(self, map);
        // iter_modify mutates the value at self.key via the closure. Containment is
        // unchanged for every key; values for keys other than self.key are preserved.
        ensures spec_contains_key(map, self.key);
        ensures spec_len(map) == spec_len(old(map));
        ensures spec_unchanged_except_at(map, self.key);
    }

    spec internal_find_with_path {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures iter_is_end(result.iterator, self) <==> !spec_contains_key(self, key);
        ensures !iter_is_end(result.iterator, self) ==> result.iterator.key == key;
    }

    spec iter_with_path_get_iter {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures result == self.iterator;
    }

    spec iter_remove {
        pragma opaque;
        pragma verify = false;
        aborts_if iter_is_end(self.iterator, map);
        ensures result == spec_get(old(map), self.iterator.key);
        ensures !spec_contains_key(map, self.iterator.key);
        ensures spec_len(map) == spec_len(old(map)) - 1;
        ensures spec_unchanged_except_at(map, self.iterator.key);
    }

    spec internal_leaf_new_begin_iter {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

    spec internal_leaf_iter_is_end {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

    spec internal_leaf_borrow_value {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures result == self.value;
    }

    spec internal_leaf_iter_borrow_entries_and_next_leaf_index {
        pragma opaque;
        pragma verify = false;
        aborts_if internal_leaf_iter_is_end(self);
    }
}
