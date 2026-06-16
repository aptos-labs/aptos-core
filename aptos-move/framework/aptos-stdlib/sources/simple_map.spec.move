/// Specifications of the `simple_map` module.
spec aptos_std::simple_map {

    // Make most of the public API intrinsic. Those functions have custom specifications in the prover.

    spec SimpleMap {
        pragma intrinsic = map,
            map_new = new,
            map_new_from = new_from,
            map_to_vec_pair = to_vec_pair,
            map_keys = keys,
            map_values = values,
            map_upsert_kv = upsert,
            map_len = length,
            map_destroy_empty = destroy_empty,
            map_has_key = contains_key,
            map_add_no_override = add,
            map_del_return_key = remove,
            map_borrow = borrow,
            map_borrow_mut = borrow_mut,
            map_spec_new = spec_new,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains_key,
            map_spec_aborts_destroy_empty = spec_aborts_destroy_empty,
            map_spec_aborts_add = spec_aborts_add,
            map_spec_aborts_del = spec_aborts_del,
            map_spec_aborts_borrow = spec_aborts_borrow,
            map_spec_aborts_new_from = spec_aborts_new_from;
    }

    spec length {
        pragma intrinsic;
    }

    spec borrow {
        pragma intrinsic;
    }

    spec borrow_mut {
        pragma intrinsic;
    }

    spec contains_key {
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

    spec find {
        pragma verify=false;
    }

    spec keys {
        pragma intrinsic;
    }

    spec values {
        pragma intrinsic;
    }

    spec new {
        pragma intrinsic;
    }

    spec new_from {
        pragma intrinsic;
    }

    spec to_vec_pair {
        pragma intrinsic;
    }

    spec upsert {
        pragma intrinsic;
    }

    // Specification functions for tables
    spec native fun spec_new<K, V>(): SimpleMap<K, V>;
    spec native fun spec_len<K, V>(t: SimpleMap<K, V>): num;
    spec native fun spec_contains_key<K, V>(t: SimpleMap<K, V>, k: K): bool;
    spec native fun spec_set<K, V>(t: SimpleMap<K, V>, k: K, v: V): SimpleMap<K, V>;
    spec native fun spec_remove<K, V>(t: SimpleMap<K, V>, k: K): SimpleMap<K, V>;
    spec native fun spec_get<K, V>(t: SimpleMap<K, V>, k: K): V;

    // Abort-condition spec functions — mirror the abort guards in the Boogie template
    spec native fun spec_aborts_destroy_empty<K, V>(m: SimpleMap<K, V>): bool;
    spec native fun spec_aborts_add<K, V>(m: SimpleMap<K, V>, k: K, v: V): bool;
    spec native fun spec_aborts_del<K, V>(m: SimpleMap<K, V>, k: K): bool;
    spec native fun spec_aborts_borrow<K, V>(m: SimpleMap<K, V>, k: K): bool;
    spec native fun spec_aborts_new_from<K, V>(keys: vector<K>, values: vector<V>): bool;
}
