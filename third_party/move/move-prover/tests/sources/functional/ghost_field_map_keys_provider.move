// A module providing an intrinsic map, and a function keying it by its type
// parameter in its body alone, to the ghost_field_map_keys.move test.
module 0x42::ghost_field_map_keys_provider {
    struct Map<phantom K: copy + drop, phantom V> has copy, store, drop {}

    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_has_key = contains,
            map_add_no_override = add,
            map_add_override_if_exists = set,
            map_len = length,
            map_del_must_exist = remove,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains,
            map_spec_new = spec_new;
    }

    public native fun new<K: copy + drop, V: store>(): Map<K, V>;
    public native fun contains<K: copy + drop, V>(m: &Map<K, V>, key: K): bool;
    public native fun add<K: copy + drop, V>(m: &mut Map<K, V>, key: K, val: V);
    public native fun set<K: copy + drop, V>(m: &mut Map<K, V>, k: K, v: V);
    public native fun length<K: copy + drop, V>(m: &Map<K, V>): u64;
    public native fun remove<K: copy + drop, V>(m: &mut Map<K, V>, key: K): V;

    spec native fun spec_len<K, V>(m: Map<K, V>): num;
    spec native fun spec_contains<K, V>(m: Map<K, V>, k: K): bool;
    spec native fun spec_get<K, V>(m: Map<K, V>, k: K): V;
    spec native fun spec_set<K, V>(m: Map<K, V>, k: K, v: V): Map<K, V>;
    spec native fun spec_remove<K, V>(m: Map<K, V>, k: K): Map<K, V>;
    spec native fun spec_new<K, V>(): Map<K, V>;

    /// Keys a map by its type parameter, which its signature does not show.
    public fun count<T: copy + drop>(x: T): u64 {
        let m = new<T, u64>();
        add(&mut m, x, 1);
        length(&m)
    }
    spec count {
        ensures result == 1;
    }

    /// A struct and an enum with a ghost field, for keys of an importer.
    struct PG has copy, drop, store { v: u64 }
    spec PG {
        ghost g: u64;
    }

    enum PE has copy, drop, store {
        A { v: u64 },
        B,
    }
    spec PE {
        ghost g: u64;
    }

    spec fun keyed<T>(x: T): bool {
        spec_contains(spec_new<T, u64>(), x)
    }

    /// A key demand on `T` only in its data invariant.
    struct Holder<T: copy + drop + store> has copy, drop, store { x: T }
    spec Holder {
        invariant !keyed(x);
    }
}
