// Spec-level pack of an intrinsic map is a diagnostic, mirroring the
// code-level rejection: the intrinsic representation — raw table, or ghost
// carrier — has no field constructors, so the emitted constructor would be
// ill-typed Boogie (wrong arguments for a struct, a nonexistent variant
// constructor for an enum).

// Raw-table representation (no validity roles bound).
module 0x42::spec_pack_raw {
    struct Map<phantom K: copy + drop, phantom V> has store, drop {}
    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_spec_len = spec_len;
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    spec native fun spec_len<K, V>(t: Map<K, V>): num;

    fun mk(): Map<u64, u64> {
        new()
    }
    spec mk {
        ensures spec_len(result) == spec_len(Map<u64, u64> {});
    }
}

// Ghost-carrier representation (validity role bound).
module 0x42::spec_pack_carrier {
    struct Map<phantom K: copy + drop, phantom V> has store, drop {}

    enum IteratorPtr<K: copy + drop> has copy, drop {
        End,
        Some { key: K },
    }

    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_spec_len = spec_len,
            map_spec_iter_valid = spec_iter_valid;
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    spec native fun spec_len<K, V>(t: Map<K, V>): num;
    spec native fun spec_iter_valid<K, V>(it: IteratorPtr<K>, t: Map<K, V>): bool;

    fun mk(): Map<u64, u64> {
        new()
    }
    spec mk {
        ensures spec_len(result) == spec_len(Map<u64, u64> {});
    }
}

// Enum intrinsic map: the carrier declares no variant constructors.
module 0x42::spec_pack_enum {
    enum Map<phantom K: copy + drop, phantom V> has store, drop {
        Empty,
    }

    enum IteratorPtr<K: copy + drop> has copy, drop {
        End,
        Some { key: K },
    }

    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_spec_len = spec_len,
            map_spec_iter_valid = spec_iter_valid;
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    spec native fun spec_len<K, V>(t: Map<K, V>): num;
    spec native fun spec_iter_valid<K, V>(it: IteratorPtr<K>, t: Map<K, V>): bool;

    fun mk(): Map<u64, u64> {
        new()
    }
    spec mk {
        ensures spec_len(result) == spec_len<u64, u64>(Map::Empty);
    }
}
