spec aptos_framework::ordered_map {

    // The ordering bindings below (`map_borrow_front`/`back`, `map_pop_front`/`back`,
    // `map_prev_key`/`next_key`) model OrderedMap's behavior in terms of `cmp::compare<K>`.
    // They presume `cmp::compare<K>` is a strict total order on the inhabited K values:
    // antisymmetric, transitive, and total (every two distinct keys are comparable as
    // strictly Less or Greater). Built-in K types (integers, bool, address, vector<u8>,
    // string) satisfy this. User-defined K types used as OrderedMap keys must too —
    // verification of an OrderedMap user against this spec block is sound only when
    // the K type's `cmp::compare` is a strict total order.
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
