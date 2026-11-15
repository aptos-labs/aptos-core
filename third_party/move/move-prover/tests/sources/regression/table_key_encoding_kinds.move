// Table keys are encoded to integers. A key type whose Boogie equality is Move
// equality gets an inverse-function encoding; any other keeps the pairwise one.
// Each kind must keep distinct keys apart and equal keys together.
module 0x42::table_key_encoding_kinds {
    use extensions::table::{Self, Table};
    use extensions::table::{spec_get, spec_contains};

    struct Key has copy, drop, store {
        a: address,
        n: u64
    }

    fun struct_keys(k1: Key, k2: Key): Table<Key, u64> {
        let t = table::new<Key, u64>();
        table::add(&mut t, k1, 1);
        if (k1 != k2) {
            table::add(&mut t, k2, 2);
        };
        t
    }
    spec struct_keys {
        ensures spec_contains(result, k1) && spec_contains(result, k2);
        ensures k1 != k2 ==> spec_get(result, k1) == 1 && spec_get(result, k2) == 2;
        ensures k1 == k2 ==> spec_get(result, k2) == 1;
    }

    fun struct_keys_fail(k1: Key, k2: Key): Table<Key, u64> {
        let t = table::new<Key, u64>();
        table::add(&mut t, k1, 1);
        if (k1 != k2) {
            table::add(&mut t, k2, 2);
        };
        t
    }
    spec struct_keys_fail {
        ensures spec_get(result, k1) == spec_get(result, k2); // fails when k1 != k2
    }

    fun vector_keys(k1: vector<u8>, k2: vector<u8>): Table<vector<u8>, u64> {
        let t = table::new<vector<u8>, u64>();
        table::add(&mut t, k1, 1);
        if (k1 != k2) {
            table::add(&mut t, k2, 2);
        };
        t
    }
    spec vector_keys {
        ensures spec_contains(result, k1) && spec_contains(result, k2);
        ensures k1 != k2 ==> spec_get(result, k1) == 1 && spec_get(result, k2) == 2;
        ensures k1 == k2 ==> spec_get(result, k2) == 1;
    }

    fun vector_keys_fail(k1: vector<u8>, k2: vector<u8>): Table<vector<u8>, u64> {
        let t = table::new<vector<u8>, u64>();
        table::add(&mut t, k1, 1);
        if (k1 != k2) {
            table::add(&mut t, k2, 2);
        };
        t
    }
    spec vector_keys_fail {
        ensures spec_get(result, k1) == spec_get(result, k2); // fails when k1 != k2
    }
}

// An intrinsic map has native equality by its (absent) Move fields, but its
// representation keeps values at removed keys, so raw equality is finer than
// Move equality. As a table key it must keep the pairwise encoding.
module 0x42::table_key_encoding_map_keys {
    struct Map<phantom K: copy + drop, phantom V> has store, drop, copy {}
    spec Map {
        pragma intrinsic = map,
            map_new = new, map_has_key = contains, map_add_no_override = add,
            map_del_must_exist = remove, map_spec_get = spec_get, map_spec_set = spec_set,
            map_spec_del = spec_remove, map_spec_len = spec_len, map_spec_has_key = spec_contains;
    }
    public native fun new<K: copy + drop, V: store>(): Map<K, V>;
    public native fun contains<K: copy + drop, V>(m: &Map<K, V>, key: K): bool;
    public native fun add<K: copy + drop, V>(m: &mut Map<K, V>, key: K, val: V);
    public native fun remove<K: copy + drop, V>(m: &mut Map<K, V>, key: K): V;
    spec native fun spec_len<K, V>(m: Map<K, V>): num;
    spec native fun spec_contains<K, V>(m: Map<K, V>, k: K): bool;
    spec native fun spec_get<K, V>(m: Map<K, V>, k: K): V;
    spec native fun spec_set<K, V>(m: Map<K, V>, k: K, v: V): Map<K, V>;
    spec native fun spec_remove<K, V>(m: Map<K, V>, k: K): Map<K, V>;

    // Two maps emptied after holding different values are equal, so the outer
    // map keyed by the first contains the second.
    fun emptied_maps_as_keys_fail(): bool {
        let m1 = new<u64, u64>();
        add(&mut m1, 1, 10);
        remove(&mut m1, 1);
        let m2 = new<u64, u64>();
        add(&mut m2, 1, 20);
        remove(&mut m2, 1);
        let outer = new<Map<u64, u64>, u64>();
        add(&mut outer, m1, 5);
        contains(&outer, m2)
    }
    spec emptied_maps_as_keys_fail {
        ensures !result; // fails: the maps are equal
    }
}
