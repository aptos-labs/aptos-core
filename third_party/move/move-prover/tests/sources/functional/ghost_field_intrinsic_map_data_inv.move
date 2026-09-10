// A deep data invariant over the values of a carrier-represented intrinsic
// map (hidden validity slot via the preserved binding): data-invariant
// instrumentation quantifies THROUGH the map (the fabricated quantifier's
// range is the map value itself, a path source syntax cannot express), so
// the emitted membership constraint must read through the carrier's
// raw-table selector. `good` verifies; `bad` violates the value invariant
// through the modeled borrow and must be reported.
module 0x42::ghost_field_intrinsic_map_data_inv {
    struct W has store, drop { v: u64 }
    spec W {
        invariant v != 0;
    }

    struct MapG<phantom K: copy + drop, phantom V> has store, drop {}
    spec MapG {
        pragma intrinsic = map,
            map_new = new,
            map_add_no_override = add,
            map_borrow_mut = borrow_mut,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains,
            map_spec_iter_preserved = spec_preserved;
    }
    native fun new<K: copy + drop, V: store>(): MapG<K, V>;
    native fun add<K: copy + drop, V>(m: &mut MapG<K, V>, k: K, v: V);
    native fun borrow_mut<K: copy + drop, V>(m: &mut MapG<K, V>, k: K): &mut V;
    spec native fun spec_get<K: copy + drop, V>(m: MapG<K, V>, k: K): V;
    spec native fun spec_set<K: copy + drop, V>(m: MapG<K, V>, k: K, v: V): MapG<K, V>;
    spec native fun spec_len<K: copy + drop, V>(m: MapG<K, V>): num;
    spec native fun spec_contains<K: copy + drop, V>(m: MapG<K, V>, k: K): bool;
    spec native fun spec_preserved<K: copy + drop, V>(m1: MapG<K, V>, m2: MapG<K, V>): bool;

    fun good(m: &mut MapG<u64, W>, k: u64) {
        let w = borrow_mut(m, k);
        w.v = 5;
    }

    fun bad(m: &mut MapG<u64, W>, k: u64) {
        let w = borrow_mut(m, k);
        w.v = 0;
    }
}
