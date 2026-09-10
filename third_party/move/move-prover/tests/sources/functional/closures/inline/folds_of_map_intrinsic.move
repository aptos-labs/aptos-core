// flag: --split-vcs-by-assert
// Intrinsic-map mutators inside a `folds_of` lambda: the exact WP over the
// map type's declared spec functions routes the map capture's post value
// into the fold transformer (`spec_set` / `spec_del`), with the declared
// abort conditions feeding the prefix no-abort invariant.
module 0x42::folds_of_map_intrinsic {
    use std::vector;

    struct Map<phantom K: copy + drop, phantom V> has copy, store, drop {}
    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_len = length,
            map_has_key = contains,
            map_add_no_override = add,
            map_add_override_if_exists = set,
            map_del_must_exist = remove,
            map_del_return_key = remove_return_key,
            map_spec_new = spec_new,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains,
            map_spec_aborts_add = spec_aborts_add,
            map_spec_aborts_del = spec_aborts_del;
    }

    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    native fun length<K: copy + drop, V>(m: &Map<K, V>): u64;
    native fun contains<K: copy + drop, V>(m: &Map<K, V>, k: K): bool;
    native fun add<K: copy + drop, V>(m: &mut Map<K, V>, k: K, v: V);
    native fun set<K: copy + drop, V>(m: &mut Map<K, V>, k: K, v: V);
    native fun remove<K: copy + drop, V>(m: &mut Map<K, V>, k: K): V;
    native fun remove_return_key<K: copy + drop, V>(m: &mut Map<K, V>, k: K): (K, V);

    spec native fun spec_new<K: copy + drop, V>(): Map<K, V>;
    spec native fun spec_get<K: copy + drop, V>(m: Map<K, V>, k: K): V;
    spec native fun spec_set<K: copy + drop, V>(m: Map<K, V>, k: K, v: V): Map<K, V>;
    spec native fun spec_remove<K: copy + drop, V>(m: Map<K, V>, k: K): Map<K, V>;
    spec native fun spec_len<K: copy + drop, V>(m: Map<K, V>): num;
    spec native fun spec_contains<K: copy + drop, V>(m: Map<K, V>, k: K): bool;
    spec native fun spec_aborts_add<K: copy + drop, V>(m: Map<K, V>, k: K, v: V): bool;
    spec native fun spec_aborts_del<K: copy + drop, V>(m: Map<K, V>, k: K): bool;

    inline fun each_ref<T>(v: &vector<T>, f: |&T|) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant n == len(v);
            invariant folds_of<f>(v, i);
        };
    }

    spec fun spec_fold<T, Acc>(f: |Acc, &T| Acc, v: vector<T>, init: Acc, end: u64): Acc {
        if (end == 0) init
        else result_of<f>(spec_fold(f, v, init, end - 1), v[end - 1])
    }

    /// Building a map from a vector via the overriding add
    /// (`map_add_override_if_exists`, never aborts): the fold over
    /// `spec_set` is the exact map value.
    fun collect(v: &vector<u64>): Map<u64, u64> {
        let m = new<u64, u64>();
        each_ref(v, |e| set(&mut m, *e, 1));
        m
    }
    spec collect {
        aborts_if false;
        ensures result == spec_fold<u64, Map<u64, u64>>(
            |acc, e| spec_set(acc, e, 1u64), v, spec_new(), len(v));
    }

    /// The non-overriding add (`map_add_no_override`): same fold, with the
    /// declared abort condition feeding the prefix invariant.
    fun collect_distinct(v: &vector<u64>): Map<u64, u64> {
        let m = new<u64, u64>();
        each_ref(v, |e| add(&mut m, *e, 0));
        m
    }
    spec collect_distinct {
        pragma aborts_if_is_partial;
        ensures result == spec_fold<u64, Map<u64, u64>>(
            |acc, e| spec_set(acc, e, 0u64), v, spec_new(), len(v));
    }

    /// Removal with discarded results (`map_del_return_key`): the fold
    /// over `spec_remove`.
    fun remove_all(v: &vector<u64>, m: Map<u64, u64>): Map<u64, u64> {
        let mm = m;
        each_ref(v, |e| {
            let (_k, _val) = remove_return_key(&mut mm, *e);
        });
        mm
    }
    spec remove_all {
        pragma aborts_if_is_partial;
        ensures result == spec_fold<u64, Map<u64, u64>>(
            |acc, e| spec_remove(acc, e), v, m, len(v));
    }

    /// Removal via `map_del_must_exist`: same fold.
    fun remove_all_must_exist(v: &vector<u64>, m: Map<u64, u64>): Map<u64, u64> {
        let mm = m;
        each_ref(v, |e| {
            let _val = remove(&mut mm, *e);
        });
        mm
    }
    spec remove_all_must_exist {
        pragma aborts_if_is_partial;
        ensures result == spec_fold<u64, Map<u64, u64>>(
            |acc, e| spec_remove(acc, e), v, m, len(v));
    }

    /// Non-vacuity canary: a wrong fold value must fail.
    fun collect_wrong(v: &vector<u64>): Map<u64, u64> {
        let m = new<u64, u64>();
        each_ref(v, |e| set(&mut m, *e, 1));
        m
    }
    spec collect_wrong {
        pragma aborts_if_is_partial;
        ensures result == spec_fold<u64, Map<u64, u64>>(
            |acc, e| spec_set(acc, e, 2u64), v, spec_new(), len(v)); // error: wrong inserted value
    }
}
