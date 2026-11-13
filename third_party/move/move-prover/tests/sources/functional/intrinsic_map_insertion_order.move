// An insertion-ordered intrinsic map, bound through `map_spec_insertion_key_at`
// and `map_spec_insertion_rank` instead of the key-ordered pair. Two things
// follow from that choice and are checked here: equality compares positions, so
// two maps holding the same entries in different orders are not equal; and the
// enumeration is not assumed to ascend under `cmp::compare`, which would be a
// false axiom for this map and would make every caller vacuous.
//
// The equality case is the regression guard for the defect where the prover
// proved `m1 == m2` for two `SimpleMap`s built by inserting the same pair of
// entries in opposite orders, while `==` returns false at runtime.
module 0x42::intrinsic_map_insertion_order {
    // `copy` matches the real insertion-ordered map this models: it is what makes
    // `==` available to code, and therefore what makes the order observable.
    struct Map<phantom K: copy + drop, phantom V> has store, drop, copy {}

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
            map_spec_insertion_key_at = spec_key_at,
            map_spec_insertion_rank = spec_rank;
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

    // THE REGRESSION GUARD. Same entries, opposite insertion orders. Equality
    // must not be provable: with the position conjunct dropped from `$IsEqual`
    // this verifies, which is the defect.
    fun same_entries_not_equal(): bool {
        let m1 = new<u64, u64>();
        add(&mut m1, 1, 10);
        add(&mut m1, 2, 20);
        let m2 = new<u64, u64>();
        add(&mut m2, 2, 20);
        add(&mut m2, 1, 10);
        m1 == m2
    }
    spec same_entries_not_equal {
        ensures result;
    }

    // Positions still agree with themselves, so a map equals itself. Guards the
    // opposite failure: a conjunct strong enough to make equality unusable.
    fun self_equal(m: &Map<u64, u64>): bool {
        *m == *m
    }
    spec self_equal {
        aborts_if false;
        ensures result;
    }

    // Content facts are unaffected by the position conjunct.
    fun content_facts(_m: &Map<u64, u64>, _k: u64) {}
    spec content_facts {
        requires spec_contains(_m, _k);
        aborts_if false;
        ensures spec_rank(_m, _k) >= 0 && spec_rank(_m, _k) < spec_len(_m);
        ensures spec_key_at(_m, spec_rank(_m, _k)) == _k;
    }

    // Non-vacuity canary. If the suppressed ascending axiom were emitted for
    // this map it would be false, the state would be inconsistent, and this
    // would verify.
    fun canary(_m: &Map<u64, u64>) {}
    spec canary {
        ensures false;
    }
}
