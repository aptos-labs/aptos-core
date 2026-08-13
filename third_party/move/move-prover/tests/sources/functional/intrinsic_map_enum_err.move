// Enum intrinsic maps are legal declarations (the framework's ordered maps
// are single-variant enums), but the intrinsic representation — raw table,
// or ghost carrier — declares no variant constructors, so verified code
// cannot pack or unpack a variant of one; each is a diagnostic instead of
// undefined Boogie. (The map's own runtime implementation is unaffected:
// role-bound functions are modeled by templates and never translated.)
module 0x42::intrinsic_map_enum_err {
    enum Map<phantom K: copy + drop, phantom V> has store, drop {
        Empty,
    }
    spec Map {
        pragma intrinsic = map,
            map_new = new;
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;

    fun mk(): Map<u64, u64> {
        Map::Empty // error: cannot pack an intrinsic map
    }
}
