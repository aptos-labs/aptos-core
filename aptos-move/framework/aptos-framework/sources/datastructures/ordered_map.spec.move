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
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains_key,
            map_spec_aborts_empty = spec_aborts_empty,
            map_spec_aborts_add_all = spec_aborts_add_all,
            map_spec_aborts_new_from = spec_aborts_new_from,
            map_spec_aborts_append_disjoint = spec_aborts_append_disjoint,
            map_spec_aborts_trim = spec_aborts_trim,
            map_spec_aborts_upsert_all = spec_aborts_upsert_all,
            map_spec_aborts_replace_key_inplace = spec_aborts_replace_key_inplace,
            map_is_empty = is_empty;
    }

    spec native fun spec_len<K, V>(t: OrderedMap<K, V>): num;
    spec native fun spec_contains_key<K, V>(t: OrderedMap<K, V>, k: K): bool;
    spec native fun spec_set<K, V>(t: OrderedMap<K, V>, k: K, v: V): OrderedMap<K, V>;
    spec native fun spec_remove<K, V>(t: OrderedMap<K, V>, k: K): OrderedMap<K, V>;
    spec native fun spec_get<K, V>(t: OrderedMap<K, V>, k: K): V;

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

    spec iter_add {
        pragma opaque;
        pragma verify = false;
    }


    spec iter_replace {
        pragma opaque;
        pragma verify = false;
    }

    spec iter_remove {
        pragma opaque;
        pragma verify = false;
    }

    spec iter_is_end {
        pragma opaque;
        pragma verify = false;
    }

    spec iter_borrow {
        pragma opaque;
        pragma verify = false;
    }

    spec iter_borrow_mut {
        pragma opaque;
        pragma verify = false;
    }

    spec iter_is_begin_from_non_empty {
        pragma opaque;
        pragma verify = false;
    }

    spec iter_is_begin {
        pragma opaque;
        pragma verify = false;
    }

    spec values {
        pragma intrinsic;
    }


    spec binary_search {
        pragma opaque;
        pragma verify = false;
    }


    spec internal_lower_bound {
        pragma opaque;
        pragma verify = false;
    }

    spec iter_borrow_key {
        pragma opaque;
        pragma verify = false;
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
    }

    spec internal_new_begin_iter {
        pragma opaque;
        pragma verify = false;
    }

    spec internal_new_end_iter {
        pragma opaque;
        pragma verify = false;
    }

    spec iter_next {
        pragma opaque;
        pragma verify = false;
    }

    spec iter_prev {
        pragma opaque;
        pragma verify = false;
    }

    spec get {
        pragma intrinsic;
    }
}
