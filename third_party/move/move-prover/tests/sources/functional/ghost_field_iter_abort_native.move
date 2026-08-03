// A carrier-represented intrinsic map (hidden validity slot via the
// preserved binding) binding `map_spec_aborts_iter_borrow_mut` to a NATIVE
// spec fun: the template-emitted predicate receives the carrier and must
// read membership through the raw-table selector. The aborts_if below
// mirrors the `iter_borrow_mut` procedure's abort behavior exactly, so
// `poke` verifies.
module 0x42::ghost_field_iter_abort_native {
    enum Iter<K: copy + drop> has copy, drop {
        Some { key: K },
        End,
    }

    struct Map<phantom K: copy + drop, phantom V> has store, drop {}
    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_spec_get = spec_get,
            map_iter_borrow_mut = ibm,
            map_spec_aborts_iter_borrow_mut = spec_aborts_ibm,
            map_spec_iter_preserved = spec_preserved;
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    native fun ibm<K: copy + drop, V>(it: Iter<K>, m: &mut Map<K, V>): &mut V;
    spec native fun spec_aborts_ibm<K: copy + drop, V>(it: Iter<K>, m: Map<K, V>): bool;
    spec native fun spec_get<K: copy + drop, V>(m: Map<K, V>, k: K): V;
    spec native fun spec_preserved<K: copy + drop, V>(m1: Map<K, V>, m2: Map<K, V>): bool;

    fun poke(m: &mut Map<u64, u64>, it: Iter<u64>): u64 {
        *ibm(it, m)
    }
    spec poke {
        aborts_if spec_aborts_ibm(it, m);
    }
}
