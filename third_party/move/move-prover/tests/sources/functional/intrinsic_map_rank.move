// The enumeration view of a bound intrinsic map: `map_spec_key_at` gives the
// i-th smallest key and `map_spec_rank` its inverse on contained keys, so a
// spec can name a position rather than only a key. Exercises the bijection on
// a symbolic map, the shift of surviving ranks across a removal, and a
// non-vacuity canary. No `cmp` instantiation exists here, so the ascending
// axiom is not emitted — the facts below are the order-independent part of
// the view.
module 0x42::intrinsic_map_rank {
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

    // A contained key has an in-range rank, and `key_at` inverts it.
    fun rank_in_range(_m: &Map<u64, u64>, _k: u64) {}
    spec rank_in_range {
        requires spec_contains(_m, _k);
        aborts_if false;
        ensures spec_rank(_m, _k) >= 0 && spec_rank(_m, _k) < spec_len(_m);
        ensures spec_key_at(_m, spec_rank(_m, _k)) == _k;
    }

    // The other direction: every position holds a contained key, and rank
    // inverts `key_at` there.
    fun key_at_contained(_m: &Map<u64, u64>, _i: u64) {}
    spec key_at_contained {
        requires _i < spec_len(_m);
        aborts_if false;
        ensures spec_contains(_m, spec_key_at(_m, _i));
        ensures spec_rank(_m, spec_key_at(_m, _i)) == _i;
    }

    // Removal splices one position out: surviving keys keep their relative
    // order, with ranks above the removed one dropping by one.
    fun remove_shift(m: &mut Map<u64, u64>, k: u64): u64 {
        remove(m, k)
    }
    spec remove_shift {
        requires spec_contains(m, k);
        aborts_if false;
        ensures result == spec_get(old(m), k);
        ensures spec_len(m) == spec_len(old(m)) - 1;
        ensures forall i in 0..spec_len(m):
            spec_key_at(m, i) == spec_key_at(old(m), if (i < spec_rank(old(m), k)) i else i + 1);
    }

    // Non-vacuity canary: the view must not prove an off-by-one rank.
    fun rank_wrong(_m: &Map<u64, u64>, _k: u64) {}
    spec rank_wrong {
        requires spec_contains(_m, _k);
        // error: the rank of a contained key can be 0, so this need not hold
        ensures spec_rank(_m, _k) > 0;
    }
}
