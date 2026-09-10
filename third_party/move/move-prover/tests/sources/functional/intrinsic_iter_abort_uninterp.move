// An UNINTERPRETED (bodyless, non-native) spec fun bound to
// `map_spec_aborts_iter_borrow_mut`: the spec translator emits its
// uninterpreted declaration, so the template must not also define it —
// the binding is treated like a bodied one (user-supplied predicate),
// not like a native. `probe` exercises a spec use of the predicate.
module 0x42::intrinsic_iter_abort_uninterp {
    enum Iter<K: copy + drop> has copy, drop {
        Some { key: K },
        End,
    }
    struct Map<phantom K: copy + drop, phantom V> has store, drop {}
    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_iter_borrow_mut = ibm,
            map_spec_get = spec_get,
            map_spec_aborts_iter_borrow_mut = spec_aborts_ibm;
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    spec native fun spec_get<K: copy + drop, V>(m: Map<K, V>, k: K): V;
    native fun ibm<K: copy + drop, V>(it: Iter<K>, m: &mut Map<K, V>): &mut V;
    spec fun spec_aborts_ibm<K: copy + drop, V>(it: Iter<K>, m: Map<K, V>): bool;

    fun touch(_m: &mut Map<u64, u64>, _it: Iter<u64>): bool {
        true
    }
    spec touch {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures result == spec_aborts_ibm(_it, _m);
    }
    fun probe(m: &mut Map<u64, u64>, it: Iter<u64>): bool {
        touch(m, it)
    }
}
