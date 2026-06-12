spec aptos_framework::big_ordered_map {

    spec BigOrderedMap {
        pragma intrinsic = map,
            map_new = new,
            map_new_with_config = new_with_config,
            map_new_from = new_from,
            map_destroy_empty = destroy_empty,
            map_has_key = contains,
            map_remove_or_none = remove_or_none,
            map_borrow = borrow,
            map_borrow_mut = borrow_mut,
            map_get = get,
            map_keys = keys,
            map_front_key = front_key,
            map_back_key = back_key,
            map_borrow_front = borrow_front,
            map_borrow_back = borrow_back,
            map_pop_front = pop_front,
            map_pop_back = pop_back,
            map_prev_key = prev_key,
            map_next_key = next_key,
            map_iter_new_begin = internal_new_begin_iter,
            map_iter_new_end = internal_new_end_iter,
            map_iter_is_end = iter_is_end,
            map_iter_borrow_key = iter_borrow_key,
            map_internal_find = internal_find,
            map_internal_lower_bound = internal_lower_bound,
            map_internal_find_with_path = internal_find_with_path,
            map_iter_with_path_get_iter = iter_with_path_get_iter,
            map_len = compute_length,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains_key,
            map_is_empty = is_empty,
            map_spec_aborts_new_from = spec_aborts_new_from,
            map_spec_aborts_new_with_config = spec_aborts_new_with_config,
            map_spec_aborts_empty_map = spec_aborts_empty_map,
            map_spec_aborts_iter_borrow_key = spec_aborts_iter_borrow_key;
    }

    spec native fun spec_len<K, V>(t: BigOrderedMap<K, V>): num;
    spec native fun spec_contains_key<K, V>(t: BigOrderedMap<K, V>, k: K): bool;
    spec native fun spec_set<K, V>(t: BigOrderedMap<K, V>, k: K, v: V): BigOrderedMap<K, V>;
    spec native fun spec_remove<K, V>(t: BigOrderedMap<K, V>, k: K): BigOrderedMap<K, V>;
    spec native fun spec_get<K, V>(t: BigOrderedMap<K, V>, k: K): V;

    // Abort-condition spec functions paired with the corresponding intrinsics.
    spec native fun spec_aborts_new_from<K, V>(keys: vector<K>, values: vector<V>): bool;
    spec native fun spec_aborts_new_with_config<K, V>(inner_max_degree: u16, leaf_max_degree: u16, reuse_slots: bool): bool;
    spec native fun spec_aborts_empty_map<K, V>(t: BigOrderedMap<K, V>): bool;
    spec native fun spec_aborts_iter_borrow_key<K, V>(it: IteratorPtr<K>): bool;

    // Uninterpreted abort predicate for the `validate_dynamic_size_and_init_max_degrees`
    // check called by both `add` and `upsert`. True iff the call would abort under the
    // dynamic key/value size validation. Body-less because it depends on hidden map
    // configuration fields.
    spec fun spec_aborts_validate_size<K, V>(t: BigOrderedMap<K, V>, k: K, v: V): bool;

    // Uninterpreted abort predicate for stale-iterator operations. The runtime aborts on
    // any iter use whose cached `node_index`/`child_iter` no longer matches the current
    // tree state (rebalance, sibling removal, etc.). Body-less because tracking node_index
    // and child_iter precisely would require modeling BOM's B+-tree state.
    spec fun spec_iter_op_aborts<K, V>(it: IteratorPtr<K>, m: BigOrderedMap<K, V>): bool;



    // Uninterpreted abort predicate for `new_with_reusable`: true iff K or V has
    // non-constant serialized size. Body-less because it depends on
    // `bcs::constant_serialized_size`, not expressible in the intrinsic-typed spec layer.
    spec fun spec_aborts_new_with_reusable_runtime<K, V>(): bool;

    // Uninterpreted abort predicate for `new_with_type_size_hints`: true iff any of the
    // size-hint validations fail (avg > max, division-by-zero on `avg_*_bytes` or
    // `max_*_bytes`, or derived max-degrees below the minimum).
    spec fun spec_aborts_new_with_type_size_hints_runtime<K, V>(
        avg_key_bytes: u64, max_key_bytes: u64,
        avg_value_bytes: u64, max_value_bytes: u64
    ): bool;


    spec new_with_config {
        pragma intrinsic;
    }

    spec new {
        pragma intrinsic;
    }

    spec new_with_reusable {
        pragma opaque;
        pragma verify = false;
        aborts_if [abstract] spec_aborts_new_with_reusable_runtime<K, V>();
        ensures !spec_aborts_new_with_reusable_runtime<K, V>() ==> spec_len(result) == 0;
        ensures !spec_aborts_new_with_reusable_runtime<K, V>() ==>
            (forall k: K: !spec_contains_key(result, k));
    }

    spec new_with_type_size_hints {
        pragma opaque;
        pragma verify = false;
        aborts_if [abstract] spec_aborts_new_with_type_size_hints_runtime<K, V>(
            avg_key_bytes, max_key_bytes, avg_value_bytes, max_value_bytes);
        ensures !spec_aborts_new_with_type_size_hints_runtime<K, V>(
            avg_key_bytes, max_key_bytes, avg_value_bytes, max_value_bytes)
            ==> spec_len(result) == 0;
        ensures !spec_aborts_new_with_type_size_hints_runtime<K, V>(
            avg_key_bytes, max_key_bytes, avg_value_bytes, max_value_bytes)
            ==> (forall k: K: !spec_contains_key(result, k));
    }

    spec borrow {
        pragma intrinsic;
    }

    spec borrow_mut {
        pragma intrinsic;
    }

    spec get {
        pragma intrinsic;
    }

    spec contains {
        pragma intrinsic;
    }

    spec destroy_empty {
        pragma intrinsic;
    }

    spec add {
        pragma opaque;
        pragma verify = false;
        aborts_if spec_contains_key(self, key);
        aborts_if [abstract] spec_aborts_validate_size(self, key, value);
        ensures spec_contains_key(self, key);
        ensures spec_get(self, key) == value;
        ensures spec_len(self) == spec_len(old(self)) + 1;
        ensures spec_unchanged_except_at(self, key);
    }

    spec remove {
        pragma opaque;
        pragma verify = false;
        aborts_if !spec_contains_key(self, key);
        ensures !spec_contains_key(self, key);
        ensures spec_get(old(self), key) == result;
        ensures spec_len(old(self)) == spec_len(self) + 1;
        ensures spec_unchanged_except_at(self, key);
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
        pragma intrinsic;
    }

    spec iter_borrow {
        pragma opaque;
        pragma verify = false;
        aborts_if iter_is_end(self, map);
        aborts_if [abstract] spec_iter_op_aborts(self, map);
        // Result is havoc'd: when the iter is stale (cached position no longer matches
        // tree state) the runtime aborts; the spec does not claim a specific value here.
    }

    // Body also asserts constant_kv_size OR bcs::constant_serialized_size<V>().is_some()
    // which is not expressible from spec context. Caller-side, iter_is_end is what's discharged.
    spec iter_borrow_mut {
        pragma opaque;
        pragma verify = false;
        aborts_if iter_is_end(self, map);
        aborts_if [abstract] spec_iter_op_aborts(self, map);
    }

    // `iter_is_begin` is not bound as intrinsic because the runtime checks
    // `node_index == min_leaf_index && child_iter_at_begin`, which the model cannot
    // track without representing BOM's tree state. We only pin the End-on-empty case;
    // for `Some` iters the return is havoc'd (sound but imprecise on stale iters).
    spec iter_is_begin {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures (self is IteratorPtr::End<K>) ==> (result == (spec_len(map) == 0));
    }

    spec internal_lower_bound {
        pragma intrinsic;
    }

    spec iter_borrow_key {
        pragma intrinsic;
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

    spec new_from {
        pragma intrinsic;
    }

    spec upsert {
        pragma opaque;
        pragma verify = false;
        aborts_if [abstract] spec_aborts_validate_size(self, key, value);
        ensures !spec_contains_key(old(self), key) ==> option::is_none(result);
        ensures spec_contains_key(old(self), key) ==>
            (option::is_some(result) && option::spec_borrow(result) == spec_get(old(self), key));
        ensures spec_contains_key(self, key);
        ensures spec_get(self, key) == value;
        ensures !spec_contains_key(old(self), key) ==> spec_len(old(self)) + 1 == spec_len(self);
        ensures spec_contains_key(old(self), key) ==> spec_len(old(self)) == spec_len(self);
        ensures spec_unchanged_except_at(self, key);
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
        pragma intrinsic;
    }

    spec internal_new_begin_iter {
        pragma intrinsic;
    }

    spec internal_new_end_iter {
        pragma intrinsic;
    }

    spec iter_next {
        pragma opaque;
        pragma verify = false;
        aborts_if iter_is_end(self, map);
        aborts_if [abstract] spec_iter_op_aborts(self, map);
    }

    spec iter_prev {
        pragma opaque;
        pragma verify = false;
        aborts_if iter_is_begin(self, map);
        aborts_if [abstract] spec_iter_op_aborts(self, map);
    }

    spec compute_length {
        pragma intrinsic;
    }

    spec iter_modify {
        pragma opaque;
        pragma verify = false;
        aborts_if iter_is_end(self, map);
        aborts_if [abstract] spec_iter_op_aborts(self, map);
        // Closure `f: |&mut V|` can abort at runtime; the model doesn't capture that
        // condition precisely. The `[abstract]` markers above make the spec partial,
        // so callers cannot conclude the function returns on any specific input.
        // The success-path ensure is sound: closure mutates in place, neither adds
        // nor removes entries. We don't claim which key was mutated.
        ensures spec_len(map) == spec_len(old(map));
    }

    spec internal_find_with_path {
        pragma intrinsic;
    }

    spec iter_with_path_get_iter {
        pragma intrinsic;
    }

    spec iter_remove {
        pragma opaque;
        pragma verify = false;
        aborts_if iter_is_end(self.iterator, map);
        aborts_if [abstract] spec_iter_op_aborts(self.iterator, map);
        // Removal of one entry; we do not claim which key was removed (cached position
        // may be stale).
        ensures spec_len(map) == spec_len(old(map)) - 1;
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
