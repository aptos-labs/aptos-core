// A position-based `map_iter_borrow_mut` binding — one whose iterator carries an
// index rather than a key — reaches the key it borrows through the enumeration,
// so it must produce a diagnostic when `map_spec_key_at` is not bound.
module 0x42::intrinsic_iter_role_err8 {
    enum PosIter has copy, drop {
        Position { index: u64 },
        End,
    }

    struct Map<phantom K: copy + drop, phantom V> has store, drop {}
    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_spec_get = spec_get,
            map_iter_borrow_mut = ibm; // error: position-based, but no map_spec_key_at
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    native fun ibm<K: copy + drop, V>(it: PosIter, m: &mut Map<K, V>): &mut V;
    spec native fun spec_get<K: copy + drop, V>(m: Map<K, V>, k: K): V;

    fun use_map(): Map<u64, u64> {
        new<u64, u64>()
    }
}
