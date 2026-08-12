// A `map_iter_borrow_mut` binding with extra parameters (or type parameters)
// must produce a diagnostic: the template has a fixed two-parameter,
// two-type-parameter shape, so wider bindings would emit arity-mismatched
// calls. Also exercises that borrow analysis tolerates the malformed binding
// (it runs before this diagnostic is emitted at mono analysis).
module 0x42::intrinsic_iter_role_err5 {
    enum Iter<K: copy + drop> has copy, drop {
        Some { key: K },
        End,
    }

    struct Map<phantom K: copy + drop, phantom V> has store, drop {}
    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_spec_get = spec_get,
            map_iter_borrow_mut = ibm; // error: extra parameter
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    native fun ibm<K: copy + drop, V, W>(it: Iter<K>, m: &mut Map<K, V>, w: W): &mut V;
    spec native fun spec_get<K: copy + drop, V>(m: Map<K, V>, k: K): V;

    fun use_map(): Map<u64, u64> {
        new<u64, u64>()
    }
}
