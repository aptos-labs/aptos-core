// A `map_iter_borrow_mut` binding whose iterator enum has more than one type
// parameter must produce a diagnostic, not an out-of-bounds panic when
// monomorphization fabricates the single-parameter instantiation.
module 0x42::intrinsic_iter_role_err2 {
    enum WideIter<K: copy + drop, V> has copy, drop {
        Some { key: K },
        End,
    }

    struct Map<phantom K: copy + drop, phantom V> has store, drop {}
    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_spec_get = spec_get,
            map_iter_borrow_mut = ibm; // error: iterator enum has two type parameters
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    native fun ibm<K: copy + drop, V>(it: WideIter<K, V>, m: &mut Map<K, V>): &mut V;
    spec native fun spec_get<K: copy + drop, V>(m: Map<K, V>, k: K): V;

    fun use_map(): Map<u64, u64> {
        new<u64, u64>()
    }
}
