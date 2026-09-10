// Native `map_spec_aborts_iter_borrow_mut` bindings must match the template's
// fixed signature; otherwise spec translation would call a symbol the
// template never defines.
module 0x42::intrinsic_iter_role_err7 {
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
            map_spec_aborts_iter_borrow_mut = bad_aborts; // error: extra type parameter
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    native fun ibm<K: copy + drop, V>(it: Iter<K>, m: &mut Map<K, V>): &mut V;
    spec native fun spec_get<K: copy + drop, V>(m: Map<K, V>, k: K): V;
    spec native fun bad_aborts<K: copy + drop, V, W>(it: Iter<K>, m: Map<K, V>, w: W): bool;

    fun use_map(): Map<u64, u64> {
        new<u64, u64>()
    }
}

// The abort predicate's iterator type comes from the `map_iter_borrow_mut`
// binding, which must therefore be co-bound.
module 0x42::intrinsic_iter_role_err7b {
    enum Iter<K: copy + drop> has copy, drop {
        Some { key: K },
        End,
    }

    struct Map<phantom K: copy + drop, phantom V> has store, drop {}
    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_spec_get = spec_get,
            map_spec_aborts_iter_borrow_mut = lone_aborts; // error: no iter_borrow_mut co-binding
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    spec native fun spec_get<K: copy + drop, V>(m: Map<K, V>, k: K): V;
    spec native fun lone_aborts<K: copy + drop, V>(it: Iter<K>, m: Map<K, V>): bool;

    fun use_map(): Map<u64, u64> {
        new<u64, u64>()
    }
}
