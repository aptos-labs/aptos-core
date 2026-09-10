// A malformed `map_iter_borrow_mut` binding — the iterator parameter is a
// plain struct, not an enum — must produce a diagnostic, not a prover crash.
module 0x42::intrinsic_iter_role_err {
    struct NotEnum<phantom K: copy + drop> has copy, drop { x: u64 }

    struct Map<phantom K: copy + drop, phantom V> has store, drop {}
    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_spec_get = spec_get,
            map_iter_borrow_mut = ibm; // error: iterator param is not an enum
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    native fun ibm<K: copy + drop, V>(it: NotEnum<K>, m: &mut Map<K, V>): &mut V;
    spec native fun spec_get<K: copy + drop, V>(m: Map<K, V>, k: K): V;

    // Use the map so the intrinsic instance is monomorphized and the prelude
    // resolution path runs.
    fun use_map(): Map<u64, u64> {
        new<u64, u64>()
    }
}
