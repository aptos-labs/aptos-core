/// Specifications of the `table` module.
spec velor_std::table {

    // Make most of the public API intrinsic. Those functions have custom specifications in the prover.

    spec Table {
        pragma intrinsic = map,
            map_new = new,
            map_destroy_empty = destroy_known_empty_unsafe,
            map_has_key = contains,
            map_add_no_override = add,
            map_add_override_if_exists = upsert,
            map_del_must_exist = remove,
            map_borrow = borrow,
            map_borrow_mut = borrow_mut,
            map_borrow_mut_with_default = borrow_mut_with_default,
            map_borrow_with_default = borrow_with_default,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove,
            map_spec_has_key = spec_contains;
    }

    spec new {
        pragma intrinsic;
    }

    spec destroy_known_empty_unsafe {
        pragma intrinsic;
    }

    spec add {
        pragma intrinsic;
    }

    spec borrow {
        pragma intrinsic;
    }

    spec borrow_mut {
        pragma intrinsic;
    }

    spec borrow_mut_with_default {
        pragma intrinsic;
    }

    spec borrow_with_default {
        pragma intrinsic;
    }

    spec upsert {
        pragma intrinsic;
    }

    spec remove {
        pragma intrinsic;
    }

    spec contains {
        pragma intrinsic;
    }

    // Specification functions for tables
    spec native fun spec_contains<K, V>(t: Table<K, V>, k: K): bool;
    spec native fun spec_remove<K, V>(t: Table<K, V>, k: K): Table<K, V>;
    spec native fun spec_set<K, V>(t: Table<K, V>, k: K, v: V): Table<K, V>;
    spec native fun spec_get<K, V>(t: Table<K, V>, k: K): V;
}
