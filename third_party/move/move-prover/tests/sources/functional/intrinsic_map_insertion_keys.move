// use-aptos-stdlib
// flag: --aptos
// An insertion-ordered intrinsic map that binds `map_keys`, in a program that
// compares its key type. `keys` returns the keys in insertion order, so it must
// not be assumed to return them ascending under `cmp::compare`: after inserting
// 2 and then 1 that assumption contradicts the enumeration, and every caller
// becomes vacuous.
module 0x42::intrinsic_map_insertion_keys {
    struct Map<phantom K: copy + drop, phantom V> has store, drop, copy {}

    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_has_key = contains,
            map_add_no_override = add,
            map_keys = keys,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains,
            map_spec_insertion_key_at = spec_key_at,
            map_spec_insertion_rank = spec_rank;
    }

    public native fun new<K: copy + drop, V: store>(): Map<K, V>;
    public native fun contains<K: copy + drop, V>(m: &Map<K, V>, key: K): bool;
    public native fun add<K: copy + drop, V>(m: &mut Map<K, V>, key: K, val: V);
    public native fun keys<K: copy + drop, V>(m: &Map<K, V>): vector<K>;

    spec native fun spec_len<K, V>(m: Map<K, V>): num;
    spec native fun spec_contains<K, V>(m: Map<K, V>, k: K): bool;
    spec native fun spec_get<K, V>(m: Map<K, V>, k: K): V;
    spec native fun spec_set<K, V>(m: Map<K, V>, k: K, v: V): Map<K, V>;
    spec native fun spec_remove<K, V>(m: Map<K, V>, k: K): Map<K, V>;
    spec native fun spec_key_at<K, V>(m: Map<K, V>, i: num): K;
    spec native fun spec_rank<K, V>(m: Map<K, V>, k: K): num;

    // Makes `compare<u64>` available, as any program comparing its keys would.
    fun compare_keys(a: u64, b: u64): std::cmp::Ordering {
        std::cmp::compare(&a, &b)
    }

    // Must fail: the keys come back as [2, 1], so they do not ascend.
    fun keys_in_insertion_order(): vector<u64> {
        let m = new<u64, u64>();
        add(&mut m, 2, 20);
        add(&mut m, 1, 10);
        keys(&m)
    }
    spec keys_in_insertion_order {
        ensures len(result) == 2 ==> result[0] < result[1];
    }
}
