spec aptos_framework::ordered_map {

    // The ordering bindings below (`map_borrow_front`/`back`, `map_pop_front`/`back`,
    // `map_prev_key`/`next_key`) presume `cmp::compare<K>` is a strict total order on K.
    // Built-in K types satisfy this; user-defined K types must too for this spec block
    // to be sound.
    spec OrderedMap {
        pragma intrinsic = map,
            map_new = new,
            map_len = length,
            map_destroy_empty = destroy_empty,
            map_has_key = contains,
            map_add_no_override = add,
            map_upsert = upsert,
            map_del_must_exist = remove,
            map_remove_or_none = remove_or_none,
            map_get = get,
            map_borrow_front = borrow_front,
            map_borrow_back = borrow_back,
            map_pop_front = pop_front,
            map_pop_back = pop_back,
            map_prev_key = prev_key,
            map_next_key = next_key,
            map_keys = keys,
            map_values = values,
            map_to_vec_pair = to_vec_pair,
            map_new_from = new_from,
            map_add_all = add_all,
            map_upsert_all = upsert_all,
            map_append = append,
            map_append_disjoint = append_disjoint,
            map_trim = trim,
            map_replace_key_inplace = replace_key_inplace,
            map_borrow = borrow,
            map_borrow_mut = borrow_mut,
            map_iter_borrow_mut = iter_borrow_mut,
            map_spec_aborts_iter_borrow_mut = spec_aborts_iter_borrow_mut,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains_key,
            map_spec_key_at = spec_key_at,
            map_spec_rank = spec_rank,
            map_spec_aborts_empty = spec_aborts_empty,
            map_spec_aborts_add_all = spec_aborts_add_all,
            map_spec_aborts_new_from = spec_aborts_new_from,
            map_spec_aborts_append_disjoint = spec_aborts_append_disjoint,
            map_spec_aborts_trim = spec_aborts_trim,
            map_spec_aborts_upsert_all = spec_aborts_upsert_all,
            map_spec_aborts_replace_key_inplace = spec_aborts_replace_key_inplace,
            map_spec_aborts_destroy_empty = spec_aborts_destroy_empty,
            map_spec_aborts_add = spec_aborts_add,
            map_spec_aborts_del = spec_aborts_del,
            map_spec_aborts_borrow = spec_aborts_borrow,
            map_is_empty = is_empty;
    }

    spec native fun spec_len<K, V>(t: OrderedMap<K, V>): num;
    spec native fun spec_contains_key<K, V>(t: OrderedMap<K, V>, k: K): bool;
    // Enumeration view (mirrors big_ordered_map): spec_key_at(t, i) is the
    // i-th smallest key, spec_rank(t, k) its inverse on contained keys.
    spec native fun spec_key_at<K, V>(t: OrderedMap<K, V>, i: num): K;

    /// Witness-test support: sum of the values at the first `n` keys.
    spec fun spec_om_sum_upto(m: OrderedMap<u64, u64>, n: num): num {
        if (n <= 0) { 0 } else { spec_om_sum_upto(m, n - 1) + spec_get(m, spec_key_at(m, n - 1)) }
    }
    spec native fun spec_rank<K, V>(t: OrderedMap<K, V>, k: K): num;
    spec native fun spec_set<K, V>(t: OrderedMap<K, V>, k: K, v: V): OrderedMap<K, V>;
    spec native fun spec_remove<K, V>(t: OrderedMap<K, V>, k: K): OrderedMap<K, V>;
    spec native fun spec_get<K, V>(t: OrderedMap<K, V>, k: K): V;
    spec native fun spec_aborts_destroy_empty<K, V>(t: OrderedMap<K, V>): bool;
    spec native fun spec_aborts_add<K, V>(t: OrderedMap<K, V>, k: K, v: V): bool;
    spec native fun spec_aborts_del<K, V>(t: OrderedMap<K, V>, k: K): bool;
    spec native fun spec_aborts_borrow<K, V>(t: OrderedMap<K, V>, k: K): bool;

    spec fun spec_aborts_empty<K, V>(t: OrderedMap<K, V>): bool {
        spec_len(t) == 0
    }

    spec fun spec_aborts_add_all<K, V>(m: OrderedMap<K, V>, keys: vector<K>, values: vector<V>): bool {
        len(keys) != len(values)
            || (exists i in 0..len(keys): spec_contains_key(m, keys[i]))
            || (exists i in 0..len(keys), j in 0..len(keys) where i != j: keys[i] == keys[j])
    }

    spec fun spec_aborts_new_from<K, V>(keys: vector<K>, values: vector<V>): bool {
        len(keys) != len(values)
            || (exists i in 0..len(keys), j in 0..len(keys) where i != j: keys[i] == keys[j])
    }

    spec fun spec_aborts_append_disjoint<K, V>(m: OrderedMap<K, V>, other: OrderedMap<K, V>): bool {
        exists k: K: spec_contains_key(m, k) && spec_contains_key(other, k)
    }

    spec fun spec_aborts_trim<K, V>(m: OrderedMap<K, V>, at: u64): bool {
        at > spec_len(m)
    }

    spec fun spec_aborts_upsert_all<K, V>(_m: OrderedMap<K, V>, keys: vector<K>, values: vector<V>): bool {
        len(keys) != len(values)
    }

    // Over-approximates the template's cmp-order-violation abort path (modeled
    // nondeterministically): when `old_key != new_key`, returns true even though
    // the actual call may succeed if the order precondition holds.
    spec fun spec_aborts_replace_key_inplace<K, V>(m: OrderedMap<K, V>, old_key: K, new_key: K): bool {
        !spec_contains_key(m, old_key) || old_key != new_key
    }

    spec length {
        pragma intrinsic;
    }

    spec new {
        pragma intrinsic;
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

    spec remove_or_none {
        pragma intrinsic;
    }

    spec is_empty {
        pragma intrinsic;
    }

    // Where an add lands: the end iterator appends, any other iterator inserts
    // at its own position.
    spec fun spec_iter_add_index<K, V>(self: IteratorPtr, map: OrderedMap<K, V>): num {
        if (self is IteratorPtr::End) spec_len(map) else self.index
    }

    // The insert splices a position in. The two ordering checks in the body say
    // exactly that the position is the sorted one for this key — larger than the
    // key before it and smaller than the key at it — so a duplicate key always
    // aborts.
    spec iter_add {
        pragma opaque;
        pragma verify = false;
        aborts_if !(self is IteratorPtr::End) && self.index > spec_len(map);
        aborts_if spec_iter_add_index(self, map) > 0
            && cmp::compare(spec_key_at(map, spec_iter_add_index(self, map) - 1), key)
                != cmp::Ordering::Less;
        aborts_if spec_iter_add_index(self, map) < spec_len(map)
            && cmp::compare(key, spec_key_at(map, spec_iter_add_index(self, map)))
                != cmp::Ordering::Less;
        ensures spec_len(map) == spec_len(old(map)) + 1;
        ensures spec_contains_key(map, key) && spec_get(map, key) == value;
        ensures spec_rank(map, key) == spec_iter_add_index(self, old(map));
        ensures forall i in 0..spec_iter_add_index(self, old(map)):
            spec_key_at(map, i) == spec_key_at(old(map), i);
        ensures forall i in (spec_iter_add_index(self, old(map)) + 1)..spec_len(map):
            spec_key_at(map, i) == spec_key_at(old(map), i - 1);
        ensures forall k: K where k != key && old(spec_contains_key(map, k)):
            spec_contains_key(map, k) && spec_get(map, k) == old(spec_get(map, k));
    }

    // A value write at the iterator's position: the key set, and so every
    // position, is untouched.
    spec iter_replace {
        pragma opaque;
        pragma verify = false;
        aborts_if (self is IteratorPtr::End) || self.index >= spec_len(map);
        ensures result == old(spec_get(map, spec_key_at(map, self.index)));
        ensures spec_get(map, old(spec_key_at(map, self.index))) == value;
        ensures spec_len(map) == spec_len(old(map));
        ensures forall i in 0..spec_len(map): spec_key_at(map, i) == spec_key_at(old(map), i);
        ensures forall k: K: spec_contains_key(map, k) == old(spec_contains_key(map, k));
        ensures forall k: K where k != old(spec_key_at(map, self.index))
            && old(spec_contains_key(map, k)):
            spec_get(map, k) == old(spec_get(map, k));
    }

    // The mirror of the add: removing at a position closes it up, so keys after
    // it move down one and keys before it stay put.
    spec iter_remove {
        pragma opaque;
        pragma verify = false;
        aborts_if (self is IteratorPtr::End) || self.index >= spec_len(map);
        ensures result == old(spec_get(map, spec_key_at(map, self.index)));
        ensures spec_len(map) == spec_len(old(map)) - 1;
        ensures !spec_contains_key(map, old(spec_key_at(map, self.index)));
        ensures forall i in 0..self.index: spec_key_at(map, i) == spec_key_at(old(map), i);
        ensures forall i in self.index..spec_len(map):
            spec_key_at(map, i) == spec_key_at(old(map), i + 1);
        ensures forall k: K where k != old(spec_key_at(map, self.index))
            && old(spec_contains_key(map, k)):
            spec_contains_key(map, k) && spec_get(map, k) == old(spec_get(map, k));
    }

    spec iter_is_end {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures result <==> (self is IteratorPtr::End);
    }

    spec iter_borrow {
        pragma opaque;
        pragma verify = false;
        aborts_if (self is IteratorPtr::End) || self.index >= spec_len(map);
        ensures result == spec_get(map, spec_key_at(map, self.index));
    }

    // Modelled by the intrinsic map: the borrow resolves this iterator's
    // position to a key through the enumeration and hands back a mutation at
    // that key, so a caller's write-back updates the abstract map instead of
    // traversing the entry vector.
    spec iter_borrow_mut {
        pragma intrinsic;
    }

    // Mirrors the borrow's abort behavior: no value at the end iterator, and
    // none at a position past the last one.
    spec fun spec_aborts_iter_borrow_mut<K, V>(
        self: IteratorPtr, map: OrderedMap<K, V>
    ): bool {
        (self is IteratorPtr::End) || self.index >= spec_len(map)
    }

    spec iter_is_begin_from_non_empty {
        pragma opaque;
        pragma verify = false;
    }

    spec iter_is_begin {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures result <==> (if (self is IteratorPtr::End) spec_len(map) == 0 else self.index == 0);
    }

    spec values {
        pragma intrinsic;
    }


    spec binary_search {
        pragma opaque;
        pragma verify = false;
    }


    // The first position whose key is not less than the input — the end iterator
    // when every key is smaller. Since positions are the enumeration, that index
    // is where the input would sit, which is what lets a scan start here knowing
    // it skipped only smaller keys.
    spec internal_lower_bound {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures (result is IteratorPtr::End) <==>
            (forall i in 0..spec_len(self):
                cmp::compare(spec_key_at(self, i), key) == cmp::Ordering::Less);
        ensures !(result is IteratorPtr::End) ==> result.index < spec_len(self);
        ensures !(result is IteratorPtr::End) ==>
            cmp::compare(spec_key_at(self, result.index), key) != cmp::Ordering::Less;
        ensures !(result is IteratorPtr::End) ==>
            (forall i in 0..result.index:
                cmp::compare(spec_key_at(self, i), key) == cmp::Ordering::Less);
        // A key that is present is never skipped past. Implied by the End
        // characterization above, but only after instantiating it at that key's
        // own position, which a caller has no term to do with. Its position
        // being the key's rank then follows from the comparison facts, so this
        // is all that needs stating.
        ensures spec_contains_key(self, key) ==> !(result is IteratorPtr::End);
    }

    spec iter_borrow_key {
        pragma opaque;
        pragma verify = false;
        aborts_if (self is IteratorPtr::End) || self.index >= spec_len(map);
        // The index is the position in ascending key order.
        ensures result == spec_key_at(map, self.index);
    }

    spec keys {
        pragma intrinsic;
    }

    spec to_vec_pair {
        pragma intrinsic;
    }

    spec new_from {
        pragma intrinsic;
    }

    spec upsert {
        pragma intrinsic;
    }

    spec replace_key_inplace {
        pragma intrinsic;
    }

    spec add_all {
        pragma intrinsic;
    }

    spec append {
        pragma intrinsic;
    }

    spec upsert_all {
        pragma intrinsic;
    }

    spec append_disjoint {
        pragma intrinsic;
    }

    spec append_impl {
        pragma opaque;
        pragma verify = false;
    }

    spec trim {
        pragma intrinsic;
    }

    spec borrow_front {
        pragma intrinsic;
    }

    spec borrow_back {
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
        ensures (result is IteratorPtr::End) <==> !spec_contains_key(self, key);
        ensures !(result is IteratorPtr::End) ==> result.index == spec_rank(self, key);
    }

    spec internal_new_begin_iter {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures (result is IteratorPtr::End) <==> spec_len(self) == 0;
        ensures !(result is IteratorPtr::End) ==> result.index == 0;
    }

    spec internal_new_end_iter {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures result is IteratorPtr::End;
    }

    spec iter_next {
        pragma opaque;
        pragma verify = false;
        aborts_if self is IteratorPtr::End;
        // One step forward, becoming End once past the last position.
        ensures (result is IteratorPtr::End) <==> self.index + 1 >= spec_len(map);
        ensures !(result is IteratorPtr::End) ==> result.index == self.index + 1;
    }

    spec iter_prev {
        pragma opaque;
        pragma verify = false;
        aborts_if if (self is IteratorPtr::End) spec_len(map) == 0 else self.index == 0;
        ensures !(result is IteratorPtr::End);
        // From End, one step back is the last position; otherwise one less.
        ensures (self is IteratorPtr::End) ==> result.index == spec_len(map) - 1;
        ensures !(self is IteratorPtr::End) ==> result.index == self.index - 1;
    }

    spec get {
        pragma intrinsic;
    }
}
