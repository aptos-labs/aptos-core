// A `map_iter_borrow_mut` binding whose iterator parameter is instantiated
// with the wrong type parameter (`Iter<V>` instead of `Iter<K>`) must produce
// a diagnostic: the template hardcodes the map's key type, so the emitted
// call would otherwise be ill-typed Boogie.
module 0x42::intrinsic_iter_role_err3 {
    enum Iter<K: copy + drop> has copy, drop {
        Some { key: K },
        End,
    }

    struct Map<phantom K: copy + drop, phantom V> has store, drop {}
    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_spec_get = spec_get,
            map_iter_borrow_mut = ibm; // error: iterator instantiated with V, not K
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    native fun ibm<K: copy + drop, V: copy + drop>(it: Iter<V>, m: &mut Map<K, V>): &mut V;
    spec native fun spec_get<K: copy + drop, V>(m: Map<K, V>, k: K): V;

    fun use_map(): Map<u64, u64> {
        new<u64, u64>()
    }
}
