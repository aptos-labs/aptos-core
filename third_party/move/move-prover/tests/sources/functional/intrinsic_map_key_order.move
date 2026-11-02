// use-aptos-stdlib
// flag: --aptos
// A key-ordered intrinsic map whose key type is never compared in bytecode.
// The map orders its keys by `cmp::compare`, so the enumeration view is
// ascending under it whether or not the program itself calls `compare`.
// Inserting a key then bounds the largest key from below by both the new key
// and the previous largest key.
module 0x42::intrinsic_map_key_order {
    struct Map<phantom K: copy + drop, phantom V> has copy, store, drop {}

    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_has_key = contains,
            map_add_no_override = add,
            map_del_must_exist = remove,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains,
            map_spec_key_at = spec_key_at,
            map_spec_rank = spec_rank;
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
    spec native fun spec_key_at<K, V>(m: Map<K, V>, i: num): K;
    spec native fun spec_rank<K, V>(m: Map<K, V>, k: K): num;

    // Compared field by field: price first, then the tie breaker.
    struct Key has copy, drop, store {
        price: u64,
        tie: u64,
    }

    fun add_bounds_largest(m: &mut Map<Key, u64>, k: Key) {
        add(m, k, 0);
    }
    spec add_bounds_largest {
        let post n = spec_len(m);
        ensures n == spec_len(old(m)) + 1;
        ensures spec_key_at(m, n - 1).price >= k.price;
        ensures spec_len(old(m)) > 0 ==>
            spec_key_at(m, n - 1).price
                >= spec_key_at(old(m), spec_len(old(m)) - 1).price;
    }

    fun add_bounds_smallest(m: &mut Map<Key, u64>, k: Key) {
        add(m, k, 0);
    }
    spec add_bounds_smallest {
        ensures spec_key_at(m, 0).price <= k.price;
        ensures spec_len(old(m)) > 0 ==> spec_key_at(m, 0).price <= spec_key_at(old(m), 0).price;
    }

    // Non-vacuity canary: the smallest key need not move.
    fun add_smallest_unchanged_wrong(m: &mut Map<Key, u64>, k: Key) {
        add(m, k, 0);
    }
    spec add_smallest_unchanged_wrong {
        requires spec_len(m) > 0;
        ensures spec_key_at(m, 0) == spec_key_at(old(m), 0);
    }

    // A key holding a map has no comparison model, so reading its positions must not
    // ask for one.
    struct MapKey has copy, drop, store {
        a: u64,
        inner: Map<u64, u64>,
    }

    fun add_map_keyed(m: &mut Map<MapKey, u64>, k: MapKey) {
        add(m, k, 0);
    }
    spec add_map_keyed {
        ensures spec_len(m) > 0 ==> spec_contains(m, spec_key_at(m, 0));
    }

    // Brings `cmp` into the program as a real package's map module does, while
    // `Key` itself is still never compared in bytecode.
    fun compare_numbers(a: u64, b: u64): std::cmp::Ordering {
        std::cmp::compare(&a, &b)
    }

    // Non-vacuity canary: the new key need not be the largest.
    fun add_new_key_largest_wrong(m: &mut Map<Key, u64>, k: Key) {
        add(m, k, 0);
    }
    spec add_new_key_largest_wrong {
        ensures spec_key_at(m, spec_len(m) - 1).price == k.price;
    }
}
