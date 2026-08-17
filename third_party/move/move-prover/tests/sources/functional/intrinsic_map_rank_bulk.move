// The enumeration view across the bulk operations. Each of these templates
// havocs a fresh result table, so the view is available on the post-state
// only because the template asserts the table's well-formedness there; these
// specs are what would fail if that were dropped for any one of them. The
// facts proved are the position/key bijection, which is the order-independent
// core (no `cmp` instantiation exists in this module).
module 0x42::intrinsic_map_rank_bulk {
    struct Map<phantom K: copy + drop, phantom V> has store, drop {}

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
            map_spec_rank = spec_rank,
            map_new_from = new_from,
            map_add_all = add_all,
            map_upsert_all = upsert_all,
            map_append = append,
            map_append_disjoint = append_disjoint,
            map_trim = trim,
            map_replace_key_inplace = replace_key_inplace;
    }

    public native fun new<K: copy + drop, V: store>(): Map<K, V>;
    public native fun contains<K: copy + drop, V>(m: &Map<K, V>, key: K): bool;
    public native fun add<K: copy + drop, V>(m: &mut Map<K, V>, key: K, val: V);
    public native fun remove<K: copy + drop, V>(m: &mut Map<K, V>, key: K): V;
    public native fun new_from<K: copy + drop, V: store>(keys: vector<K>, values: vector<V>): Map<K, V>;
    public native fun add_all<K: copy + drop, V>(m: &mut Map<K, V>, keys: vector<K>, values: vector<V>);
    public native fun upsert_all<K: copy + drop, V: drop>(m: &mut Map<K, V>, keys: vector<K>, values: vector<V>);
    public native fun append<K: copy + drop, V>(m: &mut Map<K, V>, other: Map<K, V>);
    public native fun append_disjoint<K: copy + drop, V>(m: &mut Map<K, V>, other: Map<K, V>);
    public native fun trim<K: copy + drop, V>(m: &mut Map<K, V>, at: u64): Map<K, V>;
    public native fun replace_key_inplace<K: copy + drop, V>(m: &mut Map<K, V>, old_key: &K, new_key: K);

    spec native fun spec_len<K, V>(m: Map<K, V>): num;
    spec native fun spec_contains<K, V>(m: Map<K, V>, k: K): bool;
    spec native fun spec_get<K, V>(m: Map<K, V>, k: K): V;
    spec native fun spec_set<K, V>(m: Map<K, V>, k: K, v: V): Map<K, V>;
    spec native fun spec_remove<K, V>(m: Map<K, V>, k: K): Map<K, V>;
    spec native fun spec_key_at<K, V>(m: Map<K, V>, i: num): K;
    spec native fun spec_rank<K, V>(m: Map<K, V>, k: K): num;

    // The bijection, as a reusable shorthand for the post-states below.
    spec fun enumerated(m: Map<u64, u64>): bool {
        forall i in 0..spec_len(m):
            spec_contains(m, spec_key_at(m, i)) && spec_rank(m, spec_key_at(m, i)) == i
    }

    fun build(keys: vector<u64>, values: vector<u64>): Map<u64, u64> {
        new_from(keys, values)
    }
    spec build {
        ensures enumerated(result);
        ensures spec_len(result) == len(keys);
    }

    fun bulk_add(m: &mut Map<u64, u64>, keys: vector<u64>, values: vector<u64>) {
        add_all(m, keys, values);
    }
    spec bulk_add {
        ensures enumerated(m);
    }

    fun bulk_upsert(m: &mut Map<u64, u64>, keys: vector<u64>, values: vector<u64>) {
        upsert_all(m, keys, values);
    }
    spec bulk_upsert {
        ensures enumerated(m);
    }

    fun merge(m: &mut Map<u64, u64>, other: Map<u64, u64>) {
        append(m, other);
    }
    spec merge {
        ensures enumerated(m);
    }

    fun merge_disjoint(m: &mut Map<u64, u64>, other: Map<u64, u64>) {
        append_disjoint(m, other);
    }
    spec merge_disjoint {
        ensures enumerated(m);
    }

    // Both sides of a split are fresh tables, so both need the view.
    fun split(m: &mut Map<u64, u64>, at: u64): Map<u64, u64> {
        trim(m, at)
    }
    spec split {
        ensures enumerated(m);
        ensures enumerated(result);
        ensures spec_len(m) == at;
    }

    fun rename(m: &mut Map<u64, u64>, old_key: u64, new_key: u64) {
        replace_key_inplace(m, &old_key, new_key);
    }
    spec rename {
        ensures enumerated(m);
        ensures spec_len(m) == spec_len(old(m));
    }

    // Non-vacuity canary: the bijection does not pin which key sits first.
    fun build_wrong(keys: vector<u64>, values: vector<u64>): Map<u64, u64> {
        new_from(keys, values)
    }
    spec build_wrong {
        requires len(keys) > 0;
        // error: nothing forces the rank-0 key to be keys[0]
        ensures spec_key_at(result, 0) == keys[0];
    }
}
