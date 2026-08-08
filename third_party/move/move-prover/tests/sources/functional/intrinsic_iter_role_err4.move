// A `map_iter_borrow_mut` binding that returns `&mut K` instead of `&mut V`
// must produce a diagnostic: the template writes the returned reference back
// to the abstract table at the iterator key, so with `K == V` at some
// instantiation the mismatch would type-check in Boogie while proving
// table-value updates the runtime applies to unrelated state.
module 0x42::intrinsic_iter_role_err4 {
    enum Iter<K: copy + drop> has copy, drop {
        Some { key: K },
        End,
    }

    struct Map<phantom K: copy + drop, phantom V> has store, drop {}
    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_spec_get = spec_get,
            map_iter_borrow_mut = ibm; // error: returns `&mut K`, not `&mut V`
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    native fun ibm<K: copy + drop, V>(it: Iter<K>, m: &mut Map<K, V>): &mut K;
    spec native fun spec_get<K: copy + drop, V>(m: Map<K, V>, k: K): V;

    fun use_map(): Map<u64, u64> {
        new<u64, u64>()
    }
}
