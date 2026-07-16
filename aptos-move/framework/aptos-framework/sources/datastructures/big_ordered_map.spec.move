spec aptos_framework::big_ordered_map {

    // The ordering bindings below (`map_borrow_front`/`back`, `map_pop_front`/`back`,
    // `map_prev_key`/`next_key`) presume `cmp::compare<K>` is a strict total order on K.
    // Built-in K types satisfy this; user-defined K types must too for this spec block
    // to be sound.
    //
    // Size presumption: BigOrderedMap validates K/V serialized sizes against node-size
    // limits (`validate_static_size_and_init_max_degrees` and per-insert checks) and
    // aborts when exceeded. These size-based aborts — including `borrow_mut`'s
    // constant-value-size requirement — are presumed not to fire and are not
    // modeled by the bindings below.
    spec BigOrderedMap {
        pragma intrinsic = map,
            map_new = new,
            map_new_with_config = new_with_config,
            map_len = compute_length,
            map_destroy_empty = destroy_empty,
            map_has_key = contains,
            map_add_no_override = add,
            map_upsert = upsert,
            map_del_must_exist = remove,
            map_remove_or_none = remove_or_none,
            map_get = get,
            map_borrow_front = borrow_front,
            map_borrow_back = borrow_back,
            map_front_key = front_key,
            map_back_key = back_key,
            map_pop_front = pop_front,
            map_pop_back = pop_back,
            map_prev_key = prev_key,
            map_next_key = next_key,
            map_keys = keys,
            map_to_ordered_map = to_ordered_map,
            map_new_from = new_from,
            map_add_all = add_all,
            map_borrow = borrow,
            map_borrow_mut = borrow_mut,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains_key,
            map_spec_aborts_empty = spec_aborts_empty,
            map_spec_aborts_add_all = spec_aborts_add_all,
            map_spec_aborts_new_from = spec_aborts_new_from,
            map_spec_aborts_new_with_config = spec_aborts_new_with_config,
            map_spec_aborts_destroy_empty = spec_aborts_destroy_empty,
            map_spec_aborts_add = spec_aborts_add,
            map_spec_aborts_del = spec_aborts_del,
            map_spec_aborts_borrow = spec_aborts_borrow,
            map_is_empty = is_empty;
    }

    spec native fun spec_len<K, V>(t: BigOrderedMap<K, V>): num;
    spec native fun spec_contains_key<K, V>(t: BigOrderedMap<K, V>, k: K): bool;
    spec native fun spec_set<K, V>(t: BigOrderedMap<K, V>, k: K, v: V): BigOrderedMap<K, V>;
    spec native fun spec_remove<K, V>(t: BigOrderedMap<K, V>, k: K): BigOrderedMap<K, V>;
    spec native fun spec_get<K, V>(t: BigOrderedMap<K, V>, k: K): V;
    spec native fun spec_aborts_destroy_empty<K, V>(t: BigOrderedMap<K, V>): bool;
    spec native fun spec_aborts_add<K, V>(t: BigOrderedMap<K, V>, k: K, v: V): bool;
    spec native fun spec_aborts_del<K, V>(t: BigOrderedMap<K, V>, k: K): bool;
    spec native fun spec_aborts_borrow<K, V>(t: BigOrderedMap<K, V>, k: K): bool;

    spec fun spec_aborts_empty<K, V>(t: BigOrderedMap<K, V>): bool {
        spec_len(t) == 0
    }

    spec fun spec_aborts_add_all<K, V>(m: BigOrderedMap<K, V>, keys: vector<K>, values: vector<V>): bool {
        len(keys) != len(values)
            || (exists i in 0..len(keys): spec_contains_key(m, keys[i]))
            || (exists i in 0..len(keys), j in 0..len(keys) where i != j: keys[i] == keys[j])
    }

    spec fun spec_aborts_new_from<K, V>(keys: vector<K>, values: vector<V>): bool {
        len(keys) != len(values)
            || (exists i in 0..len(keys), j in 0..len(keys) where i != j: keys[i] == keys[j])
    }

    spec fun spec_aborts_new_with_config<K, V>(
        inner_max_degree: u16, leaf_max_degree: u16, _reuse_slots: bool
    ): bool {
        (inner_max_degree != 0
            && (inner_max_degree < 4 || (inner_max_degree as u64) > 4096))
        || (leaf_max_degree != 0
            && (leaf_max_degree < 3 || (leaf_max_degree as u64) > 4096))
    }


    spec new_with_config {
        pragma intrinsic;
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
        pragma intrinsic;
    }

    spec get {
        pragma intrinsic;
    }

    spec fun spec_unchanged_except_at<K: drop + copy + store, V: store>(
        self: &mut BigOrderedMap<K, V>, key: &K
    ): bool {
        (forall k: K where k != key:
            spec_contains_key(self, k) == spec_contains_key(old(self), k))
        && (forall k: K where k != key && spec_contains_key(old(self), k):
            spec_get(self, k) == spec_get(old(self), k))
    }

    spec remove_or_none {
        pragma intrinsic;
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

    // Spec-level mirror of `iter_is_begin`. The Move body reads intrinsic map
    // internals, so the function itself cannot appear in spec expressions.
    // self is End: begin iff map is empty (End acts as both begin and end on []).
    // self is Some: begin iff self.key is the smallest key currently in map.
    spec fun spec_iter_is_begin<K, V>(self: IteratorPtr<K>, map: BigOrderedMap<K, V>): bool {
        if (self is IteratorPtr::End<K>) {
            spec_len(map) == 0
        } else {
            spec_contains_key(map, self.key)
                && (forall k: K where spec_contains_key(map, k) && k != self.key:
                    std::cmp::compare(self.key, k) == std::cmp::Ordering::Less)
        }
    }

    spec iter_is_begin {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures result <==> spec_iter_is_begin(self, map);
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
        pragma intrinsic;
    }

    spec to_ordered_map {
        pragma intrinsic;
    }

    spec new_from {
        pragma intrinsic;
    }

    spec upsert {
        pragma intrinsic;
    }

    spec add_all {
        pragma intrinsic;
    }

    spec borrow_front {
        pragma intrinsic;
    }

    spec front_key {
        pragma intrinsic;
    }

    spec borrow_back {
        pragma intrinsic;
    }

    spec back_key {
        pragma intrinsic;
    }

    spec pop_front {
        pragma intrinsic;
    }

    spec pop_back {
        pragma intrinsic;
    }

    spec prev_key {
        pragma intrinsic;
    }

    spec next_key {
        pragma intrinsic;
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
        aborts_if spec_iter_is_begin(self, map);
    }

    spec compute_length {
        pragma intrinsic;
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
