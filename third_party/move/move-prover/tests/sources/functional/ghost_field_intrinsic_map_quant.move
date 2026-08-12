// SOURCE-level quantifiers cannot range over an intrinsic map: the model
// builder rejects the range (map-ranged quantifiers exist only internally,
// fabricated by data-invariant instrumentation — see
// ghost_field_intrinsic_map_data_inv). If this front-end restriction is ever
// lifted, the carrier-represented map below immediately exercises the
// backend's carrier-unwrapped membership constraint.
module 0x42::ghost_field_intrinsic_map_quant {
    struct MapG<phantom K: copy + drop, phantom V> has store, drop {}
    spec MapG {
        pragma intrinsic = map,
            map_new = new,
            map_add_no_override = add,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains,
            map_spec_iter_preserved = spec_preserved;
    }
    native fun new<K: copy + drop, V: store>(): MapG<K, V>;
    native fun add<K: copy + drop, V>(m: &mut MapG<K, V>, k: K, v: V);
    spec native fun spec_get<K: copy + drop, V>(m: MapG<K, V>, k: K): V;
    spec native fun spec_set<K: copy + drop, V>(m: MapG<K, V>, k: K, v: V): MapG<K, V>;
    spec native fun spec_len<K: copy + drop, V>(m: MapG<K, V>): num;
    spec native fun spec_contains<K: copy + drop, V>(m: MapG<K, V>, k: K): bool;
    spec native fun spec_preserved<K: copy + drop, V>(m1: MapG<K, V>, m2: MapG<K, V>): bool;

    fun singleton(): MapG<u64, u64> {
        let m = new();
        add(&mut m, 1, 7);
        m
    }
    spec singleton {
        ensures forall k in result: spec_get(result, k) == 7;
        ensures exists k in result: k == 1;
    }
}
