// A `map_spec_aborts_iter_borrow_mut` binding whose signature does not match
// `(iterator_enum<K>, map<K, V>): bool` must be rejected for EVERY binding
// kind: the behavioral `aborts_of` translation calls the predicate with
// exactly the `map_iter_borrow_mut` binding's two parameters, so a bodied
// (or uninterpreted) mis-signed binding — which the template-side check
// never sees — would emit an ill-typed Boogie call.
module 0x42::intrinsic_iter_abort_sig_err {
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
            map_spec_aborts_iter_borrow_mut = spec_aborts_ibm;
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    spec native fun spec_get<K: copy + drop, V>(m: Map<K, V>, k: K): V;
    native fun ibm<K: copy + drop, V>(it: Iter<K>, m: &mut Map<K, V>): &mut V;
    // error: bodied AND mis-signed (missing the map parameter)
    spec fun spec_aborts_ibm<K: copy + drop, V>(it: Iter<K>): bool {
        it is Iter::End<K>
    }

    fun use_map(): Map<u64, u64> {
        new<u64, u64>()
    }
}
