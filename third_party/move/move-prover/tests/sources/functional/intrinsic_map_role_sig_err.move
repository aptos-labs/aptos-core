// Template-defined role bindings have fixed signatures (the template emits
// the definition in that shape; user spec text calls the declared shape),
// and call-only abort predicates are invoked by the behavioral `aborts_of`
// translation with the paired Move function's parameters. Mis-signed or
// wrongly-natured bindings must be model-build errors, not ill-typed Boogie.

// Mis-signed native on a template-defined model-observer role.
module 0x42::intrinsic_map_sig_observer {
    struct Map<phantom K: copy + drop, phantom V> has store, drop {}
    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_spec_len = spec_len;
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    // error: extra parameter (expected (map<K, V>): num)
    spec native fun spec_len<K, V>(t: Map<K, V>, extra: u64): num;
    fun touch(): Map<u64, u64> {
        new<u64, u64>()
    }
}

// Mis-signed bodied binding on a call-only abort role.
module 0x42::intrinsic_map_sig_callonly {
    struct Map<phantom K: copy + drop, phantom V> has store, drop {}
    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_spec_has_key = spec_has_key,
            map_spec_get = spec_get,
            map_borrow_front = borrow_front,
            map_spec_aborts_empty = spec_aborts_empty;
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    spec native fun spec_has_key<K, V>(t: Map<K, V>, k: K): bool;
    spec native fun spec_get<K, V>(t: Map<K, V>, k: K): V;
    native fun borrow_front<K: copy + drop, V>(m: &Map<K, V>): (K, &V);
    // error: extra parameter (expected the map alone, like `borrow_front`)
    spec fun spec_aborts_empty<K, V>(t: Map<K, V>, extra: u64): bool {
        extra == 0
    }
    fun touch(): Map<u64, u64> {
        new<u64, u64>()
    }
}

// Native binding on a call-only abort role: nothing would define it.
module 0x42::intrinsic_map_sig_native_callonly {
    struct Map<phantom K: copy + drop, phantom V> has store, drop {}
    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_spec_has_key = spec_has_key,
            map_spec_get = spec_get,
            map_borrow_front = borrow_front,
            map_spec_aborts_empty = spec_aborts_empty;
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    spec native fun spec_has_key<K, V>(t: Map<K, V>, k: K): bool;
    spec native fun spec_get<K, V>(t: Map<K, V>, k: K): V;
    native fun borrow_front<K: copy + drop, V>(m: &Map<K, V>): (K, &V);
    // error: native — neither the template nor spec translation defines it
    spec native fun spec_aborts_empty<K, V>(t: Map<K, V>): bool;
    fun touch(): Map<u64, u64> {
        new<u64, u64>()
    }
}
