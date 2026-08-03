// Bindings for roles whose definitions the prelude template emits must be
// `spec native fun`s: a bodied spec fun is also emitted by regular
// spec-function translation, so the same Boogie function would be declared
// twice. Covers a model-observer role (`map_spec_len`) and a validity role
// (`map_spec_iter_valid`); `map_spec_aborts_iter_borrow_mut` stays exempt —
// its template definition is gated on the binding being native.
module 0x42::intrinsic_map_native_role_err {
    enum Iter<K: copy + drop> has copy, drop {
        Some { key: K },
        End,
    }

    struct Map<phantom K: copy + drop, phantom V> has store, drop {}
    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_spec_len = spec_len,
            map_spec_iter_valid = spec_iter_valid;
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    // error: bodied binding for a template-defined role
    spec fun spec_len<K, V>(_m: Map<K, V>): num {
        0
    }
    // error: bodied binding for a template-defined role
    spec fun spec_iter_valid<K, V>(_it: Iter<K>, _m: Map<K, V>): bool {
        true
    }

    fun use_map(): Map<u64, u64> {
        new<u64, u64>()
    }
}
