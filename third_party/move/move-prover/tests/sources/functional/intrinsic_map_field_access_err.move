// Declared fields of an intrinsic map are erased by the intrinsic
// representation, so verified code cannot pack the map or select its fields
// — the emission would be ill-typed against the raw table or its carrier.
// Each is a diagnostic instead. (The map's own runtime implementation is
// unaffected: role-bound functions are modeled by templates and never
// translated.)
module 0x42::intrinsic_map_field_access_err {
    struct MapD<phantom K: copy + drop, phantom V> has store, drop { dummy: bool }
    spec MapD {
        pragma intrinsic = map;
    }

    fun make(): MapD<u64, u64> {
        MapD { dummy: true } // error: cannot pack an intrinsic map
    }

    fun read(m: &MapD<u64, u64>): bool {
        m.dummy // error: cannot select a field of an intrinsic map
    }
}
