// A `map_iter_borrow_mut` binding taking its iterator by reference must
// produce a diagnostic: the template's iterator parameter is by value, so a
// `&mut Iter<K>` binding would pass a `$Mutation` into a value slot.
module 0x42::intrinsic_iter_role_err6 {
    enum Iter<K: copy + drop> has copy, drop {
        Some { key: K },
        End,
    }

    struct Map<phantom K: copy + drop, phantom V> has store, drop {}
    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_spec_get = spec_get,
            map_iter_borrow_mut = ibm; // error: iterator taken by reference
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    native fun ibm<K: copy + drop, V>(it: &mut Iter<K>, m: &mut Map<K, V>): &mut V;
    spec native fun spec_get<K: copy + drop, V>(m: Map<K, V>, k: K): V;

    fun use_map(): Map<u64, u64> {
        new<u64, u64>()
    }
}
